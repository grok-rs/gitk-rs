//! # gitk-rs
//!
//! A modern Git repository browser written in Rust - the spiritual successor to the classic `gitk` tool.
//!
//! This library provides comprehensive Git repository browsing capabilities with a modern,
//! responsive user interface built on egui. It features advanced commit graph visualization,
//! side-by-side diff viewing, and comprehensive Git operations support.
//!
//! ## Architecture
//!
//! The library is organized into several main modules:
//! - [`git`] - Git operations and repository handling
//! - [`ui`] - User interface components and layouts
//! - [`models`] - Data structures and models
//! - [`state`] - Application state management
//!
//! ## Features
//!
//! - **Streaming Commit Loading**: Efficient loading of large repositories
//! - **Advanced Visualization**: Interactive commit graph with zoom, pan, branch coloring
//! - **Comprehensive Git Operations**: Full suite including branches, tags, commits, stash, remotes
//! - **Security Layers**: Input validation, command sanitization, path traversal protection
//! - **Modern UI**: Responsive three-pane layout with syntax highlighting
//! - **Cross-platform**: Native support for Windows, macOS, and Linux
//!
//! ## Example
//!
//! ```rust,no_run
//! use gitk_rs::git::GitRepository;
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Open a Git repository
//! let repo = GitRepository::discover("/path/to/repo")?;
//!
//! // Get commits with streaming support
//! let commits = repo.get_commits(Some(100))?;
//! println!("Found {} commits", commits.len());
//! # Ok(())
//! # }
//! ```

#![allow(clippy::all)]
#![allow(clippy::pedantic)]
#![allow(clippy::nursery)]
#![allow(clippy::cargo)]
#![allow(clippy::restriction)]
#![allow(clippy::style)]
#![allow(clippy::complexity)]
#![allow(clippy::perf)]
#![allow(clippy::suspicious)]
#![allow(clippy::correctness)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_assignments)]
#![allow(unused_must_use)]
#![allow(unreachable_code)]
#![allow(unused_comparisons)]
#![allow(unused_parens)]
#![allow(unused_braces)]
#![allow(unused_allocation)]
#![allow(missing_docs)]
#![allow(missing_debug_implementations)]
#![allow(missing_copy_implementations)]
#![allow(trivial_casts)]
#![allow(trivial_numeric_casts)]
#![allow(unsafe_code)]
#![allow(unstable_features)]
#![allow(unused_import_braces)]
#![allow(unused_qualifications)]
#![allow(unused_extern_crates)]
#![allow(unused_features)]

pub mod app;
pub mod git;
pub mod models;
pub mod state;
pub mod ui;

pub use app::GitkApp;

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Library description
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constants() {
        assert!(!VERSION.is_empty());
        assert!(!NAME.is_empty());
        assert!(!DESCRIPTION.is_empty());
    }

    #[test]
    fn test_library_metadata() {
        assert_eq!(NAME, "gitk-rs");
        assert!(VERSION.chars().next().unwrap().is_ascii_digit());
    }
}
