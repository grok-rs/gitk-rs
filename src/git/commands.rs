use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::env;
use tracing::{debug, warn, error};
use crate::git::security::{SecurityValidator, RateLimiter};
use crate::git::platform_security::PlatformSecurity;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Safe Git command execution wrapper
/// Provides security measures similar to the original gitk's safe_exec functionality
pub struct GitCommandRunner {
    repo_path: PathBuf,
    git_executable: PathBuf,
    environment: HashMap<String, String>,
    security_validator: SecurityValidator,
    rate_limiter: std::sync::Mutex<RateLimiter>,
}

impl GitCommandRunner {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        let git_executable = Self::find_git_executable()?;
        let environment = Self::create_safe_environment();
        let security_validator = SecurityValidator::new()?;
        let rate_limiter = std::sync::Mutex::new(RateLimiter::new(10, 5)); // 10 commands/sec, max 5 concurrent
        
        Ok(Self {
            repo_path,
            git_executable,
            environment,
            security_validator,
            rate_limiter,
        })
    }
    
    /// Find the Git executable in a secure manner
    fn find_git_executable() -> Result<PathBuf> {
        // Use platform-specific secure Git finding
        match PlatformSecurity::find_git_executable_secure() {
            Ok(git_path) => {
                debug!("Found Git executable at: {:?}", git_path);
                return Ok(git_path);
            }
            Err(e) => {
                warn!("Platform security Git lookup failed: {}", e);
            }
        }
        
        // Fallback: try git2 approach
        if let Ok(git_path) = git2::Repository::discover(".") {
            if let Some(config) = git_path.config().ok() {
                if let Ok(git_bin) = config.get_str("core.gitproxy") {
                    let git_path_buf = PathBuf::from(git_bin);
                    if git_path_buf.exists() {
                        return Ok(git_path_buf);
                    }
                }
            }
        }
        
        // Last resort: legacy PATH lookup
        #[cfg(windows)]
        {
            Self::find_executable_windows("git.exe")
        }
        #[cfg(not(windows))]
        {
            Self::find_executable_unix("git")
        }
    }
    
    #[cfg(windows)]
    fn find_executable_windows(name: &str) -> Result<PathBuf> {
        // Implement Windows-specific secure PATH lookup
        let path_env = env::var("PATH").unwrap_or_default();
        let paths: Vec<&str> = path_env.split(';').collect();
        
        // Filter out potentially dangerous paths
        let safe_paths: Vec<&str> = paths
            .into_iter()
            .filter(|p| !p.is_empty() && Self::is_safe_path_windows(p))
            .collect();
        
        for path in safe_paths {
            let full_path = Path::new(path).join(name);
            if full_path.exists() && full_path.is_file() {
                return Ok(full_path);
            }
        }
        
        Err(anyhow!("Git executable not found in safe PATH"))
    }
    
    #[cfg(not(windows))]
    fn find_executable_unix(name: &str) -> Result<PathBuf> {
        let path_env = env::var("PATH").unwrap_or_default();
        let paths: Vec<&str> = path_env.split(':').collect();
        
        for path in paths {
            if path.is_empty() {
                continue;
            }
            
            let full_path = Path::new(path).join(name);
            if full_path.exists() && full_path.is_file() {
                // Check if executable
                if let Ok(metadata) = full_path.metadata() {
                    if metadata.permissions().mode() & 0o111 != 0 {
                        return Ok(full_path);
                    }
                }
            }
        }
        
        Err(anyhow!("Git executable not found in PATH"))
    }
    
    #[cfg(windows)]
    fn is_safe_path_windows(path: &str) -> bool {
        // Avoid paths that could be dangerous on Windows
        let dangerous_patterns = [
            "temp", "tmp", "windows\\temp", "appdata\\local\\temp",
            "programdata", "users\\public", "system32", "syswow64",
            "windows\\system32", "windows\\syswow64", "$recycle.bin",
            "recovery", "documents and settings"
        ];
        
        let path_lower = path.to_lowercase();
        
        // Block dangerous patterns
        if dangerous_patterns.iter().any(|pattern| path_lower.contains(pattern)) {
            return false;
        }
        
        // Block UNC paths
        if path.starts_with("\\\\") {
            return false;
        }
        
        // Block paths with suspicious characters
        if path.contains("..") || path.contains("~") {
            return false;
        }
        
        // Require absolute paths on Windows for better security
        if path.len() < 3 || !path.chars().nth(1).map_or(false, |c| c == ':') {
            return false;
        }
        
        // Check for valid drive letters
        let first_char = path.chars().next().unwrap_or('\0');
        if !first_char.is_ascii_alphabetic() {
            return false;
        }
        
        true
    }
    
    /// Create a safe environment for Git commands
    fn create_safe_environment() -> HashMap<String, String> {
        // Start with platform-specific secure environment
        let mut env = PlatformSecurity::get_secure_environment();
        
        // Add secure PATH
        if let Ok(secure_path) = PlatformSecurity::secure_path() {
            env.insert("PATH".to_string(), secure_path);
        }
        
        // Git-specific environment for security
        env.insert("GIT_PAGER".to_string(), "cat".to_string());
        env.insert("GIT_EDITOR".to_string(), "true".to_string());
        env.insert("GIT_ASKPASS".to_string(), "true".to_string()); // Disable password prompts
        env.insert("GIT_SSH_COMMAND".to_string(), "false".to_string()); // Disable SSH
        env.insert("GIT_PROXY_COMMAND".to_string(), "false".to_string()); // Disable proxy commands
        env.insert("GIT_CONFIG_NOSYSTEM".to_string(), "1".to_string()); // Ignore system config
        env.insert("GIT_CEILING_DIRECTORIES".to_string(), "/".to_string()); // Limit repository discovery
        env.insert("GIT_CONFIG_GLOBAL".to_string(), "false".to_string()); // Disable global config
        env.insert("GIT_TERMINAL_PROMPT".to_string(), "0".to_string()); // Disable terminal prompts
        
        // Disable Git hooks for security
        env.insert("GIT_OPTIONAL_LOCKS".to_string(), "0".to_string());
        
        env
    }
    
    /// Execute a Git command safely
    pub fn run_command(&self, args: &[&str]) -> Result<String> {
        self.run_command_with_options(args, None, true)
    }
    
    /// Execute a Git command with custom options
    pub fn run_command_with_options(
        &self,
        args: &[&str],
        stdin: Option<&str>,
        capture_stderr: bool,
    ) -> Result<String> {
        debug!("Running git command: {:?}", args);
        
        // Check rate limiting
        {
            let mut rate_limiter = self.rate_limiter.lock()
                .map_err(|_| anyhow!("Failed to acquire rate limiter lock"))?;
            rate_limiter.check_rate_limit()?;
        }
        
        // Enhanced security validation
        if !args.is_empty() {
            self.security_validator.validate_command(args[0])?;
        }
        self.security_validator.validate_arguments(args)?;
        self.security_validator.validate_environment(&self.environment)?;
        
        // Legacy validation for backward compatibility
        for arg in args {
            self.validate_argument(arg)?;
        }
        
        let mut cmd = Command::new(&self.git_executable);
        cmd.current_dir(&self.repo_path);
        cmd.args(args);
        
        // Set safe environment
        cmd.env_clear();
        for (key, value) in &self.environment {
            cmd.env(key, value);
        }
        
        // Configure stdio
        if stdin.is_some() {
            cmd.stdin(Stdio::piped());
        }
        cmd.stdout(Stdio::piped());
        if capture_stderr {
            cmd.stderr(Stdio::piped());
        } else {
            cmd.stderr(Stdio::null());
        }
        
        // Start command tracking for rate limiting
        self.rate_limiter.lock().unwrap().start_command();
        
        let result = (|| -> Result<String> {
            let mut child = cmd.spawn()
                .map_err(|e| anyhow!("Failed to spawn git command: {}", e))?;
            
            // Send stdin if provided
            if let Some(input) = stdin {
                use std::io::Write;
                if let Some(mut stdin_handle) = child.stdin.take() {
                    stdin_handle.write_all(input.as_bytes())
                        .map_err(|e| anyhow!("Failed to write to git stdin: {}", e))?;
                }
            }
            
            let output = child.wait_with_output()
                .map_err(|e| anyhow!("Failed to read git command output: {}", e))?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("Git command failed: {}", stderr);
                return Err(anyhow!("Git command failed with status {}: {}", 
                                  output.status, stderr));
            }
            
            let stdout = String::from_utf8(output.stdout)
                .map_err(|e| anyhow!("Git command output is not valid UTF-8: {}", e))?;
            
            Ok(stdout)
        })();
        
        // End command tracking
        self.rate_limiter.lock().unwrap().end_command();
        
        result
    }
    
    /// Validate command arguments for security
    fn validate_argument(&self, arg: &str) -> Result<()> {
        // Check for potentially dangerous characters
        let dangerous_chars = ['|', '&', ';', '`', '$', '<', '>', '\n', '\r'];
        for ch in dangerous_chars {
            if arg.contains(ch) {
                return Err(anyhow!("Argument contains dangerous character: {}", ch));
            }
        }
        
        // Check for potentially dangerous patterns
        let dangerous_patterns = ["--upload-pack", "--receive-pack", "--exec"];
        for pattern in dangerous_patterns {
            if arg.starts_with(pattern) {
                return Err(anyhow!("Argument contains dangerous pattern: {}", pattern));
            }
        }
        
        Ok(())
    }
    
    /// Get the repository path
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }
    
    /// Get the git executable path
    pub fn git_executable(&self) -> &Path {
        &self.git_executable
    }
}

/// High-level Git operations using safe command execution
pub struct GitCommands {
    runner: GitCommandRunner,
}

impl GitCommands {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Result<Self> {
        let runner = GitCommandRunner::new(repo_path)?;
        Ok(Self { runner })
    }
    
    /// Get git rev-list output
    pub fn rev_list(&self, args: &[&str]) -> Result<String> {
        let mut full_args = vec!["rev-list"];
        full_args.extend_from_slice(args);
        self.runner.run_command(&full_args)
    }
    
    /// Get git log output  
    pub fn log(&self, args: &[&str]) -> Result<String> {
        let mut full_args = vec!["log", "--no-color"];
        full_args.extend_from_slice(args);
        self.runner.run_command(&full_args)
    }
    
    /// Get git show-ref output
    pub fn show_ref(&self, args: &[&str]) -> Result<String> {
        let mut full_args = vec!["show-ref"];
        full_args.extend_from_slice(args);
        self.runner.run_command(&full_args)
    }
    
    /// Get git cat-file output
    pub fn cat_file(&self, args: &[&str]) -> Result<String> {
        let mut full_args = vec!["cat-file"];
        full_args.extend_from_slice(args);
        self.runner.run_command(&full_args)
    }
    
    /// Get git diff output
    pub fn diff(&self, args: &[&str]) -> Result<String> {
        let mut full_args = vec!["diff", "--no-color"];
        full_args.extend_from_slice(args);
        self.runner.run_command(&full_args)
    }
    
    /// Get git ls-files output
    pub fn ls_files(&self, args: &[&str]) -> Result<String> {
        let mut full_args = vec!["ls-files"];
        full_args.extend_from_slice(args);
        self.runner.run_command(&full_args)
    }
    
    /// Get git rev-parse output
    pub fn rev_parse(&self, args: &[&str]) -> Result<String> {
        let mut full_args = vec!["rev-parse"];
        full_args.extend_from_slice(args);
        self.runner.run_command(&full_args)
    }
    
    /// Get git for-each-ref output
    pub fn for_each_ref(&self, args: &[&str]) -> Result<String> {
        let mut full_args = vec!["for-each-ref"];
        full_args.extend_from_slice(args);
        self.runner.run_command(&full_args)
    }
    
    /// Check if repository has a working tree
    pub fn has_work_tree(&self) -> Result<bool> {
        match self.rev_parse(&["--is-inside-work-tree"]) {
            Ok(output) => Ok(output.trim() == "true"),
            Err(_) => Ok(false),
        }
    }
    
    /// Get the git directory path
    pub fn git_dir(&self) -> Result<PathBuf> {
        let output = self.rev_parse(&["--git-dir"])?;
        Ok(PathBuf::from(output.trim()))
    }
    
    /// Get the working tree path
    pub fn work_tree(&self) -> Result<Option<PathBuf>> {
        match self.rev_parse(&["--show-toplevel"]) {
            Ok(output) => Ok(Some(PathBuf::from(output.trim()))),
            Err(_) => Ok(None),
        }
    }
}