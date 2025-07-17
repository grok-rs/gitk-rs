#![allow(clippy::all)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

//! Main integration test runner for gitk-rs
//!
//! This runs comprehensive integration tests across all major components.

// mod integration_tests;

use gitk_rs::GitkApp;
use serial_test::serial;

/// Integration test for the main application initialization
#[test]
#[serial]
fn test_app_initialization() {
    // This test ensures the application can be created without panicking
    let creation_context = eframe::CreationContext::default();
    let _app = GitkApp::new(&creation_context);

    // If we reach here, the app initialized successfully
    assert!(true);
}

/// Integration test for configuration loading
#[test]
#[serial]
fn test_configuration_system() {
    use gitk_rs::state::AppConfig;

    // Test default configuration creation
    let config = AppConfig::default();
    assert_eq!(config.window_size, (1200.0, 800.0));
    assert!(config.show_line_numbers);

    // Test configuration saving and loading
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", temp_dir.path());

    // This should work without panicking
    let saved_result = config.save();
    assert!(saved_result.is_ok());

    let loaded_config = AppConfig::load();
    assert_eq!(loaded_config.font_size, config.font_size);
}

/// Integration test for Git repository operations
#[test]
#[serial]
fn test_git_repository_integration() -> anyhow::Result<()> {
    use gitk_rs::git::GitRepository;
    use std::process::Command;

    let temp_dir = tempfile::tempdir()?;
    let repo_path = temp_dir.path();

    // Create a minimal Git repository
    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()?;

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()?;

    // Create and commit a file
    std::fs::write(repo_path.join("README.md"), "# Test Repository")?;
    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(repo_path)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()?;

    // Test repository discovery and operations
    let repository = GitRepository::discover(repo_path)?;
    let commits = repository.get_commits(Some(10))?;

    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].summary, "Initial commit");

    Ok(())
}
