use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use regex::Regex;
use tracing::{debug, warn, error};

/// Security validation and sanitization utilities
/// Implements defense-in-depth security measures for Git operations
pub struct SecurityValidator {
    allowed_commands: HashSet<String>,
    dangerous_patterns: Vec<Regex>,
    max_argument_length: usize,
    max_arguments_count: usize,
}

impl SecurityValidator {
    pub fn new() -> Result<Self> {
        let allowed_commands = Self::create_allowed_commands();
        let dangerous_patterns = Self::create_dangerous_patterns()?;
        
        Ok(Self {
            allowed_commands,
            dangerous_patterns,
            max_argument_length: 4096,
            max_arguments_count: 100,
        })
    }
    
    /// Create whitelist of allowed Git commands
    fn create_allowed_commands() -> HashSet<String> {
        let mut commands = HashSet::new();
        
        // Read-only Git commands only
        commands.insert("rev-list".to_string());
        commands.insert("log".to_string());
        commands.insert("show".to_string());
        commands.insert("diff".to_string());
        commands.insert("cat-file".to_string());
        commands.insert("ls-files".to_string());
        commands.insert("ls-tree".to_string());
        commands.insert("show-ref".to_string());
        commands.insert("for-each-ref".to_string());
        commands.insert("rev-parse".to_string());
        commands.insert("status".to_string());
        commands.insert("branch".to_string());
        commands.insert("tag".to_string());
        commands.insert("config".to_string());
        commands.insert("remote".to_string());
        commands.insert("describe".to_string());
        commands.insert("merge-base".to_string());
        commands.insert("name-rev".to_string());
        commands.insert("symbolic-ref".to_string());
        
        commands
    }
    
    /// Create patterns for dangerous argument detection
    fn create_dangerous_patterns() -> Result<Vec<Regex>> {
        let patterns = [
            // Command injection patterns
            r"[|&;`$()<>]",
            r"\\x[0-9a-fA-F]{2}",
            r"\\u[0-9a-fA-F]{4}",
            r"\\[0-7]{1,3}",
            
            // Path traversal patterns
            r"\.\./",
            r"\\\.\\./",
            r"/\.\./",
            r"\.\.\\",
            
            // Dangerous Git options
            r"--upload-pack",
            r"--receive-pack", 
            r"--exec",
            r"--ssh",
            r"--ext-cmd",
            r"--no-verify",
            r"--local",
            r"--shared",
            r"--bare",
            r"--git-dir=",
            r"--work-tree=",
            
            // Network-related patterns
            r"(https?|ssh|git|ftp)://",
            r"[a-zA-Z0-9.-]+@[a-zA-Z0-9.-]+:",
            
            // File URI patterns
            r"file://",
            
            // Shell metacharacters
            r"[\n\r\t\v\f]",
            r"[\x00-\x1f\x7f-\x9f]", // Control characters
        ];
        
        let mut compiled_patterns = Vec::new();
        for pattern in &patterns {
            compiled_patterns.push(Regex::new(pattern)?);
        }
        
        Ok(compiled_patterns)
    }
    
    /// Validate Git command for security
    pub fn validate_command(&self, command: &str) -> Result<()> {
        debug!("Validating command: {}", command);
        
        // Check if command is in whitelist
        if !self.allowed_commands.contains(command) {
            warn!("Blocked non-whitelisted command: {}", command);
            return Err(anyhow!("Command '{}' is not allowed", command));
        }
        
        Ok(())
    }
    
    /// Validate command arguments for security
    pub fn validate_arguments(&self, args: &[&str]) -> Result<()> {
        debug!("Validating {} arguments", args.len());
        
        // Check argument count
        if args.len() > self.max_arguments_count {
            return Err(anyhow!("Too many arguments: {} > {}", 
                             args.len(), self.max_arguments_count));
        }
        
        for (i, arg) in args.iter().enumerate() {
            self.validate_single_argument(i, arg)?;
        }
        
        Ok(())
    }
    
    /// Validate a single argument
    fn validate_single_argument(&self, index: usize, arg: &str) -> Result<()> {
        // Check argument length
        if arg.len() > self.max_argument_length {
            return Err(anyhow!("Argument {} too long: {} > {}", 
                             index, arg.len(), self.max_argument_length));
        }
        
        // Check for dangerous patterns
        for pattern in &self.dangerous_patterns {
            if pattern.is_match(arg) {
                warn!("Blocked dangerous pattern in argument {}: {}", index, arg);
                return Err(anyhow!("Argument {} contains dangerous pattern", index));
            }
        }
        
        // Additional specific validations
        self.validate_path_argument(arg)?;
        self.validate_ref_argument(arg)?;
        
        Ok(())
    }
    
    /// Validate path-like arguments
    fn validate_path_argument(&self, arg: &str) -> Result<()> {
        if arg.starts_with('-') && arg.len() > 1 {
            // This is likely an option, not a path
            return Ok(());
        }
        
        // Check for absolute paths outside repository
        if arg.starts_with('/') || (cfg!(windows) && arg.len() > 2 && arg.chars().nth(1) == Some(':')) {
            return Err(anyhow!("Absolute paths not allowed: {}", arg));
        }
        
        // Check for device files on Unix
        #[cfg(unix)]
        {
            if arg.starts_with("/dev/") || arg.starts_with("/proc/") || arg.starts_with("/sys/") {
                return Err(anyhow!("Device/system paths not allowed: {}", arg));
            }
        }
        
        Ok(())
    }
    
    /// Validate Git reference arguments
    fn validate_ref_argument(&self, arg: &str) -> Result<()> {
        // Skip non-ref-like arguments
        if !arg.contains("refs/") && !arg.starts_with("HEAD") && 
           !arg.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(());
        }
        
        // Validate ref names according to Git rules
        if arg.contains("..") || arg.contains("@{") || arg.contains("~") {
            // These are revision specifiers, validate them
            self.validate_revision_specifier(arg)?;
        }
        
        Ok(())
    }
    
    /// Validate Git revision specifiers
    fn validate_revision_specifier(&self, spec: &str) -> Result<()> {
        // Allow common safe revision specifiers
        let safe_patterns = [
            r"^[a-fA-F0-9]{4,40}$",          // SHA hashes
            r"^HEAD$",                        // HEAD reference
            r"^HEAD~\d+$",                    // HEAD~N
            r"^HEAD\^\d*$",                   // HEAD^N
            r"^refs/heads/[\w/-]+$",          // Branch refs
            r"^refs/tags/[\w/-]+$",           // Tag refs
            r"^refs/remotes/[\w/-]+$",        // Remote refs
            r"^[\w/-]+$",                     // Simple branch/tag names
        ];
        
        for pattern in &safe_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(spec) {
                    return Ok(());
                }
            }
        }
        
        warn!("Potentially unsafe revision specifier: {}", spec);
        Err(anyhow!("Unsafe revision specifier: {}", spec))
    }
    
    /// Sanitize file paths for safe access
    pub fn sanitize_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        let path = path.as_ref();
        
        // Convert to canonical form
        let canonical = path.canonicalize()
            .map_err(|e| anyhow!("Cannot canonicalize path: {}", e))?;
        
        // Ensure path doesn't escape repository boundaries
        // This would need repository context in real implementation
        
        Ok(canonical)
    }
    
    /// Validate environment variables for Git execution
    pub fn validate_environment(&self, env_vars: &std::collections::HashMap<String, String>) -> Result<()> {
        let dangerous_env_vars = [
            "LD_PRELOAD", "LD_LIBRARY_PATH", "DYLD_INSERT_LIBRARIES",
            "PATH", "GIT_EXEC_PATH", "GIT_SSH", "GIT_SSH_COMMAND",
            "GIT_PROXY_COMMAND", "GIT_CONNECT_TIMEOUT"
        ];
        
        for var in &dangerous_env_vars {
            if env_vars.contains_key(*var) {
                warn!("Potentially dangerous environment variable: {}", var);
                return Err(anyhow!("Environment variable '{}' not allowed", var));
            }
        }
        
        Ok(())
    }
}

/// Input sanitization utilities
pub struct InputSanitizer;

impl InputSanitizer {
    /// Sanitize user input for search queries
    pub fn sanitize_search_query(query: &str) -> Result<String> {
        // Limit length
        if query.len() > 1000 {
            return Err(anyhow!("Search query too long"));
        }
        
        // Remove control characters
        let sanitized: String = query
            .chars()
            .filter(|c| !c.is_control() || *c == '\t' || *c == '\n')
            .collect();
        
        // Basic validation for regex patterns if enabled
        if sanitized.contains("|") || sanitized.contains(".*") {
            // Validate as regex
            if let Err(e) = Regex::new(&sanitized) {
                return Err(anyhow!("Invalid regex pattern: {}", e));
            }
        }
        
        Ok(sanitized)
    }
    
    /// Sanitize commit ID input
    pub fn sanitize_commit_id(id: &str) -> Result<String> {
        // Must be valid hex string
        if !id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(anyhow!("Invalid commit ID format"));
        }
        
        // Reasonable length limits
        if id.len() < 4 || id.len() > 40 {
            return Err(anyhow!("Commit ID length must be 4-40 characters"));
        }
        
        Ok(id.to_lowercase())
    }
    
    /// Sanitize branch/tag names
    pub fn sanitize_ref_name(name: &str) -> Result<String> {
        // Git ref name rules
        if name.is_empty() || name.len() > 255 {
            return Err(anyhow!("Invalid reference name length"));
        }
        
        // Check for invalid characters
        let invalid_chars = [' ', '~', '^', ':', '?', '*', '[', '\\', '\x7f'];
        for ch in &invalid_chars {
            if name.contains(*ch) {
                return Err(anyhow!("Reference name contains invalid character: {}", ch));
            }
        }
        
        // Additional Git rules
        if name.starts_with('-') || name.ends_with('.') || 
           name.contains("..") || name.contains("@{") {
            return Err(anyhow!("Reference name violates Git naming rules"));
        }
        
        Ok(name.to_string())
    }
    
    /// Sanitize file paths from user input
    pub fn sanitize_file_path(path: &str) -> Result<String> {
        // Length check
        if path.len() > 4096 {
            return Err(anyhow!("File path too long"));
        }
        
        // Check for path traversal
        if path.contains("../") || path.contains("..\\") {
            return Err(anyhow!("Path traversal not allowed"));
        }
        
        // No absolute paths
        if path.starts_with('/') || (cfg!(windows) && path.len() > 2 && path.chars().nth(1) == Some(':')) {
            return Err(anyhow!("Absolute paths not allowed"));
        }
        
        // Normalize path separators
        #[cfg(windows)]
        let normalized = path.replace('/', "\\");
        #[cfg(not(windows))]
        let normalized = path.replace('\\', "/");
        
        Ok(normalized)
    }
    
    /// Sanitize commit message for safety
    pub fn sanitize_commit_message(message: &str) -> Result<String> {
        // Remove null bytes and other control characters
        let sanitized = message
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect::<String>();
        
        // Limit message length
        let truncated = if sanitized.len() > 2048 {
            sanitized[..2048].to_string()
        } else {
            sanitized
        };
        
        Ok(truncated)
    }
}

/// Rate limiting for command execution
pub struct RateLimiter {
    max_commands_per_second: u32,
    max_concurrent_commands: u32,
    command_history: Vec<std::time::Instant>,
    active_commands: std::sync::atomic::AtomicU32,
}

impl RateLimiter {
    pub fn new(max_commands_per_second: u32, max_concurrent: u32) -> Self {
        Self {
            max_commands_per_second,
            max_concurrent_commands: max_concurrent,
            command_history: Vec::new(),
            active_commands: std::sync::atomic::AtomicU32::new(0),
        }
    }
    
    /// Check if command execution is allowed
    pub fn check_rate_limit(&mut self) -> Result<()> {
        let now = std::time::Instant::now();
        
        // Check concurrent commands
        let active = self.active_commands.load(std::sync::atomic::Ordering::Relaxed);
        if active >= self.max_concurrent_commands {
            return Err(anyhow!("Too many concurrent commands"));
        }
        
        // Clean old entries
        self.command_history.retain(|&time| now.duration_since(time).as_secs() < 1);
        
        // Check rate limit
        if self.command_history.len() as u32 >= self.max_commands_per_second {
            return Err(anyhow!("Rate limit exceeded"));
        }
        
        self.command_history.push(now);
        Ok(())
    }
    
    /// Increment active command count
    pub fn start_command(&self) {
        self.active_commands.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// Decrement active command count
    pub fn end_command(&self) {
        self.active_commands.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_security_validator() {
        let validator = SecurityValidator::new().unwrap();
        
        // Test allowed commands
        assert!(validator.validate_command("log").is_ok());
        assert!(validator.validate_command("rev-list").is_ok());
        
        // Test blocked commands
        assert!(validator.validate_command("push").is_err());
        assert!(validator.validate_command("commit").is_err());
        
        // Test dangerous arguments
        assert!(validator.validate_arguments(&["--upload-pack"]).is_err());
        assert!(validator.validate_arguments(&["../../../etc/passwd"]).is_err());
        assert!(validator.validate_arguments(&["|rm -rf /"]).is_err());
    }
    
    #[test]
    fn test_input_sanitizer() {
        // Test commit ID sanitization
        assert!(InputSanitizer::sanitize_commit_id("abc123").is_ok());
        assert!(InputSanitizer::sanitize_commit_id("xyz").is_err());
        
        // Test ref name sanitization
        assert!(InputSanitizer::sanitize_ref_name("feature/branch").is_ok());
        assert!(InputSanitizer::sanitize_ref_name("../../../etc/passwd").is_err());
        assert!(InputSanitizer::sanitize_ref_name("refs/heads/main").is_ok());
        
        // Test search query sanitization
        assert!(InputSanitizer::sanitize_search_query("search term").is_ok());
        assert!(InputSanitizer::sanitize_search_query("").is_err());
        assert!(InputSanitizer::sanitize_search_query(&"x".repeat(2000)).is_err());
    }
    
    #[test]
    fn test_input_validator() {
        // Test commit ID validation
        assert!(InputValidator::validate_commit_id("a1b2c3d4e5f6789012345678901234567890abcd").is_ok());
        assert!(InputValidator::validate_commit_id("short").is_err());
        assert!(InputValidator::validate_commit_id("invalid_chars!").is_err());
        assert!(InputValidator::validate_commit_id("").is_err());
        
        // Test search query validation
        assert!(InputValidator::validate_search_query("valid search").is_ok());
        assert!(InputValidator::validate_search_query("").is_err());
        assert!(InputValidator::validate_search_query(&"x".repeat(1500)).is_err());
        
        // Test branch name validation
        assert!(InputValidator::validate_ref_name("main").is_ok());
        assert!(InputValidator::validate_ref_name("feature/branch-name").is_ok());
        assert!(InputValidator::validate_ref_name("refs/heads/main").is_ok());
        assert!(InputValidator::validate_ref_name("").is_err());
        assert!(InputValidator::validate_ref_name("../invalid").is_err());
        assert!(InputValidator::validate_ref_name("branch with spaces").is_err());
    }
    
    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(2, 1);
        
        // First command should be allowed
        assert!(limiter.check_rate_limit().is_ok());
        limiter.start_command();
        
        // Second command should be blocked due to concurrent limit
        assert!(limiter.check_rate_limit().is_err());
        
        // End first command
        limiter.end_command();
        
        // Now second command should be allowed
        assert!(limiter.check_rate_limit().is_ok());
    }
    
    #[test]
    fn test_sanitize_commit_message() {
        // Test normal commit message
        let normal_msg = "Add new feature\n\nThis adds a new feature to the application.";
        assert_eq!(InputSanitizer::sanitize_commit_message(normal_msg).unwrap(), normal_msg);
        
        // Test message with control characters
        let msg_with_controls = "Add feature\x00with\x01null\x02bytes";
        let sanitized = InputSanitizer::sanitize_commit_message(msg_with_controls).unwrap();
        assert!(!sanitized.contains('\x00'));
        assert!(!sanitized.contains('\x01'));
        assert!(!sanitized.contains('\x02'));
        
        // Test very long message
        let long_msg = "x".repeat(3000);
        let sanitized = InputSanitizer::sanitize_commit_message(&long_msg).unwrap();
        assert!(sanitized.len() <= 2048);
        
        // Test message with tabs and newlines (should be preserved)
        let msg_with_whitespace = "First line\n\tSecond line with tab";
        let sanitized = InputSanitizer::sanitize_commit_message(msg_with_whitespace).unwrap();
        assert_eq!(sanitized, msg_with_whitespace);
    }
    
    #[test]
    fn test_path_traversal_detection() {
        let validator = SecurityValidator::new().unwrap();
        
        // Valid paths
        assert!(validator.validate_arguments(&["file.txt"]).is_ok());
        assert!(validator.validate_arguments(&["src/main.rs"]).is_ok());
        assert!(validator.validate_arguments(&["docs/README.md"]).is_ok());
        
        // Path traversal attempts
        assert!(validator.validate_arguments(&["../../../etc/passwd"]).is_err());
        assert!(validator.validate_arguments(&["..\\..\\windows\\system32"]).is_err());
        assert!(validator.validate_arguments(&["/etc/passwd"]).is_err());
        assert!(validator.validate_arguments(&["C:\\Windows\\System32"]).is_err());
    }
    
    #[test]
    fn test_command_injection_prevention() {
        let validator = SecurityValidator::new().unwrap();
        
        // Command injection attempts
        assert!(validator.validate_arguments(&["; rm -rf /"]).is_err());
        assert!(validator.validate_arguments(&["| cat /etc/passwd"]).is_err());
        assert!(validator.validate_arguments(&["&& echo pwned"]).is_err());
        assert!(validator.validate_arguments(&["$(whoami)"]).is_err());
        assert!(validator.validate_arguments(&["`id`"]).is_err());
        
        // Valid arguments
        assert!(validator.validate_arguments(&["--oneline"]).is_ok());
        assert!(validator.validate_arguments(&["HEAD~5"]).is_ok());
        assert!(validator.validate_arguments(&["feature/branch"]).is_ok());
    }
    
    #[test]
    fn test_dangerous_git_options() {
        let validator = SecurityValidator::new().unwrap();
        
        // Dangerous Git options that should be blocked
        assert!(validator.validate_arguments(&["--upload-pack=/bin/sh"]).is_err());
        assert!(validator.validate_arguments(&["--receive-pack=evil"]).is_err());
        assert!(validator.validate_arguments(&["--ext=sh -c 'rm -rf /'"]).is_err());
        assert!(validator.validate_arguments(&["--config=alias.test=!sh"]).is_err());
        
        // Safe Git options
        assert!(validator.validate_arguments(&["--pretty=oneline"]).is_ok());
        assert!(validator.validate_arguments(&["--graph"]).is_ok());
        assert!(validator.validate_arguments(&["--decorate"]).is_ok());
    }
    
    use test_case::test_case;
    use proptest::prelude::*;
    
    #[test_case("a1b2c3d4e5f6789012345678901234567890abcd", true; "valid full SHA")]
    #[test_case("a1b2c3d", true; "valid short SHA")]
    #[test_case("HEAD", true; "HEAD reference")]
    #[test_case("", false; "empty string")]
    #[test_case("invalid!", false; "invalid characters")]
    #[test_case("xyz", false; "too short")]
    fn test_commit_id_validation_cases(commit_id: &str, should_be_valid: bool) {
        let result = InputValidator::validate_commit_id(commit_id);
        assert_eq!(result.is_ok(), should_be_valid);
    }
    
    #[test_case("main", true; "simple branch name")]
    #[test_case("feature/branch-name", true; "feature branch")]
    #[test_case("refs/heads/main", true; "full ref path")]
    #[test_case("", false; "empty string")]
    #[test_case("../invalid", false; "path traversal")]
    #[test_case("branch with spaces", false; "spaces not allowed")]
    #[test_case("branch\twith\ttabs", false; "tabs not allowed")]
    fn test_ref_name_validation_cases(ref_name: &str, should_be_valid: bool) {
        let result = InputValidator::validate_ref_name(ref_name);
        assert_eq!(result.is_ok(), should_be_valid);
    }
    
    // Property-based tests
    proptest! {
        #[test]
        fn test_commit_id_sanitization_preserves_valid_ids(
            id in "[a-f0-9]{7,40}"
        ) {
            let result = InputSanitizer::sanitize_commit_id(&id);
            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap(), id);
        }
        
        #[test]
        fn test_search_query_length_limits(
            query in "[a-zA-Z0-9 ]{1,100}"
        ) {
            let result = InputSanitizer::sanitize_search_query(&query);
            prop_assert!(result.is_ok());
            let sanitized = result.unwrap();
            prop_assert!(sanitized.len() <= 1000);
        }
        
        #[test]
        fn test_ref_name_sanitization_rejects_invalid_chars(
            ref_name in ".*[^a-zA-Z0-9/_-].*"
        ) {
            // This tests that ref names with invalid characters are rejected
            if ref_name.contains("..") || ref_name.contains(' ') || ref_name.contains('\t') {
                let result = InputValidator::validate_ref_name(&ref_name);
                prop_assert!(result.is_err());
            }
        }
    }
}