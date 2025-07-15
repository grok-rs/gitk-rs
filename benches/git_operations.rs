//! Benchmarks for Git operations
//!
//! This file contains performance benchmarks for core Git operations
//! to ensure optimal performance with large repositories.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use gitk_rs::git::GitRepository;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Create a test repository with specified number of commits
fn create_large_test_repo(commit_count: usize) -> anyhow::Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize Git repository
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()?;

    // Configure Git user
    Command::new("git")
        .args(["config", "user.name", "Benchmark User"])
        .current_dir(&repo_path)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "benchmark@example.com"])
        .current_dir(&repo_path)
        .output()?;

    // Create commits
    for i in 0..commit_count {
        let content = format!("Content for commit {}", i);
        let filename = format!("file_{}.txt", i);
        
        std::fs::write(repo_path.join(&filename), content)?;
        
        Command::new("git")
            .args(["add", &filename])
            .current_dir(&repo_path)
            .output()?;

        Command::new("git")
            .args(["commit", "-m", &format!("Commit {}", i)])
            .current_dir(&repo_path)
            .output()?;
    }

    Ok((temp_dir, repo_path))
}

/// Benchmark repository discovery performance
fn bench_repository_discovery(c: &mut Criterion) {
    let (_temp_dir, repo_path) = create_large_test_repo(100).unwrap();

    c.bench_function("repository_discovery", |b| {
        b.iter(|| {
            let _repo = GitRepository::discover(black_box(&repo_path)).unwrap();
        });
    });
}

/// Benchmark commit loading performance with different repository sizes
fn bench_commit_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("commit_loading");
    
    for commit_count in [10, 50, 100, 500].iter() {
        let (_temp_dir, repo_path) = create_large_test_repo(*commit_count).unwrap();
        let repo = GitRepository::discover(&repo_path).unwrap();
        
        group.throughput(Throughput::Elements(*commit_count as u64));
        group.bench_with_input(
            BenchmarkId::new("commits", commit_count),
            commit_count,
            |b, &commit_count| {
                b.iter(|| {
                    let _commits = repo.get_commits(black_box(Some(commit_count))).unwrap();
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark branch operations
fn bench_branch_operations(c: &mut Criterion) {
    let (_temp_dir, repo_path) = create_large_test_repo(10).unwrap();
    
    // Create some branches
    for i in 0..5 {
        Command::new("git")
            .args(["checkout", "-b", &format!("feature/branch-{}", i)])
            .current_dir(&repo_path)
            .output()
            .unwrap();
            
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
    }
    
    let repo = GitRepository::discover(&repo_path).unwrap();

    c.bench_function("get_branches", |b| {
        b.iter(|| {
            let _branches = repo.get_branches().unwrap();
        });
    });
}

/// Benchmark diff operations
fn bench_diff_operations(c: &mut Criterion) {
    let (_temp_dir, repo_path) = create_large_test_repo(5).unwrap();
    let repo = GitRepository::discover(&repo_path).unwrap();
    let commits = repo.get_commits(Some(5)).unwrap();
    
    if !commits.is_empty() {
        let commit_id = &commits[0].id;
        
        c.bench_function("get_commit_diff", |b| {
            b.iter(|| {
                let _diff = repo.get_commit_diff_enhanced(black_box(commit_id)).unwrap();
            });
        });
    }
}

criterion_group!(
    benches,
    bench_repository_discovery,
    bench_commit_loading,
    bench_branch_operations,
    bench_diff_operations
);
criterion_main!(benches);