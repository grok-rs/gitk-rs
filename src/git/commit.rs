use crate::git::GitRepository;
use crate::models::GitCommit;
use anyhow::Result;

impl GitRepository {
    pub fn get_commit_parents(&self, commit: &GitCommit) -> Result<Vec<GitCommit>> {
        let mut parents = Vec::new();

        for parent_id in &commit.parent_ids {
            if let Ok(parent_commit) = self.get_commit(parent_id) {
                parents.push(parent_commit);
            }
        }

        Ok(parents)
    }

    pub fn get_commit_tree_entries(
        &self,
        commit_id: &str,
    ) -> Result<Vec<crate::models::TreeEntry>> {
        let oid = git2::Oid::from_str(commit_id)?;
        let commit = self.repo().find_commit(oid)?;
        let tree = commit.tree()?;

        let mut entries = Vec::new();

        tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
            let full_path = if root.is_empty() {
                entry.name().unwrap_or("").to_string()
            } else {
                format!("{}/{}", root, entry.name().unwrap_or(""))
            };

            entries.push(crate::models::TreeEntry {
                name: entry.name().unwrap_or("").to_string(),
                path: full_path,
                id: entry.id().to_string(),
                filemode: git2::FileMode::Tree, // Simplified for now
                is_tree: entry.filemode() == 0o040000, // Tree mode in git
            });

            git2::TreeWalkResult::Ok
        })?;

        Ok(entries)
    }

    pub fn get_commit_diff_stats(&self, commit_id: &str) -> Result<crate::models::GitDiffStats> {
        let oid = git2::Oid::from_str(commit_id)?;
        let commit = self.repo().find_commit(oid)?;

        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = self
            .repo()
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

        let stats = diff.stats()?;

        Ok(crate::models::GitDiffStats {
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
        })
    }
}
