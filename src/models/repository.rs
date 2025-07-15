use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryInfo {
    pub path: PathBuf,
    pub name: String,
    pub is_bare: bool,
    pub head_branch: Option<String>,
    pub branches: Vec<String>,
    pub tags: Vec<String>,
    pub remotes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub mode: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub name: String,
    pub path: String,
    pub id: String,
    pub filemode: git2::FileMode,
    pub is_tree: bool,
}

impl RepositoryInfo {
    pub fn from_repo(repo: &git2::Repository) -> anyhow::Result<Self> {
        let path = repo.workdir().unwrap_or_else(|| repo.path()).to_path_buf();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let is_bare = repo.is_bare();
        
        let head_branch = repo
            .head()
            .ok()
            .and_then(|head| head.shorthand().map(|s| s.to_string()));

        let mut branches = Vec::new();
        let mut tags = Vec::new();
        let mut remotes = Vec::new();

        // Collect branches
        if let Ok(branch_iter) = repo.branches(Some(git2::BranchType::Local)) {
            for branch in branch_iter {
                if let Ok((branch, _)) = branch {
                    if let Some(name) = branch.name()? {
                        branches.push(name.to_string());
                    }
                }
            }
        }

        // Collect tags
        if let Ok(tag_names) = repo.tag_names(None) {
            tag_names.iter().for_each(|tag| {
                if let Some(tag_name) = tag {
                    tags.push(tag_name.to_string());
                }
            });
        }

        // Collect remotes
        if let Ok(remote_names) = repo.remotes() {
            remote_names.iter().for_each(|remote| {
                if let Some(remote_name) = remote {
                    remotes.push(remote_name.to_string());
                }
            });
        }

        Ok(RepositoryInfo {
            path,
            name,
            is_bare,
            head_branch,
            branches,
            tags,
            remotes,
        })
    }
}