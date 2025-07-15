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
    pub header: String,
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
        let short_id = commit
            .as_object()
            .short_id()?
            .as_str()
            .unwrap_or("")
            .to_string();

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_git_commit_creation() {
        let author = GitSignature {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            when: Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap(),
        };

        let committer = GitSignature {
            name: "Jane Smith".to_string(),
            email: "jane@example.com".to_string(),
            when: Utc.with_ymd_and_hms(2023, 1, 1, 12, 30, 0).unwrap(),
        };

        let commit = GitCommit {
            id: "abc123def456".to_string(),
            short_id: "abc123d".to_string(),
            author,
            committer,
            message: "Initial commit\n\nThis is the first commit".to_string(),
            summary: "Initial commit".to_string(),
            parent_ids: vec![],
            tree_id: "tree123".to_string(),
        };

        assert_eq!(commit.id, "abc123def456");
        assert_eq!(commit.short_id, "abc123d");
        assert_eq!(commit.author.name, "John Doe");
        assert_eq!(commit.committer.name, "Jane Smith");
        assert_eq!(commit.summary, "Initial commit");
        assert!(commit.parent_ids.is_empty());
    }

    #[test]
    fn test_git_signature_creation() {
        let signature = GitSignature {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            when: Utc::now(),
        };

        assert_eq!(signature.name, "Test User");
        assert_eq!(signature.email, "test@example.com");
        assert!(signature.when <= Utc::now());
    }

    #[test]
    fn test_git_diff_creation() {
        let hunk = GitHunk {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 4,
            header: "@@ -1,3 +1,4 @@".to_string(),
            lines: vec![
                GitDiffLine {
                    origin: ' ',
                    content: "line 1".to_string(),
                    old_lineno: Some(1),
                    new_lineno: Some(1),
                },
                GitDiffLine {
                    origin: '+',
                    content: "new line".to_string(),
                    old_lineno: None,
                    new_lineno: Some(2),
                },
            ],
        };

        let stats = GitDiffStats {
            files_changed: 1,
            insertions: 1,
            deletions: 0,
        };

        let diff = GitDiff {
            old_file: Some("file.txt".to_string()),
            new_file: Some("file.txt".to_string()),
            hunks: vec![hunk],
            stats,
            is_binary: false,
            status: DiffStatus::Modified,
            similarity: None,
        };

        assert_eq!(diff.old_file, Some("file.txt".to_string()));
        assert_eq!(diff.new_file, Some("file.txt".to_string()));
        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.stats.insertions, 1);
        assert_eq!(diff.stats.deletions, 0);
        assert!(!diff.is_binary);
        assert_eq!(diff.status, DiffStatus::Modified);
    }

    #[test]
    fn test_git_diff_line_creation() {
        let line = GitDiffLine {
            origin: '+',
            content: "added line".to_string(),
            old_lineno: None,
            new_lineno: Some(10),
        };

        assert_eq!(line.origin, '+');
        assert_eq!(line.content, "added line");
        assert_eq!(line.old_lineno, None);
        assert_eq!(line.new_lineno, Some(10));
    }

    #[test]
    fn test_diff_status_variants() {
        let statuses = vec![
            DiffStatus::Added,
            DiffStatus::Deleted,
            DiffStatus::Modified,
            DiffStatus::Renamed,
            DiffStatus::Copied,
            DiffStatus::Ignored,
            DiffStatus::Untracked,
            DiffStatus::Typechange,
        ];

        // Test that all variants can be created
        for status in statuses {
            match status {
                DiffStatus::Added => assert_eq!(status, DiffStatus::Added),
                DiffStatus::Deleted => assert_eq!(status, DiffStatus::Deleted),
                DiffStatus::Modified => assert_eq!(status, DiffStatus::Modified),
                DiffStatus::Renamed => assert_eq!(status, DiffStatus::Renamed),
                DiffStatus::Copied => assert_eq!(status, DiffStatus::Copied),
                DiffStatus::Ignored => assert_eq!(status, DiffStatus::Ignored),
                DiffStatus::Untracked => assert_eq!(status, DiffStatus::Untracked),
                DiffStatus::Typechange => assert_eq!(status, DiffStatus::Typechange),
            }
        }
    }

    #[test]
    fn test_serialization() {
        let signature = GitSignature {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            when: Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap(),
        };

        let commit = GitCommit {
            id: "abc123".to_string(),
            short_id: "abc".to_string(),
            author: signature.clone(),
            committer: signature,
            message: "Test commit".to_string(),
            summary: "Test commit".to_string(),
            parent_ids: vec!["parent1".to_string()],
            tree_id: "tree1".to_string(),
        };

        // Test serialization
        let serialized = serde_json::to_string(&commit).unwrap();
        assert!(serialized.contains("abc123"));
        assert!(serialized.contains("Test User"));
        assert!(serialized.contains("test@example.com"));

        // Test deserialization
        let deserialized: GitCommit = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.id, commit.id);
        assert_eq!(deserialized.author.name, commit.author.name);
        assert_eq!(deserialized.author.email, commit.author.email);
    }

    #[test]
    fn test_commit_with_multiple_parents() {
        let commit = GitCommit {
            id: "merge123".to_string(),
            short_id: "merge12".to_string(),
            author: GitSignature {
                name: "Merger".to_string(),
                email: "merger@example.com".to_string(),
                when: Utc::now(),
            },
            committer: GitSignature {
                name: "Merger".to_string(),
                email: "merger@example.com".to_string(),
                when: Utc::now(),
            },
            message: "Merge branch 'feature'".to_string(),
            summary: "Merge branch 'feature'".to_string(),
            parent_ids: vec!["parent1".to_string(), "parent2".to_string()],
            tree_id: "tree123".to_string(),
        };

        assert_eq!(commit.parent_ids.len(), 2);
        assert_eq!(commit.parent_ids[0], "parent1");
        assert_eq!(commit.parent_ids[1], "parent2");
    }

    #[test]
    fn test_git_diff_stats() {
        let stats = GitDiffStats {
            files_changed: 5,
            insertions: 100,
            deletions: 50,
        };

        assert_eq!(stats.files_changed, 5);
        assert_eq!(stats.insertions, 100);
        assert_eq!(stats.deletions, 50);
    }

    #[test]
    fn test_binary_diff() {
        let diff = GitDiff {
            old_file: Some("image.png".to_string()),
            new_file: Some("image.png".to_string()),
            hunks: vec![], // Binary files don't have hunks
            stats: GitDiffStats {
                files_changed: 1,
                insertions: 0,
                deletions: 0,
            },
            is_binary: true,
            status: DiffStatus::Modified,
            similarity: None,
        };

        assert!(diff.is_binary);
        assert!(diff.hunks.is_empty());
        assert_eq!(diff.status, DiffStatus::Modified);
    }

    #[test]
    fn test_clone_implementations() {
        let signature = GitSignature {
            name: "Test".to_string(),
            email: "test@example.com".to_string(),
            when: Utc::now(),
        };

        let commit = GitCommit {
            id: "test123".to_string(),
            short_id: "test".to_string(),
            author: signature.clone(),
            committer: signature.clone(),
            message: "Test".to_string(),
            summary: "Test".to_string(),
            parent_ids: vec![],
            tree_id: "tree".to_string(),
        };

        // Test cloning
        let cloned_commit = commit.clone();
        let cloned_signature = signature.clone();

        assert_eq!(commit.id, cloned_commit.id);
        assert_eq!(signature.name, cloned_signature.name);
    }
}
