use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use tracing::debug;

/// Platform-specific security utilities
/// Implements defense measures against platform-specific attack vectors
pub struct PlatformSecurity;

impl PlatformSecurity {
    /// Get secure environment variables for the current platform
    pub fn get_secure_environment() -> HashMap<String, String> {
        let mut env = HashMap::new();

        #[cfg(windows)]
        {
            Self::add_windows_environment(&mut env);
        }

        #[cfg(unix)]
        {
            Self::add_unix_environment(&mut env);
        }

        #[cfg(target_os = "macos")]
        {
            Self::add_macos_environment(&mut env);
        }

        env
    }

    /// Validate and secure the PATH environment variable
    pub fn secure_path() -> Result<String> {
        #[cfg(windows)]
        {
            Self::secure_windows_path()
        }

        #[cfg(not(windows))]
        {
            Self::secure_unix_path()
        }
    }

    /// Find Git executable securely across platforms
    pub fn find_git_executable_secure() -> Result<PathBuf> {
        #[cfg(windows)]
        {
            Self::find_git_windows()
        }

        #[cfg(not(windows))]
        {
            Self::find_git_unix()
        }
    }

    /// Validate file permissions for security
    pub fn validate_file_permissions<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = path.as_ref();

        #[cfg(unix)]
        {
            Self::validate_unix_permissions(path)
        }

        #[cfg(windows)]
        {
            Self::validate_windows_permissions(path)
        }
    }

    /// Check if a directory is safe for Git operations
    pub fn is_safe_directory<P: AsRef<Path>>(path: P) -> bool {
        let path = path.as_ref();

        #[cfg(windows)]
        {
            Self::is_safe_directory_windows(path)
        }

        #[cfg(not(windows))]
        {
            Self::is_safe_directory_unix(path)
        }
    }
}

// Windows-specific implementations
#[cfg(windows)]
impl PlatformSecurity {
    fn add_windows_environment(env: &mut HashMap<String, String>) {
        debug!("Adding Windows-specific environment variables");

        // Essential Windows environment variables
        if let Ok(userprofile) = env::var("USERPROFILE") {
            if Self::is_safe_directory(&userprofile) {
                env.insert("USERPROFILE".to_string(), userprofile);
            }
        }

        if let Ok(username) = env::var("USERNAME") {
            // Sanitize username
            let safe_username = username
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect::<String>();
            if !safe_username.is_empty() {
                env.insert("USERNAME".to_string(), safe_username);
            }
        }

        if let Ok(computername) = env::var("COMPUTERNAME") {
            // Sanitize computer name
            let safe_computername = computername
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect::<String>();
            if !safe_computername.is_empty() {
                env.insert("COMPUTERNAME".to_string(), safe_computername);
            }
        }

        // System root (required for many Windows operations)
        if let Ok(systemroot) = env::var("SYSTEMROOT") {
            let systemroot_path = Path::new(&systemroot);
            if systemroot_path.exists() && systemroot_path.is_dir() {
                env.insert("SYSTEMROOT".to_string(), systemroot);
            }
        }

        // Windows directory
        if let Ok(windir) = env::var("WINDIR") {
            let windir_path = Path::new(&windir);
            if windir_path.exists() && windir_path.is_dir() {
                env.insert("WINDIR".to_string(), windir);
            }
        }

        // Temporary directory (use safe system temp)
        env.insert("TEMP".to_string(), "C:\\Windows\\Temp".to_string());
        env.insert("TMP".to_string(), "C:\\Windows\\Temp".to_string());

        // Application data path (if safe)
        if let Ok(appdata) = env::var("APPDATA") {
            if Self::is_safe_directory(&appdata) {
                env.insert("APPDATA".to_string(), appdata);
            }
        }
    }

    fn secure_windows_path() -> Result<String> {
        debug!("Securing Windows PATH");

        // Known safe directories for Git on Windows
        let safe_paths = vec![
            "C:\\Program Files\\Git\\cmd",
            "C:\\Program Files\\Git\\bin",
            "C:\\Program Files\\Git\\usr\\bin",
            "C:\\Program Files (x86)\\Git\\cmd",
            "C:\\Program Files (x86)\\Git\\bin",
            "C:\\Program Files (x86)\\Git\\usr\\bin",
            "C:\\Windows\\System32",
            "C:\\Windows",
            "C:\\Windows\\System32\\WindowsPowerShell\\v1.0",
        ];

        let mut verified_paths = Vec::new();

        for path in safe_paths {
            let path_obj = Path::new(path);
            if path_obj.exists() && path_obj.is_dir() {
                verified_paths.push(path);
            }
        }

        if verified_paths.is_empty() {
            return Err(anyhow!("No safe PATH directories found on Windows"));
        }

        Ok(verified_paths.join(";"))
    }

    fn find_git_windows() -> Result<PathBuf> {
        debug!("Finding Git executable on Windows");

        // Priority order for Git on Windows
        let git_paths = vec![
            "C:\\Program Files\\Git\\cmd\\git.exe",
            "C:\\Program Files\\Git\\bin\\git.exe",
            "C:\\Program Files (x86)\\Git\\cmd\\git.exe",
            "C:\\Program Files (x86)\\Git\\bin\\git.exe",
        ];

        for path in git_paths {
            let git_path = Path::new(path);
            if git_path.exists() && git_path.is_file() {
                return Ok(git_path.to_path_buf());
            }
        }

        // Fallback: check PATH (but only safe directories)
        if let Ok(path_env) = env::var("PATH") {
            let paths: Vec<&str> = path_env.split(';').collect();

            for path in paths {
                if Self::is_safe_windows_path_dir(path) {
                    let git_exe = Path::new(path).join("git.exe");
                    if git_exe.exists() && git_exe.is_file() {
                        return Ok(git_exe);
                    }
                }
            }
        }

        Err(anyhow!(
            "Git executable not found in safe locations on Windows"
        ))
    }

    fn is_safe_windows_path_dir(path: &str) -> bool {
        let path_lower = path.to_lowercase();

        // Whitelist approach - only allow known safe directories
        let safe_prefixes = [
            "c:\\program files\\git",
            "c:\\program files (x86)\\git",
            "c:\\windows\\system32",
            "c:\\windows",
        ];

        safe_prefixes
            .iter()
            .any(|prefix| path_lower.starts_with(prefix))
    }

    fn validate_windows_permissions(path: &Path) -> Result<()> {
        debug!("Validating Windows file permissions for: {:?}", path);

        if !path.exists() {
            return Err(anyhow!("Path does not exist: {:?}", path));
        }

        // Basic checks for Windows
        let metadata = path
            .metadata()
            .map_err(|e| anyhow!("Cannot read file metadata: {}", e))?;

        if metadata.is_file() && metadata.len() > 100_000_000 {
            // 100MB limit
            return Err(anyhow!("File too large: {:?}", path));
        }

        // Check if path is in a dangerous location
        let path_str = path.to_string_lossy().to_lowercase();
        let dangerous_locations = [
            "c:\\windows\\system32\\drivers",
            "c:\\windows\\system32\\config",
            "c:\\program files\\internet explorer",
            "c:\\program files\\common files\\microsoft shared",
        ];

        for dangerous in &dangerous_locations {
            if path_str.starts_with(dangerous) {
                return Err(anyhow!("Path in dangerous location: {:?}", path));
            }
        }

        Ok(())
    }

    fn is_safe_directory_windows(path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();

        // Block obviously dangerous directories
        let dangerous_patterns = [
            "c:\\windows\\system32\\drivers",
            "c:\\windows\\system32\\config",
            "c:\\windows\\temp",
            "c:\\users\\public",
            "c:\\programdata",
            "\\device\\",
            "\\\\?\\",
        ];

        for pattern in &dangerous_patterns {
            if path_str.contains(pattern) {
                return false;
            }
        }

        // Check for UNC paths
        if path_str.starts_with("\\\\") {
            return false;
        }

        true
    }
}

// Unix-specific implementations
#[cfg(unix)]
impl PlatformSecurity {
    fn add_unix_environment(env: &mut HashMap<String, String>) {
        debug!("Adding Unix-specific environment variables");

        if let Ok(home) = env::var("HOME") {
            if Self::is_safe_directory(&home) {
                env.insert("HOME".to_string(), home);
            }
        }

        if let Ok(user) = env::var("USER") {
            // Sanitize username
            let safe_user = user
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect::<String>();
            if !safe_user.is_empty() {
                env.insert("USER".to_string(), safe_user);
            }
        }

        if let Ok(shell) = env::var("SHELL") {
            if Path::new(&shell).exists() {
                env.insert("SHELL".to_string(), shell);
            }
        }

        // Safe temporary directory
        env.insert("TMPDIR".to_string(), "/tmp".to_string());
    }

    fn secure_unix_path() -> Result<String> {
        debug!("Securing Unix PATH");

        let safe_paths = vec![
            "/usr/bin",
            "/bin",
            "/usr/local/bin",
            "/opt/local/bin",
            "/usr/local/git/bin",
            "/opt/homebrew/bin", // macOS Homebrew
        ];

        let mut verified_paths = Vec::new();

        for path in safe_paths {
            let path_obj = Path::new(path);
            if path_obj.exists() && path_obj.is_dir() {
                verified_paths.push(path);
            }
        }

        if verified_paths.is_empty() {
            return Err(anyhow!("No safe PATH directories found"));
        }

        Ok(verified_paths.join(":"))
    }

    fn find_git_unix() -> Result<PathBuf> {
        debug!("Finding Git executable on Unix");

        let git_paths = vec![
            "/usr/bin/git",
            "/usr/local/bin/git",
            "/opt/local/bin/git",
            "/usr/local/git/bin/git",
            "/opt/homebrew/bin/git", // macOS Homebrew
        ];

        for path in git_paths {
            let git_path = Path::new(path);
            if git_path.exists() && git_path.is_file() {
                return Ok(git_path.to_path_buf());
            }
        }

        Err(anyhow!("Git executable not found in safe locations"))
    }

    fn validate_unix_permissions(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        debug!("Validating Unix file permissions for: {:?}", path);

        if !path.exists() {
            return Err(anyhow!("Path does not exist: {:?}", path));
        }

        let metadata = path
            .metadata()
            .map_err(|e| anyhow!("Cannot read file metadata: {}", e))?;

        let permissions = metadata.permissions();
        let mode = permissions.mode();

        // Check for suspicious permissions
        if mode & 0o002 != 0 {
            // World-writable
            return Err(anyhow!("File is world-writable: {:?}", path));
        }

        if metadata.is_file() {
            // Check file size limit
            if metadata.len() > 100_000_000 {
                // 100MB limit
                return Err(anyhow!("File too large: {:?}", path));
            }

            // Check for setuid/setgid bits
            if mode & 0o4000 != 0 || mode & 0o2000 != 0 {
                return Err(anyhow!(
                    "File has dangerous setuid/setgid permissions: {:?}",
                    path
                ));
            }
        }

        Ok(())
    }

    fn is_safe_directory_unix(path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Block dangerous directories
        let dangerous_patterns = [
            "/dev/",
            "/proc/",
            "/sys/",
            "/var/spool/",
            "/etc/shadow",
            "/etc/passwd",
            "/boot/",
            "/var/log/",
            "/tmp/",
            "/var/tmp/",
        ];

        for pattern in &dangerous_patterns {
            if path_str.starts_with(pattern) {
                return false;
            }
        }

        // Check for suspicious characters
        if path_str.contains("..") || path_str.contains("~") {
            return false;
        }

        true
    }
}

// macOS-specific implementations
#[cfg(target_os = "macos")]
impl PlatformSecurity {
    fn add_macos_environment(env: &mut HashMap<String, String>) {
        debug!("Adding macOS-specific environment variables");

        // macOS-specific paths
        if let Ok(home) = env::var("HOME") {
            env.insert("HOME".to_string(), home);
        }

        // Xcode command line tools path
        if Path::new("/Library/Developer/CommandLineTools").exists() {
            env.insert(
                "DEVELOPER_DIR".to_string(),
                "/Library/Developer/CommandLineTools".to_string(),
            );
        }
    }
}

/// File system security utilities
pub struct FileSystemSecurity;

impl FileSystemSecurity {
    /// Check if a file is safe to read based on various heuristics
    pub fn is_safe_to_read<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = path.as_ref();

        // Check basic file properties
        if !path.exists() {
            return Err(anyhow!("File does not exist"));
        }

        if !path.is_file() {
            return Err(anyhow!("Path is not a regular file"));
        }

        // Validate permissions
        PlatformSecurity::validate_file_permissions(path)?;

        // Check file extension safety
        if let Some(extension) = path.extension() {
            let ext_str = extension.to_string_lossy().to_lowercase();
            let dangerous_extensions = [
                "exe", "bat", "cmd", "com", "scr", "pif", "msi", "dll", "sys", "drv", "bin", "o",
                "so", "dylib",
            ];

            if dangerous_extensions.contains(&ext_str.as_str()) {
                return Err(anyhow!("Dangerous file extension: {}", ext_str));
            }
        }

        Ok(())
    }

    /// Sanitize a file path for safe operations
    pub fn sanitize_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let path = path.as_ref();

        // Convert to canonical form to resolve symlinks and relative paths
        let canonical = path
            .canonicalize()
            .map_err(|e| anyhow!("Cannot canonicalize path: {}", e))?;

        // Ensure the canonical path is safe
        if !PlatformSecurity::is_safe_directory(&canonical) {
            return Err(anyhow!("Path points to unsafe directory: {:?}", canonical));
        }

        Ok(canonical)
    }

    /// Create a secure temporary directory
    pub fn create_secure_temp_dir() -> Result<PathBuf> {
        #[cfg(windows)]
        let temp_base = Path::new("C:\\Windows\\Temp");

        #[cfg(not(windows))]
        let temp_base = Path::new("/tmp");

        if !temp_base.exists() {
            return Err(anyhow!("System temporary directory does not exist"));
        }

        // Create a random subdirectory name
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let temp_dir = temp_base.join(format!("gitk_rust_{}", timestamp));

        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| anyhow!("Cannot create temporary directory: {}", e))?;

        Ok(temp_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_security() {
        let env = PlatformSecurity::get_secure_environment();
        assert!(!env.is_empty());

        // Test PATH security
        assert!(PlatformSecurity::secure_path().is_ok());

        // Test Git executable finding
        // Note: This might fail in CI environments without Git
        let _git_result = PlatformSecurity::find_git_executable_secure();
    }

    #[test]
    fn test_filesystem_security() {
        // Test with a known safe file
        let safe_path = std::env::current_exe().unwrap();
        assert!(FileSystemSecurity::is_safe_to_read(&safe_path).is_ok());

        // Test path sanitization
        assert!(FileSystemSecurity::sanitize_path(".").is_ok());
    }
}
