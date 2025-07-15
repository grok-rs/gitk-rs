# gitk-rs

A modern, fast, and feature-rich Git repository browser written in Rust - the spiritual successor to the classic `gitk` tool.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![Release](https://img.shields.io/github/v/release/gitk-rs/gitk-rs.svg)](https://github.com/gitk-rs/gitk-rs/releases)

## Overview

gitk-rs is a complete rewrite of the beloved `gitk` Git repository visualization tool, built from the ground up in Rust with modern UI frameworks and enhanced functionality. While maintaining the familiar three-pane interface that developers know and love, gitk-rs adds powerful new features, improved performance, and enhanced security.

### Key Features

🚀 **Performance**
- Streaming commit loading for instant repository browsing
- Virtual scrolling for repositories with millions of commits
- Efficient memory usage and responsive UI

🎨 **Advanced Visualization**
- Interactive commit graph with zoom, pan, and highlighting
- Sophisticated branch coloring and merge visualization
- Side-by-side diff viewer with syntax highlighting
- Word-level diff detection using advanced algorithms

🔧 **Comprehensive Git Operations**
- Complete branch management (create, delete, checkout, merge)
- Tag management with annotation support
- Commit operations (cherry-pick, revert, reset)
- Stash management for temporary changes
- Remote operations (fetch, push, pull) with authentication

🛡️ **Security & Safety**
- Multi-layered input validation and sanitization
- Safe command execution with proper escaping
- Cross-platform security measures
- Comprehensive error handling and recovery

🔍 **Enhanced User Experience**
- Modern, responsive UI built with egui
- Resizable three-pane layout
- Comprehensive search and filtering
- Operation history tracking
- Keyboard shortcuts and menu system

## Screenshots

*[Screenshots would go here showing the main interface, diff viewer, and commit graph]*

## Installation

### Pre-built Binaries

Download the latest release for your platform from the [releases page](https://github.com/gitk-rs/gitk-rs/releases).

### From Source

#### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- Git (for development)

#### Build Instructions

```bash
# Clone the repository
git clone https://github.com/gitk-rs/gitk-rs.git
cd gitk-rs

# Build in release mode
cargo build --release

# The binary will be available at target/release/gitk-rs
```

#### Install from Cargo

```bash
cargo install gitk-rs
```

## Usage

### Basic Usage

```bash
# Open current repository
gitk-rs

# Open specific repository
gitk-rs /path/to/repository

# Open with specific commit selected
gitk-rs --select-commit abc123

# Show help
gitk-rs --help
```

### Navigation

- **Commit List**: Browse through commits chronologically
- **Commit Graph**: Interactive visualization of branch history
- **Diff Viewer**: Side-by-side comparison of changes

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+O` | Open repository |
| `Ctrl+R` | Refresh view |
| `Ctrl+F` | Search commits |
| `Ctrl+B` | Toggle branch view |
| `F5` | Refresh repository |
| `Space` | Quick diff view |
| `Enter` | View commit details |

### Git Operations

gitk-rs provides a comprehensive set of Git operations through the UI:

#### Branch Operations
- Create new branches from any commit
- Delete branches with safety checks
- Checkout branches
- Merge branches with conflict resolution

#### Tag Management
- Create lightweight or annotated tags
- Delete tags with confirmation
- View tag details and history

#### Commit Operations
- Cherry-pick commits across branches
- Revert commits with automatic commit creation
- Reset branches to specific commits
- View detailed commit information

#### Stash Management
- Create stashes with custom messages
- Apply, pop, or drop stashes
- View stash contents and metadata

#### Remote Operations
- Fetch from remotes with progress tracking
- Push to remotes with authentication
- Pull with merge or rebase options
- Manage remote configurations

## Configuration

gitk-rs stores its configuration in platform-specific directories:

- **Linux**: `~/.config/gitk-rs/`
- **macOS**: `~/Library/Application Support/gitk-rs/`
- **Windows**: `%APPDATA%/gitk-rs/`

### Configuration Files

- `config.json`: Application settings
- `layout.json`: UI layout preferences
- `themes.json`: Color themes and styling

### Example Configuration

```json
{
  "ui": {
    "theme": "dark",
    "font_size": 12,
    "show_merge_commits": true,
    "max_commits_loaded": 10000
  },
  "git": {
    "default_remote": "origin",
    "auto_fetch": true,
    "fetch_interval_minutes": 15
  },
  "diff": {
    "syntax_highlighting": true,
    "word_wrap": false,
    "context_lines": 3
  }
}
```

## Architecture

gitk-rs is built with a modular architecture emphasizing safety, performance, and maintainability:

### Core Components

- **Git Backend**: Safe wrapper around libgit2 with comprehensive error handling
- **UI Framework**: Modern immediate-mode GUI using egui
- **Security Layer**: Multi-level input validation and command sanitization
- **Operation Manager**: Unified interface for all Git operations with history tracking
- **View System**: Flexible filtering and presentation of repository data

### Security Features

- **Input Validation**: All user inputs are validated and sanitized
- **Command Sanitization**: Git commands are safely constructed and executed
- **Path Traversal Protection**: Prevents access outside repository boundaries
- **Cross-platform Security**: Platform-specific security measures for Windows, macOS, and Linux

## Development

### Development Setup

```bash
# Clone the repository
git clone https://github.com/gitk-rs/gitk-rs.git
cd gitk-rs

# Run in development mode
cargo run

# Run tests
cargo test

# Check code formatting
cargo fmt --check

# Run linter
cargo clippy
```

### Project Structure

```
src/
├── main.rs              # Application entry point
├── app.rs               # Main application logic
├── config/              # Configuration management
├── git/                 # Git operations and repository handling
│   ├── repository.rs    # Repository abstraction
│   ├── operations.rs    # Unified Git operations manager
│   ├── security.rs      # Security and validation
│   ├── remotes.rs       # Remote operations
│   ├── tags.rs          # Tag management
│   ├── commits.rs       # Commit operations
│   └── stash.rs         # Stash management
├── ui/                  # User interface components
│   ├── main_window.rs   # Main application window
│   ├── graph.rs         # Commit graph visualization
│   ├── diff_viewer.rs   # Diff viewing component
│   └── views.rs         # Repository views and filtering
├── models/              # Data models and structures
└── state/               # Application state management
```

### Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

### Code Style

- Follow Rust naming conventions
- Use `cargo fmt` for formatting
- Address all `clippy` warnings
- Write tests for new functionality
- Document public APIs

## Comparison with Original gitk

| Feature | Original gitk | gitk-rs |
|---------|---------------|---------|
| **Language** | Tcl/Tk | Rust |
| **Performance** | Limited by Tcl | High-performance native code |
| **Memory Usage** | High for large repos | Efficient streaming |
| **UI Framework** | Tk (outdated) | egui (modern) |
| **Git Operations** | Basic viewing | Full Git operations suite |
| **Security** | Limited validation | Comprehensive security layers |
| **Cross-platform** | Basic support | Native platform integration |
| **Extensibility** | Limited | Modular Rust architecture |
| **Syntax Highlighting** | None | Full syntax highlighting |
| **Search & Filter** | Basic | Advanced filtering system |

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

- Original `gitk` by Paul Mackerras for inspiring this project
- The Rust community for excellent libraries and tools
- Contributors and users who make this project possible

## Roadmap

### Version 1.0 Goals
- ✅ Complete Git operations suite
- ✅ Advanced visualization features
- ✅ Security and safety improvements
- 🔄 Comprehensive documentation
- 🔄 Platform-specific installers
- 🔄 Plugin system architecture

### Future Enhancements
- 📋 Plugin system for extensibility
- 📋 Advanced merge conflict resolution UI
- 📋 Integration with popular Git hosting services
- 📋 Collaborative features for team workflows
- 📋 Performance optimizations for massive repositories
- 📋 AI-powered commit message suggestions

## Support

- **Issues**: [GitHub Issues](https://github.com/gitk-rs/gitk-rs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/gitk-rs/gitk-rs/discussions)
- **Documentation**: [Wiki](https://github.com/gitk-rs/gitk-rs/wiki)

---

*gitk-rs: Bringing Git repository browsing into the modern age* 🦀