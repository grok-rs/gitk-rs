//! Benchmarks for commit streaming performance
//!
//! This file benchmarks the commit streaming functionality which is critical
//! for performance with large repositories.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use gitk_rs::git::{CommitStream, GitRepository};
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Create a repository with many commits for streaming tests
fn create_streaming_test_repo(commit_count: usize) -> anyhow::Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize Git repository
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()?;

    // Configure Git user
    Command::new("git")
        .args(["config", "user.name", "Stream Benchmark"])
        .current_dir(&repo_path)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "stream@example.com"])
        .current_dir(&repo_path)
        .output()?;

    // Create many commits with different file sizes
    for i in 0..commit_count {
        let content = "x".repeat((i % 1000) + 100); // Variable content size
        let filename = format!("file_{}.txt", i);

        std::fs::write(repo_path.join(&filename), content)?;

        Command::new("git")
            .args(["add", &filename])
            .current_dir(&repo_path)
            .output()?;

        Command::new("git")
            .args(["commit", "-m", &format!("Streaming commit {}", i)])
            .current_dir(&repo_path)
            .output()?;
    }

    Ok((temp_dir, repo_path))
}

/// Benchmark commit stream initialization
fn bench_stream_initialization(c: &mut Criterion) {
    let (_temp_dir, repo_path) = create_streaming_test_repo(1000).unwrap();
    let repo = GitRepository::discover(&repo_path).unwrap();

    c.bench_function("stream_initialization", |b| {
        b.iter(|| {
            let _stream = CommitStream::new(black_box(&repo), black_box(100), black_box(50));
        });
    });
}

/// Benchmark commit streaming with different batch sizes
fn bench_commit_streaming_batches(c: &mut Criterion) {
    let mut group = c.benchmark_group("commit_streaming_batches");

    let (_temp_dir, repo_path) = create_streaming_test_repo(1000).unwrap();
    let repo = GitRepository::discover(&repo_path).unwrap();

    for batch_size in [10, 25, 50, 100].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_size", batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let mut stream = CommitStream::new(&repo, 500, batch_size);
                    let _commits = stream.next_batch().unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark streaming vs traditional loading
fn bench_streaming_vs_traditional(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_vs_traditional");

    let (_temp_dir, repo_path) = create_streaming_test_repo(500).unwrap();
    let repo = GitRepository::discover(&repo_path).unwrap();

    // Traditional loading
    group.bench_function("traditional_loading", |b| {
        b.iter(|| {
            let _commits = repo.get_commits(black_box(Some(100))).unwrap();
        });
    });

    // Streaming loading
    group.bench_function("streaming_loading", |b| {
        b.iter(|| {
            let mut stream = CommitStream::new(&repo, 100, 25);
            let mut total_commits = 0;

            while !stream.is_complete() {
                if let Ok(commits) = stream.next_batch() {
                    total_commits += commits.len();
                    if total_commits >= 100 {
                        break;
                    }
                } else {
                    break;
                }
            }
        });
    });

    group.finish();
}

/// Benchmark memory usage patterns in streaming
fn bench_streaming_memory_efficiency(c: &mut Criterion) {
    let (_temp_dir, repo_path) = create_streaming_test_repo(2000).unwrap();
    let repo = GitRepository::discover(&repo_path).unwrap();

    c.bench_function("memory_efficient_streaming", |b| {
        b.iter(|| {
            let mut stream = CommitStream::new(&repo, 1000, 20);
            let mut processed = 0;

            // Process commits in small batches to test memory efficiency
            while !stream.is_complete() && processed < 200 {
                if let Ok(commits) = stream.next_batch() {
                    // Simulate processing each commit
                    for commit in commits {
                        black_box(&commit.id);
                        processed += 1;
                    }
                } else {
                    break;
                }
            }
        });
    });
}

criterion_group!(
    benches,
    bench_stream_initialization,
    bench_commit_streaming_batches,
    bench_streaming_vs_traditional,
    bench_streaming_memory_efficiency
);
criterion_main!(benches);
