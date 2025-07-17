#![allow(clippy::all)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

//! UI rendering performance benchmarks
//!
//! This file contains benchmarks for UI rendering performance,
//! particularly for the commit graph and diff viewer components.

use chrono::{DateTime, Utc};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use egui::Ui;
use gitk_rs::{
    models::GitCommit,
    state::AppState,
    ui::{CommitGraph, DiffViewer},
};

/// Create mock commits for UI testing
fn create_mock_commits(count: usize) -> Vec<GitCommit> {
    let mut commits = Vec::new();
    let base_time = Utc::now();

    for i in 0..count {
        commits.push(GitCommit {
            id: format!("commit_{:08x}", i),
            short_id: format!("{:07x}", i),
            author: gitk_rs::models::GitSignature {
                name: format!("Author {}", i % 10),
                email: format!("author{}@example.com", i % 10),
                when: base_time - chrono::Duration::seconds(i as i64 * 3600),
            },
            committer: gitk_rs::models::GitSignature {
                name: format!("Committer {}", i % 5),
                email: format!("committer{}@example.com", i % 5),
                when: base_time - chrono::Duration::seconds(i as i64 * 3600 - 300),
            },
            message: format!("Commit message for commit {}\n\nThis is a longer description that explains what this commit does. It might span multiple lines and contain various details about the changes made.", i),
            summary: format!("Commit message for commit {}", i),
            parent_ids: if i == 0 {
                vec![]
            } else {
                vec![format!("commit_{:08x}", i - 1)]
            },
            tree_id: format!("tree_{:08x}", i),
        });
    }

    commits
}

/// Benchmark commit graph layout computation
fn bench_commit_graph_layout(c: &mut Criterion) {
    let mut group = c.benchmark_group("commit_graph_layout");

    for commit_count in [50, 100, 500, 1000].iter() {
        let commits = create_mock_commits(*commit_count);
        let mut graph = CommitGraph::new();

        group.throughput(Throughput::Elements(*commit_count as u64));
        group.bench_with_input(
            BenchmarkId::new("commits", commit_count),
            &commits,
            |b, commits| {
                b.iter(|| {
                    // Simulate layout computation
                    let _layout = graph
                        .compute_graph_layout(black_box(commits), egui::Vec2::new(800.0, 600.0));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark commit graph rendering performance
fn bench_commit_graph_rendering(c: &mut Criterion) {
    let commits = create_mock_commits(200);
    let mut graph = CommitGraph::new();
    let state = AppState::default();

    // Note: This is a simplified benchmark since we can't easily create a real egui::Ui in tests
    // In a real scenario, this would involve actual rendering operations

    c.bench_function("commit_graph_rendering", |b| {
        b.iter(|| {
            // Simulate the work done in graph rendering
            for commit in black_box(&commits) {
                black_box(&commit.id);
                black_box(&commit.summary);
                black_box(&commit.author.name);
            }
        });
    });
}

/// Benchmark diff viewer operations
fn bench_diff_viewer_operations(c: &mut Criterion) {
    let mut diff_viewer = DiffViewer::new();

    c.bench_function("diff_viewer_initialization", |b| {
        b.iter(|| {
            let _viewer = DiffViewer::new();
        });
    });

    // Test various diff viewer state operations
    c.bench_function("diff_viewer_state_operations", |b| {
        b.iter(|| {
            diff_viewer.set_show_line_numbers(black_box(true));
            diff_viewer.set_font_size(black_box(14.0));
            diff_viewer.set_word_wrap(black_box(false));
        });
    });
}

/// Benchmark large commit set filtering and search
fn bench_commit_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("commit_filtering");

    for commit_count in [100, 500, 1000, 2000].iter() {
        let commits = create_mock_commits(*commit_count);

        group.throughput(Throughput::Elements(*commit_count as u64));
        group.bench_with_input(
            BenchmarkId::new("commits", commit_count),
            &commits,
            |b, commits| {
                b.iter(|| {
                    // Simulate commit filtering by author
                    let _filtered: Vec<_> = commits
                        .iter()
                        .filter(|commit| black_box(&commit.author.name).contains("Author 1"))
                        .collect();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark commit search operations
fn bench_commit_search(c: &mut Criterion) {
    let commits = create_mock_commits(1000);

    c.bench_function("commit_message_search", |b| {
        b.iter(|| {
            let search_term = black_box("commit 123");
            let _results: Vec<_> = commits
                .iter()
                .filter(|commit| commit.message.contains(search_term))
                .collect();
        });
    });

    c.bench_function("commit_author_search", |b| {
        b.iter(|| {
            let search_author = black_box("Author 5");
            let _results: Vec<_> = commits
                .iter()
                .filter(|commit| commit.author.name.contains(search_author))
                .collect();
        });
    });
}

/// Benchmark virtual scrolling simulation
fn bench_virtual_scrolling(c: &mut Criterion) {
    let commits = create_mock_commits(10000);

    c.bench_function("virtual_scrolling_window", |b| {
        b.iter(|| {
            let viewport_start = black_box(500);
            let viewport_size = black_box(50);

            // Simulate getting visible commits for virtual scrolling
            let _visible_commits: Vec<_> = commits
                .iter()
                .skip(viewport_start)
                .take(viewport_size)
                .collect();
        });
    });
}

criterion_group!(
    benches,
    bench_commit_graph_layout,
    bench_commit_graph_rendering,
    bench_diff_viewer_operations,
    bench_commit_filtering,
    bench_commit_search,
    bench_virtual_scrolling
);
criterion_main!(benches);
