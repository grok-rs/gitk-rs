use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub id: String,
    pub short_id: String,
    pub author: GitSignature,
    pub committer: GitSignature,
    pub message: String,
    pub summary: String,
    pub parent_ids: Vec<String>,
    pub tree_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSignature {
    pub name: String,
    pub email: String,
    pub when: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct GitDiff {
    pub old_file: Option<String>,
    pub new_file: Option<String>,
    pub hunks: Vec<GitHunk>,
    pub stats: GitDiffStats,
    pub is_binary: bool,
    pub status: DiffStatus,
    pub similarity: Option<u32>, // For renames and copies
}

#[derive(Debug, Clone)]
pub struct GitHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<GitDiffLine>,
}

#[derive(Debug, Clone)]
pub struct GitDiffLine {
    pub origin: char,
    pub content: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct GitDiffStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiffStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    Ignored,
    Untracked,
    Typechange,
}

impl GitCommit {
    pub fn new(commit: &git2::Commit) -> anyhow::Result<Self> {
        let id = commit.id().to_string();
        let short_id = commit.as_object().short_id()?.as_str().unwrap_or("").to_string();
        
        let author = GitSignature {
            name: commit.author().name().unwrap_or("").to_string(),
            email: commit.author().email().unwrap_or("").to_string(),
            when: DateTime::from_timestamp(commit.author().when().seconds(), 0)
                .unwrap_or_else(|| Utc::now()),
        };

        let committer = GitSignature {
            name: commit.committer().name().unwrap_or("").to_string(),
            email: commit.committer().email().unwrap_or("").to_string(),
            when: DateTime::from_timestamp(commit.committer().when().seconds(), 0)
                .unwrap_or_else(|| Utc::now()),
        };

        let message = commit.message().unwrap_or("").to_string();
        let summary = commit.summary().unwrap_or("").to_string();
        
        let parent_ids = commit.parent_ids().map(|id| id.to_string()).collect();
        let tree_id = commit.tree_id().to_string();

        Ok(GitCommit {
            id,
            short_id,
            author,
            committer,
            message,
            summary,
            parent_ids,
            tree_id,
        })
    }
}