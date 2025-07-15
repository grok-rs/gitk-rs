# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is **gitk-rs**, a modern Git repository browser written in Rust - the spiritual successor to the classic `gitk` tool. This project is a complete rewrite that maintains the familiar three-pane interface while adding powerful new features, improved performance, and enhanced security.

## Build System and Commands

**Cargo-based build (Rust):**
```bash
# Development build
cargo build

# Release build  
cargo build --release

# Run the application
cargo run

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy

# Clean build artifacts
cargo clean
```

**Development commands:**
```bash
# Run with specific repository
cargo run -- /path/to/repository

# Run with debug logging
RUST_LOG=debug cargo run

# Run tests with output
cargo test -- --nocapture

# Check compilation without building
cargo check
```

## Architecture

### Core Components

- **src/main.rs**: Application entry point and initialization
- **src/app.rs**: Main application logic and state management
- **src/git/**: Git operations and repository handling
  - `repository.rs`: Core repository abstraction and Git command execution
  - `operations.rs`: Unified Git operations manager (branches, tags, commits, stash, remotes)
  - `security.rs`: Input validation, command sanitization, and security layers
  - `remotes.rs`: Remote repository operations (fetch, push, pull) with authentication
  - `tags.rs`: Tag management (create, delete, list, filter)
  - `commits.rs`: Commit operations (cherry-pick, revert, reset)
  - `stash.rs`: Stash management (create, apply, pop, drop)
- **src/ui/**: User interface components built with egui
  - `main_window.rs`: Main application window and layout
  - `graph.rs`: Interactive commit graph visualization with advanced rendering
  - `diff_viewer.rs`: Side-by-side diff viewer with syntax highlighting
  - `views.rs`: Repository views and filtering system
- **src/models/**: Data structures and models
- **src/state/**: Application state management
- **src/config/**: Configuration management

### Key Technical Details

- **Language**: Rust with modern async/await patterns
- **UI Framework**: egui (immediate mode GUI) for responsive, native performance
- **Git Backend**: libgit2 via git2-rs crate for safe Git operations
- **Security**: Multi-layered input validation and command sanitization
- **Performance**: Streaming commit loading, virtual scrolling, efficient memory usage
- **Cross-platform**: Native support for Windows, macOS, and Linux

## Development Notes

### Code Organization

- **Modular Architecture**: Clean separation between Git operations, UI components, and state management
- **Safety First**: Comprehensive error handling, input validation, and safe command execution
- **Modern Rust**: Uses current Rust idioms, async/await, and best practices
- **Testing**: Unit tests for Git operations and integration tests for UI components
- **Documentation**: Inline documentation and examples for all public APIs

### Key Features Implemented

1. **Streaming Commit Loading**: Efficient loading of large repositories
2. **Advanced Visualization**: Interactive commit graph with zoom, pan, branch coloring
3. **Comprehensive Git Operations**: Full suite including branches, tags, commits, stash, remotes
4. **Security Layers**: Input validation, command sanitization, path traversal protection
5. **Modern UI**: Responsive three-pane layout with syntax highlighting
6. **Operation History**: Complete audit trail of all Git operations
7. **Authentication**: SSH/HTTPS support for remote operations

### Dependencies

- **egui/eframe**: Modern immediate mode GUI framework
- **git2**: Safe Rust bindings for libgit2
- **tokio**: Async runtime for non-blocking operations
- **serde**: Serialization for configuration and state persistence
- **chrono**: Date/time handling for commit timestamps
- **tracing**: Structured logging and diagnostics
- **anyhow/thiserror**: Error handling and propagation

### Testing and Quality

- Unit tests for all Git operations
- Integration tests for UI components
- Property-based testing for critical paths
- Code coverage monitoring
- Continuous integration with multiple Rust versions
- Cross-platform testing on Windows, macOS, and Linux

## Configuration

The application stores configuration in platform-specific directories:
- **Linux**: `~/.config/gitk-rs/`
- **macOS**: `~/Library/Application Support/gitk-rs/`  
- **Windows**: `%APPDATA%/gitk-rs/`

Configuration files include:
- `config.json`: Application settings
- `layout.json`: UI layout preferences
- `themes.json`: Color themes and styling

## Migration Status

This project represents a complete migration from the original Tcl/Tk gitk to modern Rust:

### ✅ Completed Features

**Phase 1: Core Infrastructure & Security**
- ✅ Streaming commit loading system
- ✅ Comprehensive Git command support with safe wrappers  
- ✅ Enhanced diff processing for binary files, renames, and complex merges
- ✅ Reference management for branches, tags, and remotes
- ✅ Multi-view system with filtering capabilities
- ✅ Command sanitization and security layers
- ✅ Cross-platform security measures
- ✅ Comprehensive input validation and error handling

**Phase 2: Advanced UI & Visualization**
- ✅ Advanced commit graph rendering with branch layout algorithms
- ✅ Merge commit visualization with sophisticated branch coloring
- ✅ Interactive graph with zoom, pan, and path highlighting
- ✅ Comprehensive side-by-side diff viewer
- ✅ Syntax highlighting and word-level diff detection
- ✅ Resizable three-pane layout with proper UI components
- ✅ Comprehensive menu system and keyboard shortcuts

**Phase 3: Complete Git Operations Suite**
- ✅ Branch operations (create, delete, checkout, merge)
- ✅ Tag management with annotation features
- ✅ Commit operations (cherry-pick, revert, reset)
- ✅ Stash management for temporary changes
- ✅ Remote operations (fetch, pull, push) with authentication

### Architecture Improvements

The Rust implementation provides significant improvements over the original:

- **Performance**: Native code performance vs interpreted Tcl
- **Memory Safety**: Rust's ownership system prevents memory-related bugs
- **Security**: Multi-layered validation and sanitization
- **Maintainability**: Modular architecture with clear separation of concerns
- **Extensibility**: Plugin-ready architecture for future enhancements
- **Cross-platform**: Better platform integration and native look/feel

### Development Guidelines

When working on this codebase:

1. **Safety First**: Always validate inputs and handle errors gracefully
2. **Modular Design**: Keep Git operations, UI, and state management separate
3. **Testing**: Add tests for new functionality
4. **Documentation**: Update both inline docs and this file for major changes
5. **Performance**: Consider memory usage and UI responsiveness
6. **Security**: Review all user inputs and Git command construction

## Test-Driven Development (TDD) Practices

This project follows strict TDD practices to ensure code quality, maintainability, and reliability.

### TDD Workflow

1. **Red**: Write a failing test that describes the desired functionality
2. **Green**: Write the minimal code necessary to make the test pass
3. **Refactor**: Improve the code while keeping tests green

### Testing Strategy

#### Unit Tests
- **Location**: Each module contains `#[cfg(test)] mod tests`
- **Coverage**: Aim for >90% code coverage
- **Focus**: Test individual functions and methods in isolation
- **Tools**: Use `cargo test`, `proptest` for property-based testing

#### Integration Tests
- **Location**: `tests/integration/` directory
- **Coverage**: Test complete workflows and component interactions
- **Focus**: End-to-end functionality with real Git repositories
- **Tools**: Use `tempfile` for temporary repositories, `assert_cmd` for CLI testing

#### Property-Based Testing
- **Tool**: `proptest` for generating test inputs
- **Focus**: Test invariants and edge cases with random inputs
- **Example**: Input validation, data serialization roundtrips

#### Benchmarks
- **Location**: `benches/` directory
- **Tool**: `criterion` for performance benchmarks
- **Focus**: Critical performance paths (commit loading, graph rendering)
- **CI**: Automated performance regression detection

### Testing Commands

```bash
# Run all tests
cargo test

# Run tests with coverage
cargo tarpaulin --all-features --out html

# Run specific test module
cargo test git::repository::tests

# Run integration tests only
cargo test --test '*'

# Run benchmarks
cargo bench

# Run property-based tests
cargo test proptest

# Test with different features
cargo test --features testing
cargo test --no-default-features
```

### Code Quality Standards

#### Linting and Formatting
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Check documentation
cargo doc --no-deps --document-private-items
```

#### Security and Dependencies
```bash
# Security audit
cargo audit

# Check licenses and dependencies
cargo deny check

# Check for outdated dependencies
cargo outdated
```

### Test Guidelines

#### Writing Good Tests

1. **Descriptive Names**: Use clear, descriptive test function names
   ```rust
   #[test]
   fn test_repository_discovery_with_invalid_path_returns_error() {
       // Test implementation
   }
   ```

2. **Arrange-Act-Assert Pattern**:
   ```rust
   #[test]
   fn test_commit_parsing() {
       // Arrange
       let input = "test commit data";
       
       // Act
       let result = parse_commit(input);
       
       // Assert
       assert!(result.is_ok());
       assert_eq!(result.unwrap().message, "test commit data");
   }
   ```

3. **Use Helper Functions**: Create test utilities for common setup
   ```rust
   fn create_test_repo() -> TempDir { /* ... */ }
   fn create_test_commit(repo: &Path, msg: &str) { /* ... */ }
   ```

4. **Test Edge Cases**: Include boundary conditions and error paths
   ```rust
   #[test_case("", false; "empty string")]
   #[test_case("valid_sha", true; "valid SHA")]
   #[test_case("invalid!", false; "invalid characters")]
   fn test_sha_validation(input: &str, expected: bool) {
       assert_eq!(validate_sha(input).is_ok(), expected);
   }
   ```

5. **Property-Based Testing**: Use `proptest` for complex validation
   ```rust
   proptest! {
       #[test]
       fn test_commit_id_roundtrip(id in "[a-f0-9]{40}") {
           let parsed = parse_commit_id(&id).unwrap();
           prop_assert_eq!(format_commit_id(&parsed), id);
       }
   }
   ```

#### Testing Best Practices

- **Isolation**: Tests should not depend on each other
- **Deterministic**: Tests should produce consistent results
- **Fast**: Unit tests should run in milliseconds
- **Readable**: Tests serve as documentation
- **Maintainable**: Easy to update when requirements change

#### Mock and Stub Guidelines

```rust
#[cfg(feature = "testing")]
use mockall::predicate::*;

#[cfg_attr(feature = "testing", mockall::automock)]
trait GitRepository {
    fn get_commits(&self, limit: Option<usize>) -> Result<Vec<GitCommit>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_with_mock() {
        let mut mock = MockGitRepository::new();
        mock.expect_get_commits()
            .with(eq(Some(10)))
            .times(1)
            .returning(|_| Ok(vec![]));
            
        // Test with mock
    }
}
```

### Continuous Integration

#### Pre-commit Hooks
```bash
# Install pre-commit hooks
pre-commit install
pre-commit install --hook-type commit-msg

# Run hooks manually
pre-commit run --all-files
```

#### CI Pipeline
- **Format Check**: `cargo fmt --check`
- **Lint Check**: `cargo clippy -- -D warnings`
- **Test Suite**: `cargo test --all-features`
- **Coverage**: `cargo tarpaulin --all-features`
- **Security Audit**: `cargo audit`
- **Documentation**: `cargo doc --no-deps`

#### Quality Gates
- Minimum 80% code coverage
- Zero Clippy warnings
- All tests passing
- Security audit clean
- Documentation builds successfully

### Performance Testing

#### Benchmark Guidelines
```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_commit_loading(c: &mut Criterion) {
    c.bench_function("load_1000_commits", |b| {
        b.iter(|| {
            // Benchmark implementation
        });
    });
}

criterion_group!(benches, bench_commit_loading);
criterion_main!(benches);
```

#### Performance Targets
- Repository discovery: <100ms
- Commit loading (100 commits): <50ms
- Graph rendering: <16ms (60 FPS)
- Memory usage: <100MB for 10k commits

### Development Workflow

1. **Start with Tests**: Write failing tests first
2. **Implement**: Write minimal code to pass tests
3. **Refactor**: Improve code quality
4. **Document**: Add/update documentation
5. **Review**: Self-review changes
6. **Quality Check**: Run all quality checks
7. **Submit**: Create pull request

### Error Handling Standards

```rust
// Use thiserror for error types
#[derive(thiserror::Error, Debug)]
pub enum GitError {
    #[error("Repository not found: {path}")]
    RepositoryNotFound { path: String },
    
    #[error("Invalid commit SHA: {sha}")]
    InvalidCommitSha { sha: String },
}

// Use anyhow for error propagation
pub fn process_repository(path: &Path) -> anyhow::Result<()> {
    let repo = discover_repository(path)
        .context("Failed to discover repository")?;
    // ...
    Ok(())
}
```

### Documentation Standards

- All public APIs must have rustdoc comments
- Include examples in documentation
- Document error conditions
- Explain complex algorithms
- Keep CLAUDE.md updated with architectural changes