use anyhow::Result;
use crate::models::{GitDiff, GitDiffStats, GitHunk, GitDiffLine, DiffStatus};
use crate::git::GitRepository;

/// Convert git2::Delta to our DiffStatus
fn delta_to_status(delta: git2::Delta) -> DiffStatus {
    match delta {
        git2::Delta::Added => DiffStatus::Added,
        git2::Delta::Deleted => DiffStatus::Deleted,
        git2::Delta::Modified => DiffStatus::Modified,
        git2::Delta::Renamed => DiffStatus::Renamed,
        git2::Delta::Copied => DiffStatus::Copied,
        git2::Delta::Ignored => DiffStatus::Ignored,
        git2::Delta::Untracked => DiffStatus::Untracked,
        git2::Delta::Typechange => DiffStatus::Typechange,
        _ => DiffStatus::Modified, // Default fallback
    }
}

impl GitRepository {
    pub fn get_commit_diff(&self, commit_id: &str) -> Result<Vec<GitDiff>> {
        let oid = git2::Oid::from_str(commit_id)?;
        let commit = self.repo().find_commit(oid)?;
        
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        
        let diff = self.repo().diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&tree),
            None,
        )?;
        
        let mut diffs = Vec::new();
        
        for delta in diff.deltas() {
            let old_file = delta.old_file().path().map(|p| p.to_string_lossy().to_string());
            let new_file = delta.new_file().path().map(|p| p.to_string_lossy().to_string());
            let status = delta_to_status(delta.status());
            let similarity = None; // Similarity detection requires more complex processing
            
            // Check if file is binary
            let is_binary = delta.old_file().is_binary() || delta.new_file().is_binary();
            
            let git_diff = GitDiff {
                old_file,
                new_file,
                hunks: Vec::new(), // Will be populated separately for non-binary files
                stats: GitDiffStats {
                    files_changed: 1,
                    insertions: 0,
                    deletions: 0,
                },
                is_binary,
                status,
                similarity,
            };
            
            diffs.push(git_diff);
        }
        
        Ok(diffs)
    }

    pub fn get_file_diff(&self, commit_id: &str, file_path: &str) -> Result<GitDiff> {
        let oid = git2::Oid::from_str(commit_id)?;
        let commit = self.repo().find_commit(oid)?;
        
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        
        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.pathspec(file_path);
        
        let diff = self.repo().diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&tree),
            Some(&mut diff_opts),
        )?;
        
        let mut result_diff = GitDiff {
            old_file: None,
            new_file: None,
            hunks: Vec::new(),
            stats: GitDiffStats {
                files_changed: 0,
                insertions: 0,
                deletions: 0,
            },
            is_binary: false,
            status: DiffStatus::Modified,
            similarity: None,
        };
        
        // Get basic file info from deltas
        for delta in diff.deltas() {
            result_diff.old_file = delta.old_file().path().map(|p| p.to_string_lossy().to_string());
            result_diff.new_file = delta.new_file().path().map(|p| p.to_string_lossy().to_string());
            result_diff.stats.files_changed = 1;
            result_diff.status = delta_to_status(delta.status());
            result_diff.similarity = None; // Similarity detection requires more complex processing
            result_diff.is_binary = delta.old_file().is_binary() || delta.new_file().is_binary();
        }
        
        // Generate patch and parse it for hunk information
        if let Ok(patch) = git2::Patch::from_diff(&diff, 0) {
            if let Some(patch) = patch {
                let num_hunks = patch.num_hunks();
                
                for hunk_idx in 0..num_hunks {
                    if let Ok((hunk, _)) = patch.hunk(hunk_idx) {
                        let mut lines = Vec::new();
                        let num_lines = patch.num_lines_in_hunk(hunk_idx)?;
                        
                        for line_idx in 0..num_lines {
                            if let Ok(line) = patch.line_in_hunk(hunk_idx, line_idx) {
                                let content = String::from_utf8_lossy(line.content()).to_string();
                                let origin = line.origin();
                                
                                match origin {
                                    '+' => result_diff.stats.insertions += 1,
                                    '-' => result_diff.stats.deletions += 1,
                                    _ => {}
                                }
                                
                                lines.push(GitDiffLine {
                                    origin,
                                    content,
                                    old_lineno: line.old_lineno(),
                                    new_lineno: line.new_lineno(),
                                });
                            }
                        }
                        
                        result_diff.hunks.push(GitHunk {
                            old_start: hunk.old_start(),
                            old_lines: hunk.old_lines(),
                            new_start: hunk.new_start(),
                            new_lines: hunk.new_lines(),
                            header: format!("@@ -{},{} +{},{} @@", hunk.old_start(), hunk.old_lines(), hunk.new_start(), hunk.new_lines()),
                            lines,
                        });
                    }
                }
            }
        }
        
        Ok(result_diff)
    }

    /// Get enhanced diff information for a commit including binary detection and renames
    pub fn get_commit_diff_enhanced(&self, commit_id: &str) -> Result<Vec<GitDiff>> {
        let oid = git2::Oid::from_str(commit_id)?;
        let commit = self.repo().find_commit(oid)?;
        
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        
        let diff = self.repo().diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&tree),
            None,
        )?;
        
        let mut diffs = Vec::new();
        
        for delta in diff.deltas() {
            let old_file = delta.old_file().path().map(|p| p.to_string_lossy().to_string());
            let new_file = delta.new_file().path().map(|p| p.to_string_lossy().to_string());
            let status = delta_to_status(delta.status());
            let similarity = None; // Similarity detection requires more complex processing
            let is_binary = delta.old_file().is_binary() || delta.new_file().is_binary();
            
            let mut hunks = Vec::new();
            let mut insertions = 0;
            let mut deletions = 0;
            
            // Only process hunks for non-binary files
            if !is_binary {
                if let Ok(patch) = git2::Patch::from_diff(&diff, diffs.len()) {
                    if let Some(patch) = patch {
                        let num_hunks = patch.num_hunks();
                        
                        for hunk_idx in 0..num_hunks {
                            if let Ok((hunk, _)) = patch.hunk(hunk_idx) {
                                let mut lines = Vec::new();
                                let num_lines = patch.num_lines_in_hunk(hunk_idx)?;
                                
                                for line_idx in 0..num_lines {
                                    if let Ok(line) = patch.line_in_hunk(hunk_idx, line_idx) {
                                        let content = String::from_utf8_lossy(line.content()).to_string();
                                        let origin = line.origin();
                                        
                                        match origin {
                                            '+' => insertions += 1,
                                            '-' => deletions += 1,
                                            _ => {}
                                        }
                                        
                                        lines.push(GitDiffLine {
                                            origin,
                                            content,
                                            old_lineno: line.old_lineno(),
                                            new_lineno: line.new_lineno(),
                                        });
                                    }
                                }
                                
                                hunks.push(GitHunk {
                                    old_start: hunk.old_start(),
                                    old_lines: hunk.old_lines(),
                                    new_start: hunk.new_start(),
                                    new_lines: hunk.new_lines(),
                                    header: format!("@@ -{},{} +{},{} @@", hunk.old_start(), hunk.old_lines(), hunk.new_start(), hunk.new_lines()),
                                    lines,
                                });
                            }
                        }
                    }
                }
            }
            
            let git_diff = GitDiff {
                old_file,
                new_file,
                hunks,
                stats: GitDiffStats {
                    files_changed: 1,
                    insertions,
                    deletions,
                },
                is_binary,
                status,
                similarity,
            };
            
            diffs.push(git_diff);
        }
        
        Ok(diffs)
    }
}