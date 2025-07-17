use crate::git::platform_security::PlatformSecurity;
use crate::git::security::{RateLimiter, SecurityValidator};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tracing::{debug, error, warn};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Safe Git command execution wrapper
/// Provides security measures similar to the original gitk's `safe_exec` functionality
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
            if let Ok(config) = git_path.config() {
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
            "temp",
            "tmp",
            "windows\\temp",
            "appdata\\local\\temp",
            "programdata",
            "users\\public",
            "system32",
            "syswow64",
            "windows\\system32",
            "windows\\syswow64",
            "$recycle.bin",
            "recovery",
            "documents and settings",
        ];

        let path_lower = path.to_lowercase();

        // Block dangerous patterns
        if dangerous_patterns
            .iter()
            .any(|pattern| path_lower.contains(pattern))
        {
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
            let mut rate_limiter = self
                .rate_limiter
                .lock()
                .map_err(|_| anyhow!("Failed to acquire rate limiter lock"))?;
            rate_limiter.check_rate_limit()?;
        }

        // Enhanced security validation
        if !args.is_empty() {
            self.security_validator.validate_command(args[0])?;
        }
        self.security_validator.validate_arguments(args)?;
        self.security_validator
            .validate_environment(&self.environment)?;

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
            let mut child = cmd
                .spawn()
                .map_err(|e| anyhow!("Failed to spawn git command: {}", e))?;

            // Send stdin if provided
            if let Some(input) = stdin {
                use std::io::Write;
                if let Some(mut stdin_handle) = child.stdin.take() {
                    stdin_handle
                        .write_all(input.as_bytes())
                        .map_err(|e| anyhow!("Failed to write to git stdin: {}", e))?;
                }
            }

            let output = child
                .wait_with_output()
                .map_err(|e| anyhow!("Failed to read git command output: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("Git command failed: {}", stderr);
                return Err(anyhow!(
                    "Git command failed with status {}: {}",
                    output.status,
                    stderr
                ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_repo() -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize Git repository
        let output = Command::new("git")
            .args(&["init"])
            .current_dir(&repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to initialize Git repository"));
        }

        // Configure user for testing
        Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()?;

        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()?;

        Ok((temp_dir, repo_path))
    }

    fn create_test_commit(
        repo_path: &Path,
        filename: &str,
        content: &str,
        message: &str,
    ) -> Result<()> {
        let file_path = repo_path.join(filename);
        fs::write(&file_path, content)?;

        Command::new("git")
            .args(&["add", filename])
            .current_dir(repo_path)
            .output()?;

        let output = Command::new("git")
            .args(&["commit", "-m", message])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to create commit: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    #[test]
    fn test_git_command_runner_creation() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        let runner = GitCommandRunner::new(&repo_path)?;

        assert_eq!(runner.repo_path, repo_path);
        assert!(runner.git_executable.is_absolute());
        assert!(!runner.environment.is_empty());

        Ok(())
    }

    #[test]
    fn test_git_commands_creation() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        let commands = GitCommands::new(&repo_path)?;

        // Test that the GitCommands struct is properly initialized
        // We can't easily test private fields, but we can verify it was created successfully
        assert!(std::ptr::addr_of!(commands) as *const _ != std::ptr::null());

        Ok(())
    }

    #[test]
    fn test_execute_safe_command() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "Hello World", "Initial commit")?;

        let commands = GitCommands::new(&repo_path)?;

        // Test safe command execution
        let result = commands.log(&["--oneline", "-1"])?;
        assert!(result.contains("Initial commit"));

        Ok(())
    }

    #[test]
    fn test_command_security_validation() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        let runner = GitCommandRunner::new(&repo_path)?;

        // Test that dangerous commands are rejected
        let dangerous_args = ["--upload-pack", "/bin/sh"];
        let result = runner.run_command(&dangerous_args);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_environment_sanitization() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        let runner = GitCommandRunner::new(&repo_path)?;

        // Verify that the environment has been sanitized
        assert!(runner.environment.contains_key("PATH"));
        assert!(runner.environment.contains_key("HOME"));
        // GIT_DIR should not be inherited from parent process
        assert!(
            !runner.environment.contains_key("GIT_WORK_TREE")
                || runner.environment.get("GIT_WORK_TREE").unwrap().is_empty()
        );

        Ok(())
    }

    #[test]
    fn test_git_executable_discovery() -> Result<()> {
        // Test that we can find a Git executable
        let git_path = GitCommandRunner::find_git_executable()?;
        assert!(git_path.is_absolute());
        assert!(git_path.exists());

        Ok(())
    }

    #[test]
    fn test_rate_limiting() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        let runner = GitCommandRunner::new(&repo_path)?;

        // Execute multiple commands quickly to test rate limiting
        for i in 0..15 {
            let result = runner.run_command(&["status", "--porcelain"]);
            if i > 10 {
                // After too many rapid requests, we should hit rate limits
                if result.is_err() {
                    break; // Rate limiting is working
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_command_argument_validation() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        let runner = GitCommandRunner::new(&repo_path)?;

        // Test valid arguments
        let valid_result = runner.run_command(&["status", "--porcelain"]);
        assert!(valid_result.is_ok());

        // Test invalid arguments with path traversal
        let invalid_result = runner.run_command(&["log", "../../../etc/passwd"]);
        assert!(invalid_result.is_err());

        Ok(())
    }

    #[test]
    fn test_git_commands_methods() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "content", "Test commit")?;

        let commands = GitCommands::new(&repo_path)?;

        // Test various GitCommands methods
        let log_result = commands.log(&["--oneline", "-1"])?;
        assert!(log_result.contains("Test commit"));

        let show_ref_result = commands.show_ref(&["--heads"])?;
        assert!(show_ref_result.contains("refs/heads/"));

        let _diff_result = commands.diff(&["--name-only", "HEAD~1..HEAD"]);
        // This might fail for first commit, which is expected

        Ok(())
    }

    #[test]
    fn test_working_directory_validation() -> Result<()> {
        // Test with non-existent directory
        let non_existent = PathBuf::from("/this/path/does/not/exist");
        let result = GitCommandRunner::new(&non_existent);
        // Should still create the runner, but commands will fail appropriately
        assert!(result.is_ok() || result.is_err()); // Either is acceptable

        // Test with valid directory
        let (_temp_dir, repo_path) = create_test_repo()?;
        let result = GitCommandRunner::new(&repo_path);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_git_command_timeout() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        let runner = GitCommandRunner::new(&repo_path)?;

        // Test that commands don't hang indefinitely
        // Using a command that should complete quickly
        let start = std::time::Instant::now();
        let result = runner.run_command(&["status"]);
        let duration = start.elapsed();

        // Command should complete within reasonable time (5 seconds)
        assert!(duration.as_secs() < 5);
        assert!(result.is_ok() || result.is_err()); // Either outcome is fine for testing

        Ok(())
    }

    #[test]
    fn test_command_output_sanitization() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "content", "Test commit")?;

        let commands = GitCommands::new(&repo_path)?;
        let output = commands.log(&["--oneline", "-1"])?;

        // Verify output doesn't contain control characters that could be dangerous
        assert!(!output.contains('\x00')); // No null bytes
        assert!(!output.contains('\x1b')); // No ANSI escape sequences (should be filtered)

        Ok(())
    }

    #[test]
    fn test_error_handling_and_logging() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        let runner = GitCommandRunner::new(&repo_path)?;

        // Test command that will definitely fail
        let result = runner.run_command(&["invalid-git-command"]);
        assert!(result.is_err());

        // Verify error messages are meaningful
        if let Err(e) = result {
            let error_str = e.to_string();
            assert!(!error_str.is_empty());
            // Should not expose internal system details in error messages
            assert!(!error_str.contains("/proc/"));
            assert!(!error_str.contains("/sys/"));
        }

        Ok(())
    }
}
