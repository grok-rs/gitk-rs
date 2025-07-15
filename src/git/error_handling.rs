use anyhow::{anyhow, Result};
use thiserror::Error;
use std::fmt;
use tracing::{error, warn, debug};

/// Comprehensive error types for Git operations
#[derive(Error, Debug)]
pub enum GitError {
    #[error("Repository error: {message}")]
    Repository { message: String },
    
    #[error("Security violation: {violation}")]
    Security { violation: String },
    
    #[error("Command execution failed: {command} - {reason}")]
    CommandExecution { command: String, reason: String },
    
    #[error("Invalid input: {input} - {reason}")]
    InvalidInput { input: String, reason: String },
    
    #[error("File system error: {path} - {reason}")]
    FileSystem { path: String, reason: String },
    
    #[error("Network error: {operation} - {reason}")]
    Network { operation: String, reason: String },
    
    #[error("Configuration error: {setting} - {reason}")]
    Configuration { setting: String, reason: String },
    
    #[error("Permission denied: {resource} - {reason}")]
    PermissionDenied { resource: String, reason: String },
    
    #[error("Resource limit exceeded: {resource} - {limit}")]
    ResourceLimit { resource: String, limit: String },
    
    #[error("Timeout: {operation} took longer than {timeout_ms}ms")]
    Timeout { operation: String, timeout_ms: u64 },
    
    #[error("Rate limit exceeded: {operation}")]
    RateLimit { operation: String },
    
    #[error("Git internal error: {0}")]
    Git2(#[from] git2::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

impl GitError {
    /// Create a security violation error
    pub fn security(violation: impl Into<String>) -> Self {
        let violation = violation.into();
        error!("Security violation: {}", violation);
        GitError::Security { violation }
    }
    
    /// Create an invalid input error
    pub fn invalid_input(input: impl Into<String>, reason: impl Into<String>) -> Self {
        let input = input.into();
        let reason = reason.into();
        warn!("Invalid input '{}': {}", input, reason);
        GitError::InvalidInput { input, reason }
    }
    
    /// Create a command execution error
    pub fn command_failed(command: impl Into<String>, reason: impl Into<String>) -> Self {
        let command = command.into();
        let reason = reason.into();
        error!("Command '{}' failed: {}", command, reason);
        GitError::CommandExecution { command, reason }
    }
    
    /// Create a resource limit error
    pub fn resource_limit(resource: impl Into<String>, limit: impl Into<String>) -> Self {
        let resource = resource.into();
        let limit = limit.into();
        warn!("Resource limit exceeded for '{}': {}", resource, limit);
        GitError::ResourceLimit { resource, limit }
    }
    
    /// Create a timeout error
    pub fn timeout(operation: impl Into<String>, timeout_ms: u64) -> Self {
        let operation = operation.into();
        warn!("Operation '{}' timed out after {}ms", operation, timeout_ms);
        GitError::Timeout { operation, timeout_ms }
    }
    
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            GitError::Security { .. } => false,
            GitError::PermissionDenied { .. } => false,
            GitError::InvalidInput { .. } => false,
            GitError::Timeout { .. } => true,
            GitError::RateLimit { .. } => true,
            GitError::Network { .. } => true,
            GitError::CommandExecution { .. } => true,
            GitError::Repository { .. } => true,
            GitError::FileSystem { .. } => false,
            GitError::Configuration { .. } => false,
            GitError::ResourceLimit { .. } => true,
            GitError::Git2(_) => true,
            GitError::Io(_) => true,
            GitError::Serialization(_) => false,
            GitError::Utf8(_) => false,
            GitError::Regex(_) => false,
        }
    }
    
    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            GitError::Security { .. } => ErrorSeverity::Critical,
            GitError::PermissionDenied { .. } => ErrorSeverity::High,
            GitError::InvalidInput { .. } => ErrorSeverity::Medium,
            GitError::Timeout { .. } => ErrorSeverity::Low,
            GitError::RateLimit { .. } => ErrorSeverity::Low,
            GitError::Network { .. } => ErrorSeverity::Medium,
            GitError::CommandExecution { .. } => ErrorSeverity::Medium,
            GitError::Repository { .. } => ErrorSeverity::Medium,
            GitError::FileSystem { .. } => ErrorSeverity::High,
            GitError::Configuration { .. } => ErrorSeverity::Medium,
            GitError::ResourceLimit { .. } => ErrorSeverity::Medium,
            GitError::Git2(_) => ErrorSeverity::Medium,
            GitError::Io(_) => ErrorSeverity::Medium,
            GitError::Serialization(_) => ErrorSeverity::Low,
            GitError::Utf8(_) => ErrorSeverity::Low,
            GitError::Regex(_) => ErrorSeverity::Low,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Low => write!(f, "LOW"),
            ErrorSeverity::Medium => write!(f, "MEDIUM"),
            ErrorSeverity::High => write!(f, "HIGH"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Input validation utilities
pub struct InputValidator;

impl InputValidator {
    /// Validate commit ID format and length
    pub fn validate_commit_id(id: &str) -> Result<(), GitError> {
        if id.is_empty() {
            return Err(GitError::invalid_input(id, "Commit ID cannot be empty"));
        }
        
        if id.len() < 4 {
            return Err(GitError::invalid_input(id, "Commit ID too short (minimum 4 characters)"));
        }
        
        if id.len() > 40 {
            return Err(GitError::invalid_input(id, "Commit ID too long (maximum 40 characters)"));
        }
        
        if !id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(GitError::invalid_input(id, "Commit ID contains non-hexadecimal characters"));
        }
        
        Ok(())
    }
    
    /// Validate reference name (branch/tag)
    pub fn validate_ref_name(name: &str) -> Result<(), GitError> {
        if name.is_empty() {
            return Err(GitError::invalid_input(name, "Reference name cannot be empty"));
        }
        
        if name.len() > 255 {
            return Err(GitError::invalid_input(name, "Reference name too long (maximum 255 characters)"));
        }
        
        // Git ref name rules
        let invalid_chars = [' ', '~', '^', ':', '?', '*', '[', '\\', '\x7f', '\n', '\r'];
        for ch in &invalid_chars {
            if name.contains(*ch) {
                return Err(GitError::invalid_input(name, 
                    format!("Reference name contains invalid character: {}", ch)));
            }
        }
        
        if name.starts_with('-') || name.ends_with('.') || 
           name.contains("..") || name.contains("@{") {
            return Err(GitError::invalid_input(name, "Reference name violates Git naming rules"));
        }
        
        Ok(())
    }
    
    /// Validate commit message
    pub fn validate_commit_message(message: &str) -> Result<(), GitError> {
        if message.is_empty() {
            return Err(GitError::invalid_input(message, "Commit message cannot be empty"));
        }
        
        if message.len() > 8192 {
            return Err(GitError::invalid_input(message, "Commit message too long (maximum 8192 characters)"));
        }
        
        // Check for null bytes
        if message.contains('\0') {
            return Err(GitError::invalid_input(message, "Commit message contains null bytes"));
        }
        
        Ok(())
    }
    
    /// Validate search query
    pub fn validate_search_query(query: &str) -> Result<(), GitError> {
        if query.len() > 1000 {
            return Err(GitError::invalid_input(query, "Search query too long (maximum 1000 characters)"));
        }
        
        // Check for control characters (except tab and newline)
        for ch in query.chars() {
            if ch.is_control() && ch != '\t' && ch != '\n' {
                return Err(GitError::invalid_input(query, 
                    format!("Search query contains control character: {:?}", ch)));
            }
        }
        
        // If it looks like a regex, validate it
        if query.contains('|') || query.contains(".*") || query.contains("\\") {
            if let Err(e) = regex::Regex::new(query) {
                return Err(GitError::invalid_input(query, 
                    format!("Invalid regex pattern: {}", e)));
            }
        }
        
        Ok(())
    }
    
    /// Validate file path
    pub fn validate_file_path(path: &str) -> Result<(), GitError> {
        if path.is_empty() {
            return Err(GitError::invalid_input(path, "File path cannot be empty"));
        }
        
        if path.len() > 4096 {
            return Err(GitError::invalid_input(path, "File path too long (maximum 4096 characters)"));
        }
        
        // Check for path traversal
        if path.contains("../") || path.contains("..\\") {
            return Err(GitError::security("Path traversal detected in file path"));
        }
        
        // No absolute paths for security
        if path.starts_with('/') || (cfg!(windows) && path.len() > 2 && path.chars().nth(1) == Some(':')) {
            return Err(GitError::security("Absolute paths not allowed"));
        }
        
        // Check for null bytes
        if path.contains('\0') {
            return Err(GitError::invalid_input(path, "File path contains null byte"));
        }
        
        Ok(())
    }
    
    /// Validate command arguments
    pub fn validate_command_args(args: &[&str]) -> Result<(), GitError> {
        if args.len() > 100 {
            return Err(GitError::resource_limit("command arguments", "100"));
        }
        
        for (i, arg) in args.iter().enumerate() {
            if arg.len() > 4096 {
                return Err(GitError::invalid_input(
                    format!("arg[{}]", i), 
                    "Argument too long (maximum 4096 characters)"
                ));
            }
            
            // Check for dangerous characters
            let dangerous_chars = ['|', '&', ';', '`', '$', '<', '>', '\n', '\r', '\0'];
            for ch in &dangerous_chars {
                if arg.contains(*ch) {
                    return Err(GitError::security(
                        format!("Dangerous character '{}' in argument", ch)
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate numeric input with bounds
    pub fn validate_numeric_input(value: i64, min: i64, max: i64, name: &str) -> Result<(), GitError> {
        if value < min || value > max {
            return Err(GitError::invalid_input(
                value.to_string(),
                format!("{} must be between {} and {}", name, min, max)
            ));
        }
        Ok(())
    }
}

/// Error recovery and retry logic
pub struct ErrorRecovery;

impl ErrorRecovery {
    /// Attempt to recover from an error with exponential backoff
    pub async fn retry_with_backoff<F, T>(
        operation: F,
        max_attempts: u32,
        base_delay_ms: u64,
        operation_name: &str,
    ) -> Result<T, GitError>
    where
        F: Fn() -> Result<T, GitError>,
    {
        let mut attempt = 0;
        let mut delay = base_delay_ms;
        
        loop {
            attempt += 1;
            
            match operation() {
                Ok(result) => {
                    if attempt > 1 {
                        debug!("Operation '{}' succeeded on attempt {}", operation_name, attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    if attempt >= max_attempts {
                        error!("Operation '{}' failed after {} attempts: {}", 
                               operation_name, max_attempts, e);
                        return Err(e);
                    }
                    
                    if !e.is_recoverable() {
                        error!("Operation '{}' failed with non-recoverable error: {}", 
                               operation_name, e);
                        return Err(e);
                    }
                    
                    warn!("Operation '{}' failed on attempt {} ({}), retrying in {}ms", 
                          operation_name, attempt, e, delay);
                    
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    delay = std::cmp::min(delay * 2, 5000); // Cap at 5 seconds
                }
            }
        }
    }
    
    /// Check if error requires immediate abort
    pub fn should_abort(error: &GitError) -> bool {
        match error.severity() {
            ErrorSeverity::Critical => true,
            ErrorSeverity::High => true,
            _ => false,
        }
    }
    
    /// Get user-friendly error message
    pub fn user_friendly_message(error: &GitError) -> String {
        match error {
            GitError::Security { .. } => {
                "Security error: This operation is not allowed for safety reasons.".to_string()
            }
            GitError::PermissionDenied { resource, .. } => {
                format!("Permission denied: Cannot access {}. Please check file permissions.", resource)
            }
            GitError::InvalidInput { .. } => {
                "Invalid input: Please check your input and try again.".to_string()
            }
            GitError::Timeout { operation, .. } => {
                format!("Operation '{}' timed out. Please try again.", operation)
            }
            GitError::RateLimit { .. } => {
                "Too many requests. Please wait a moment and try again.".to_string()
            }
            GitError::Network { .. } => {
                "Network error: Please check your connection and try again.".to_string()
            }
            GitError::CommandExecution { command, .. } => {
                format!("Command '{}' failed. Please check the repository state.", command)
            }
            GitError::Repository { .. } => {
                "Repository error: The Git repository may be corrupted or inaccessible.".to_string()
            }
            GitError::FileSystem { .. } => {
                "File system error: Please check file permissions and disk space.".to_string()
            }
            GitError::Configuration { .. } => {
                "Configuration error: Please check your Git configuration.".to_string()
            }
            GitError::ResourceLimit { resource, .. } => {
                format!("Resource limit exceeded for {}. Please reduce the operation scope.", resource)
            }
            GitError::Git2(e) => {
                format!("Git error: {}", e)
            }
            GitError::Io(e) => {
                format!("File operation failed: {}", e)
            }
            _ => {
                "An unexpected error occurred. Please try again.".to_string()
            }
        }
    }
}

/// Error reporting and logging utilities
pub struct ErrorReporter;

impl ErrorReporter {
    /// Log error with appropriate level based on severity
    pub fn log_error(error: &GitError, context: &str) {
        match error.severity() {
            ErrorSeverity::Critical => {
                error!("[CRITICAL] {}: {}", context, error);
            }
            ErrorSeverity::High => {
                error!("[HIGH] {}: {}", context, error);
            }
            ErrorSeverity::Medium => {
                warn!("[MEDIUM] {}: {}", context, error);
            }
            ErrorSeverity::Low => {
                debug!("[LOW] {}: {}", context, error);
            }
        }
    }
    
    /// Create error report for debugging
    pub fn create_error_report(error: &GitError, context: &str) -> String {
        format!(
            "Error Report\n\
            Context: {}\n\
            Error: {}\n\
            Severity: {}\n\
            Recoverable: {}\n\
            Timestamp: {}\n",
            context,
            error,
            error.severity(),
            error.is_recoverable(),
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        )
    }
}

/// Resource monitoring for preventing resource exhaustion
pub struct ResourceMonitor {
    max_memory_mb: u64,
    max_open_files: u32,
    max_concurrent_operations: u32,
    current_operations: std::sync::atomic::AtomicU32,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            max_memory_mb: 512, // 512MB limit
            max_open_files: 100,
            max_concurrent_operations: 10,
            current_operations: std::sync::atomic::AtomicU32::new(0),
        }
    }
    
    /// Check if operation can proceed without exceeding limits
    pub fn check_resource_limits(&self) -> Result<(), GitError> {
        let current_ops = self.current_operations.load(std::sync::atomic::Ordering::Relaxed);
        if current_ops >= self.max_concurrent_operations {
            return Err(GitError::resource_limit(
                "concurrent operations",
                self.max_concurrent_operations.to_string()
            ));
        }
        
        // Check memory usage (simplified)
        if let Ok(memory_info) = sys_info::mem_info() {
            let used_mb = (memory_info.total - memory_info.avail) / 1024;
            if used_mb > self.max_memory_mb {
                return Err(GitError::resource_limit(
                    "memory usage",
                    format!("{}MB", self.max_memory_mb)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Start tracking an operation
    pub fn start_operation(&self) -> OperationGuard {
        self.current_operations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        OperationGuard { monitor: self }
    }
}

/// RAII guard for operation tracking
pub struct OperationGuard<'a> {
    monitor: &'a ResourceMonitor,
}

impl<'a> Drop for OperationGuard<'a> {
    fn drop(&mut self) {
        self.monitor.current_operations.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_input_validation() {
        // Test commit ID validation
        assert!(InputValidator::validate_commit_id("abc123").is_ok());
        assert!(InputValidator::validate_commit_id("xyz").is_err());
        assert!(InputValidator::validate_commit_id("").is_err());
        
        // Test ref name validation
        assert!(InputValidator::validate_ref_name("feature/branch").is_ok());
        assert!(InputValidator::validate_ref_name("../evil").is_err());
        
        // Test search query validation
        assert!(InputValidator::validate_search_query("normal search").is_ok());
        assert!(InputValidator::validate_search_query("a".repeat(2000)).is_err());
        
        // Test file path validation
        assert!(InputValidator::validate_file_path("src/main.rs").is_ok());
        assert!(InputValidator::validate_file_path("../../../etc/passwd").is_err());
    }
    
    #[test]
    fn test_error_properties() {
        let security_error = GitError::security("test violation");
        assert!(!security_error.is_recoverable());
        assert_eq!(security_error.severity(), ErrorSeverity::Critical);
        
        let timeout_error = GitError::timeout("test operation", 5000);
        assert!(timeout_error.is_recoverable());
        assert_eq!(timeout_error.severity(), ErrorSeverity::Low);
    }
}