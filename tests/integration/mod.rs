//! Integration tests for gitk-rs
//!
//! This module contains comprehensive integration tests that verify the entire application
//! works correctly with real Git repositories and UI interactions.

use gitk_rs::git::GitRepository;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test utilities for integration tests
pub mod test_utils {
    use super::*;
    use std::process::Command;

    /// Create a temporary Git repository for testing
    pub fn create_test_repo() -> anyhow::Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize Git repository
        Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()?;

        // Configure Git user for commits
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()?;

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()?;

        Ok((temp_dir, repo_path))
    }

    /// Create a test commit in the repository
    pub fn create_test_commit(repo_path: &std::path::Path, message: &str) -> anyhow::Result<()> {
        // Create a test file
        let test_file = repo_path.join("test.txt");
        std::fs::write(&test_file, format!("Test content for {}", message))?;

        // Add and commit
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(repo_path)
            .output()?;

        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(repo_path)
            .output()?;

        Ok(())
    }

    /// Create a test repository with multiple commits and branches
    pub fn create_complex_test_repo() -> anyhow::Result<(TempDir, PathBuf)> {
        let (temp_dir, repo_path) = create_test_repo()?;

        // Create initial commit
        create_test_commit(&repo_path, "Initial commit")?;

        // Create a feature branch
        Command::new("git")
            .args(["checkout", "-b", "feature/test"])
            .current_dir(&repo_path)
            .output()?;

        // Add commits to feature branch
        create_test_commit(&repo_path, "Add feature functionality")?;
        create_test_commit(&repo_path, "Fix feature bug")?;

        // Switch back to main and add another commit
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(&repo_path)
            .output()?;

        create_test_commit(&repo_path, "Main branch update")?;

        // Merge feature branch
        Command::new("git")
            .args(["merge", "feature/test", "--no-ff", "-m", "Merge feature branch"])
            .current_dir(&repo_path)
            .output()?;

        Ok((temp_dir, repo_path))
    }
}

/// Basic repository operations integration tests
#[cfg(test)]
mod repository_tests {
    use super::*;
    use super::test_utils::*;

    #[test]
    fn test_repository_discovery() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "Test commit")?;

        let repo = GitRepository::discover(&repo_path)?;
        assert!(repo.path().exists());

        Ok(())
    }

    #[test]
    fn test_commit_loading() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_complex_test_repo()?;

        let repo = GitRepository::discover(&repo_path)?;
        let commits = repo.get_commits(Some(10))?;

        assert!(!commits.is_empty());
        assert!(commits.len() >= 5); // Should have at least 5 commits from our setup

        Ok(())
    }

    #[test]
    fn test_branch_operations() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_complex_test_repo()?;

        let repo = GitRepository::discover(&repo_path)?;
        let branches = repo.get_branches()?;

        assert!(branches.contains(&"main".to_string()));
        assert!(branches.contains(&"feature/test".to_string()));

        Ok(())
    }
}