# Contributing to gitk-rs

Thank you for your interest in contributing to gitk-rs! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Development Setup](#development-setup)
- [Development Workflow](#development-workflow)
- [Testing Guidelines](#testing-guidelines)
- [Code Style and Standards](#code-style-and-standards)
- [Submitting Changes](#submitting-changes)
- [Issue Reporting](#issue-reporting)
- [Performance Considerations](#performance-considerations)
- [Security](#security)

## Code of Conduct

This project adheres to a code of conduct that we expect all contributors to follow. Please be respectful and constructive in all interactions.

## Development Setup

### Prerequisites

- **Rust**: Install the latest stable Rust toolchain via [rustup](https://rustup.rs/)
- **Git**: Git 2.20+ for repository operations
- **System Dependencies**:
  - Linux: `libgtk-3-dev libx11-dev libxrandr-dev libxcursor-dev libxi-dev libgl1-mesa-dev`
  - macOS: Xcode command line tools
  - Windows: Visual Studio Build Tools

### Initial Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/your-username/gitk-rs.git
   cd gitk-rs
   ```

2. **Install Development Tools**
   ```bash
   # Install pre-commit hooks
   pip install pre-commit
   pre-commit install
   
   # Install additional Rust tools
   cargo install cargo-audit cargo-deny cargo-tarpaulin
   ```

3. **Build and Test**
   ```bash
   cargo build
   cargo test
   ```

## Development Workflow

### Test-Driven Development (TDD)

We follow strict TDD practices:

1. **Red**: Write a failing test first
2. **Green**: Write minimal code to make the test pass
3. **Refactor**: Improve code while keeping tests green

### Branch Strategy

- `main`: Stable, production-ready code
- `develop`: Integration branch for features
- `feature/*`: Individual feature branches
- `bugfix/*`: Bug fix branches
- `hotfix/*`: Critical production fixes

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes
- `refactor`: Code refactoring
- `test`: Test additions/modifications
- `chore`: Maintenance tasks

**Examples:**
```
feat(ui): add dark mode toggle to settings panel

fix(git): resolve memory leak in commit loading

docs(api): update repository operations documentation
```

## Testing Guidelines

### Test Categories

1. **Unit Tests** (`src/`)
   - Test individual functions and methods
   - Mock external dependencies
   - Aim for 100% code coverage

2. **Integration Tests** (`tests/`)
   - Test component interactions
   - Use real Git repositories
   - Test complete workflows

3. **Property-Based Tests**
   - Use `proptest` for testing invariants
   - Generate random inputs
   - Verify properties hold across input space

4. **Benchmarks** (`benches/`)
   - Performance regression tests
   - Use `criterion` for statistical analysis
   - Test with realistic data sizes

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    #[test]
    fn test_basic_functionality() {
        // Arrange
        let input = create_test_input();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected_output);
    }
    
    proptest! {
        #[test]
        fn test_property_holds(input in any::<String>()) {
            let result = sanitize_input(&input);
            prop_assert!(result.is_valid());
        }
    }
}
```

### Test Data Management

- Use `insta` for snapshot testing
- Create helper functions for test data
- Clean up temporary files and repositories
- Mock external dependencies appropriately

## Code Style and Standards

### Rust Conventions

- **Formatting**: Use `cargo fmt` with project `.rustfmt.toml`
- **Linting**: Address all `cargo clippy` warnings
- **Documentation**: Document all public APIs with examples
- **Error Handling**: Use `anyhow` for applications, `thiserror` for libraries

### Code Organization

```
src/
├── main.rs          # Application entry point
├── lib.rs           # Library entry point
├── app.rs           # Main application logic
├── git/             # Git operations
│   ├── mod.rs
│   ├── repository.rs
│   ├── operations.rs
│   └── security.rs
├── ui/              # User interface
│   ├── mod.rs
│   ├── main_window.rs
│   └── components/
└── models/          # Data structures
```

### Performance Guidelines

- Prefer `&str` over `String` for temporary data
- Use `Arc<T>` for shared immutable data
- Implement `Clone` judiciously
- Profile before optimizing
- Use `cargo bench` for performance testing

### Security Practices

- Validate all user inputs
- Sanitize shell command arguments
- Use secure random number generation
- Avoid logging sensitive information
- Review security implications of dependencies

## Submitting Changes

### Pull Request Process

1. **Create Feature Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Implement Changes**
   - Follow TDD practices
   - Write comprehensive tests
   - Update documentation

3. **Quality Checks**
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features
   cargo test --all-features
   cargo audit
   ```

4. **Submit Pull Request**
   - Use descriptive title and description
   - Reference related issues
   - Include test evidence
   - Request appropriate reviewers

### PR Template

```markdown
## Summary
Brief description of changes

## Changes
- [ ] Feature/bug fix implemented
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] Benchmarks added (if performance-related)

## Testing
- [ ] All tests pass
- [ ] Manual testing performed
- [ ] Performance impact assessed

## Security Considerations
- [ ] Input validation reviewed
- [ ] No sensitive data logged
- [ ] Dependencies audited
```

### Review Process

- All PRs require review from at least one maintainer
- CI checks must pass
- Code coverage must not decrease
- Security implications must be addressed

## Issue Reporting

### Bug Reports

Include:
- **Environment**: OS, Rust version, gitk-rs version
- **Steps to Reproduce**: Minimal example
- **Expected vs Actual Behavior**
- **Logs/Error Messages**
- **Git Repository State** (if relevant)

### Feature Requests

Include:
- **Use Case**: Why is this needed?
- **Proposed Solution**: How should it work?
- **Alternatives Considered**
- **Implementation Notes** (if applicable)

### Security Issues

**Do not open public issues for security vulnerabilities.**

Email security issues to: [security contact]

Include:
- Detailed vulnerability description
- Steps to reproduce
- Potential impact
- Suggested fixes (if any)

## Performance Considerations

### Optimization Guidelines

- **Profile First**: Use `cargo bench` and profiling tools
- **Async Operations**: Use `tokio` for I/O-bound tasks
- **Memory Management**: Monitor allocation patterns
- **UI Responsiveness**: Keep UI thread unblocked

### Performance Targets

- Repository loading: < 500ms for 10k commits
- UI responsiveness: < 16ms frame time (60 FPS)
- Memory usage: < 100MB for typical repositories
- Cold start time: < 2 seconds

### Benchmarking

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench git_operations

# Generate performance reports
cargo bench -- --save-baseline main
```

## Development Tools

### Recommended VS Code Extensions

- `rust-analyzer`: Rust language support
- `Even Better TOML`: TOML file support
- `GitLens`: Enhanced Git integration
- `Test Explorer UI`: Test management

### Debugging

```bash
# Debug build with symbols
cargo build --profile dev

# Run with debug logging
RUST_LOG=debug cargo run

# Memory profiling
cargo install cargo-profdata
cargo profdata -- target/debug/gitk-rs
```

### IDE Configuration

**.vscode/settings.json**
```json
{
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.checkOnSave.command": "clippy",
    "files.watcherExclude": {
        "**/target/**": true
    }
}
```

## Release Process

### Version Management

- Follow [Semantic Versioning](https://semver.org/)
- Update `Cargo.toml` version
- Create release notes
- Tag releases: `v1.2.3`

### Release Checklist

- [ ] All tests pass on all platforms
- [ ] Documentation updated
- [ ] Changelog updated
- [ ] Version bumped appropriately
- [ ] Security audit clean
- [ ] Performance regression tests pass

## Getting Help

- **Discussions**: GitHub Discussions for questions
- **Issues**: GitHub Issues for bugs and features
- **Chat**: [Development chat channel]
- **Documentation**: Check existing docs first

## Recognition

Contributors are recognized in:
- Release notes
- `CONTRIBUTORS.md` file
- Project documentation

Thank you for contributing to gitk-rs!