# Security Policy

## Overview

The gitk-rs project takes security seriously. As a Git repository browser that executes Git commands and handles repository data, we implement multiple layers of security to protect users and their repositories.

## Supported Versions

We provide security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 1.x.x   | ✅ Full support    |
| 0.9.x   | ✅ Critical fixes  |
| < 0.9   | ❌ No support      |

## Reporting Security Vulnerabilities

**Please do not report security vulnerabilities through public GitHub issues.**

### Preferred Method

Send security reports to: **[SECURITY EMAIL - TO BE CONFIGURED]**

### What to Include

- **Vulnerability Description**: Clear description of the security issue
- **Impact Assessment**: Potential impact and attack scenarios
- **Reproduction Steps**: Detailed steps to reproduce the vulnerability
- **Environment Details**: OS, Rust version, gitk-rs version
- **Proposed Solution**: If you have suggestions for fixes
- **Timeline**: Any constraints on disclosure timing

### Response Timeline

- **Initial Response**: Within 48 hours of report
- **Assessment**: Within 1 week of initial response
- **Fix Development**: Depends on severity and complexity
- **Disclosure**: Coordinated with reporter

## Security Architecture

### Input Validation and Sanitization

gitk-rs implements comprehensive input validation:

```rust
// Example: Command argument sanitization
pub fn sanitize_git_arg(arg: &str) -> Result<String, SecurityError> {
    // Remove shell metacharacters
    // Validate against allowlists
    // Escape special characters
    // Verify path traversal protection
}
```

**Protection Layers:**
- Shell command injection prevention
- Path traversal protection
- Git argument validation
- File extension filtering
- Size and length limits

### Git Command Security

All Git operations use safe wrappers:

- **Argument Sanitization**: All command arguments are validated
- **Command Allowlisting**: Only approved Git commands are executed
- **Path Validation**: Repository paths are validated and contained
- **Process Isolation**: Git processes run with limited privileges

### Memory Safety

Rust's ownership system provides memory safety, but we also:

- Use `#![forbid(unsafe_code)]` where possible
- Audit all `unsafe` blocks in dependencies
- Regular dependency security scans
- Memory usage monitoring and limits

### File System Security

- **Sandbox Mode**: Optional repository sandboxing
- **Path Validation**: Prevent access outside repository boundaries
- **Symlink Protection**: Safe symlink resolution
- **Permission Checks**: Verify file/directory permissions

## Security Features

### Built-in Protections

1. **Command Injection Prevention**
   ```rust
   // Safe command construction
   let output = Command::new("git")
       .arg("log")
       .arg("--oneline")
       .arg(validated_commit_id)
       .output()?;
   ```

2. **Path Traversal Protection**
   ```rust
   // Path validation
   fn validate_repository_path(path: &Path) -> Result<(), SecurityError> {
       // Canonicalize path
       // Check for traversal attempts
       // Verify within allowed directories
   }
   ```

3. **Input Size Limits**
   - Maximum file size for diff viewing
   - Commit message length limits
   - Repository path length limits
   - Command line argument limits

### Security Configuration

**Configuration Options:**
```toml
[security]
# Enable strict mode for additional security checks
strict_mode = true

# Maximum file size for diff viewing (bytes)
max_diff_file_size = 10485760  # 10MB

# Allowed git commands (allowlist)
allowed_commands = ["log", "show", "diff", "branch", "tag"]

# Repository access restrictions
sandbox_mode = false
allowed_repo_paths = ["/home/user/projects"]
```

### Audit Logging

Security-relevant events are logged:

```rust
// Security event logging
security_log!("repository_access", {
    "path": repo_path,
    "user": current_user,
    "action": "open",
    "timestamp": Utc::now(),
});
```

**Logged Events:**
- Repository access attempts
- Command executions
- Permission failures
- Configuration changes
- Error conditions

## Dependency Security

### Supply Chain Security

- **Dependency Auditing**: Regular `cargo audit` scans
- **License Compliance**: Automated license checking
- **SBOM Generation**: Software Bill of Materials for releases
- **Vulnerability Scanning**: Continuous dependency monitoring

### Dependency Management

```toml
# Security-focused dependency selection
[dependencies]
# Prefer well-maintained, audited crates
git2 = "0.18"           # Official libgit2 bindings
tokio = "1.0"           # Async runtime
serde = "1.0"           # Serialization
anyhow = "1.0"          # Error handling

[dev-dependencies]
# Security testing tools
proptest = "1.0"        # Property-based testing
```

### Automated Security Checks

- **Daily Security Scans**: Automated vulnerability scanning
- **Dependency Updates**: Automated dependency update PRs
- **License Compliance**: Continuous license validation
- **Code Analysis**: Static analysis with CodeQL

## Threat Model

### Assets

- **User Repositories**: Git repository data and history
- **System Access**: Local file system access
- **User Credentials**: Git authentication credentials
- **Application Data**: Configuration and cache files

### Threats

1. **Malicious Repositories**
   - Git hooks execution
   - Large file attacks
   - Symlink attacks
   - Binary file exploits

2. **Command Injection**
   - Shell metacharacter injection
   - Argument injection
   - Environment variable manipulation

3. **Path Traversal**
   - Directory traversal attacks
   - Symlink following
   - Absolute path injection

4. **Resource Exhaustion**
   - Memory exhaustion
   - CPU exhaustion
   - Disk space exhaustion
   - Network resource abuse

### Mitigations

- Input validation and sanitization
- Command allowlisting and argument validation
- Path canonicalization and validation
- Resource limits and monitoring
- Process isolation and sandboxing

## Security Testing

### Automated Testing

```rust
#[cfg(test)]
mod security_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_command_injection_prevention(
            malicious_input in r"[;&|`$(){}[\]\\]+"
        ) {
            let result = sanitize_git_arg(&malicious_input);
            prop_assert!(result.is_err() || !contains_shell_metacharacters(&result.unwrap()));
        }
    }

    #[test]
    fn test_path_traversal_prevention() {
        let traversal_attempts = [
            "../../../etc/passwd",
            "..\\..\\windows\\system32",
            "/etc/passwd",
            "~/../../etc/passwd",
        ];

        for attempt in &traversal_attempts {
            assert!(validate_repository_path(Path::new(attempt)).is_err());
        }
    }
}
```

### Security Test Categories

1. **Input Validation Tests**
   - Command injection attempts
   - Path traversal attempts
   - Buffer overflow attempts
   - Format string attacks

2. **Authentication Tests**
   - Credential handling
   - Session management
   - Permission validation

3. **Resource Limit Tests**
   - Memory usage limits
   - CPU usage limits
   - File size limits
   - Network timeout tests

### Penetration Testing

Regular security assessments include:

- Static code analysis
- Dynamic testing with fuzzing
- Dependency vulnerability scanning
- Configuration security review

## Incident Response

### Response Plan

1. **Detection**: Automated monitoring and user reports
2. **Assessment**: Severity evaluation and impact analysis
3. **Containment**: Immediate containment measures
4. **Investigation**: Root cause analysis
5. **Resolution**: Fix development and deployment
6. **Recovery**: Service restoration and monitoring
7. **Lessons Learned**: Post-incident review

### Severity Levels

- **Critical**: Immediate risk to user systems or data
- **High**: Significant security impact
- **Medium**: Moderate security impact
- **Low**: Minor security impact

### Communication

- **Public Disclosure**: After fix is available
- **User Notification**: Security advisories for users
- **Vendor Coordination**: With downstream distributors

## Security Hardening

### Development Environment

```bash
# Secure development setup
cargo install cargo-audit
cargo install cargo-deny
pre-commit install

# Regular security checks
cargo audit
cargo deny check
cargo clippy -- -D warnings
```

### Production Deployment

- **Minimal Attack Surface**: Disable unnecessary features
- **Resource Limits**: Configure appropriate limits
- **Logging**: Enable security event logging
- **Updates**: Regular security updates

### Configuration Security

```toml
[security]
# Production security configuration
strict_mode = true
sandbox_mode = true
max_repo_size = 1073741824  # 1GB
allowed_extensions = ["rs", "toml", "md", "txt"]
log_security_events = true
```

## Compliance and Standards

### Security Standards

- **OWASP**: Follow OWASP secure coding practices
- **CWE**: Address Common Weakness Enumeration items
- **NIST**: Align with NIST Cybersecurity Framework

### Code Quality

- **Static Analysis**: Continuous static code analysis
- **Code Review**: Security-focused code reviews
- **Testing**: Comprehensive security testing
- **Documentation**: Security documentation and training

## Security Resources

### For Developers

- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [OWASP Secure Coding Practices](https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/)
- [Git Security Best Practices](https://git-scm.com/docs/gitnamespaces)

### For Users

- Keep gitk-rs updated to the latest version
- Use trusted repositories only
- Enable security features in configuration
- Report suspicious behavior immediately

### Security Tools

```bash
# Development security tools
cargo install cargo-audit      # Vulnerability scanning
cargo install cargo-deny       # Dependency validation
cargo install cargo-supply-chain  # Supply chain analysis
```

## Contact Information

- **Security Team**: [SECURITY EMAIL]
- **General Issues**: GitHub Issues (for non-security issues)
- **Documentation**: Security section in project documentation

---

**Last Updated**: [DATE]
**Next Review**: [DATE + 6 months]