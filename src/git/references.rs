use crate::git::GitRepository;
use crate::models::GitCommit;
use anyhow::{Result, anyhow};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum RefType {
    LocalBranch,
    RemoteBranch,
    Tag,
    Head,
    Other,
}

#[derive(Debug, Clone)]
pub struct GitRef {
    pub name: String,
    pub full_name: String,
    pub target: String, // commit SHA
    pub ref_type: RefType,
    pub is_head: bool,
}

#[derive(Debug, Clone)]
pub struct RefManager {
    refs: HashMap<String, GitRef>,
    branches: Vec<GitRef>,
    tags: Vec<GitRef>,
    remotes: Vec<GitRef>,
    head_ref: Option<GitRef>,
}

impl RefManager {
    pub fn new() -> Self {
        Self {
            refs: HashMap::new(),
            branches: Vec::new(),
            tags: Vec::new(),
            remotes: Vec::new(),
            head_ref: None,
        }
    }

    /// Load all references from the repository
    pub fn load_refs(&mut self, repo: &GitRepository) -> Result<()> {
        self.refs.clear();
        self.branches.clear();
        self.tags.clear();
        self.remotes.clear();
        self.head_ref = None;

        // Load branches
        self.load_branches(repo)?;

        // Load tags
        self.load_tags(repo)?;

        // Load remotes
        self.load_remotes(repo)?;

        // Determine HEAD
        self.determine_head(repo)?;

        Ok(())
    }

    fn load_branches(&mut self, repo: &GitRepository) -> Result<()> {
        // Load local branches
        let branch_iter = repo.repo().branches(Some(git2::BranchType::Local))?;
        for branch in branch_iter {
            let (branch, _branch_type) = branch?;
            if let Some(name) = branch.name()? {
                if let Some(target) = branch.get().target() {
                    let git_ref = GitRef {
                        name: name.to_string(),
                        full_name: format!("refs/heads/{}", name),
                        target: target.to_string(),
                        ref_type: RefType::LocalBranch,
                        is_head: false,
                    };

                    self.refs.insert(git_ref.full_name.clone(), git_ref.clone());
                    self.branches.push(git_ref);
                }
            }
        }

        // Load remote branches
        let remote_branch_iter = repo.repo().branches(Some(git2::BranchType::Remote))?;
        for branch in remote_branch_iter {
            let (branch, _branch_type) = branch?;
            if let Some(name) = branch.name()? {
                if let Some(target) = branch.get().target() {
                    let git_ref = GitRef {
                        name: name.to_string(),
                        full_name: format!("refs/remotes/{}", name),
                        target: target.to_string(),
                        ref_type: RefType::RemoteBranch,
                        is_head: false,
                    };

                    self.refs.insert(git_ref.full_name.clone(), git_ref.clone());
                    self.remotes.push(git_ref);
                }
            }
        }

        Ok(())
    }

    fn load_tags(&mut self, repo: &GitRepository) -> Result<()> {
        repo.repo().tag_names(None)?.iter().for_each(|tag_name| {
            if let Some(tag_name) = tag_name {
                let full_name = format!("refs/tags/{}", tag_name);

                // Try to resolve the tag to a commit
                if let Ok(reference) = repo.repo().find_reference(&full_name) {
                    if let Some(target_oid) = reference.target() {
                        let git_ref = GitRef {
                            name: tag_name.to_string(),
                            full_name: full_name.clone(),
                            target: target_oid.to_string(),
                            ref_type: RefType::Tag,
                            is_head: false,
                        };

                        self.refs.insert(full_name, git_ref.clone());
                        self.tags.push(git_ref);
                    } else if let Ok(target_oid) = reference.peel_to_commit() {
                        // Handle annotated tags
                        let git_ref = GitRef {
                            name: tag_name.to_string(),
                            full_name: full_name.clone(),
                            target: target_oid.id().to_string(),
                            ref_type: RefType::Tag,
                            is_head: false,
                        };

                        self.refs.insert(full_name, git_ref.clone());
                        self.tags.push(git_ref);
                    }
                }
            }
        });

        Ok(())
    }

    fn load_remotes(&mut self, _repo: &GitRepository) -> Result<()> {
        // Remote branches are already loaded in load_branches
        // This could be extended to load remote information
        Ok(())
    }

    fn determine_head(&mut self, repo: &GitRepository) -> Result<()> {
        if let Ok(head) = repo.repo().head() {
            let target = if let Some(target_oid) = head.target() {
                target_oid.to_string()
            } else if let Ok(commit) = head.peel_to_commit() {
                commit.id().to_string()
            } else {
                return Ok(()); // No valid HEAD
            };

            let (name, ref_type) = if head.is_branch() {
                if let Some(shorthand) = head.shorthand() {
                    (shorthand.to_string(), RefType::LocalBranch)
                } else {
                    ("HEAD".to_string(), RefType::Head)
                }
            } else {
                ("HEAD".to_string(), RefType::Head)
            };

            let git_ref = GitRef {
                name: name.clone(),
                full_name: "HEAD".to_string(),
                target,
                ref_type,
                is_head: true,
            };

            // Mark the corresponding branch as HEAD if it exists
            if let Some(branch_ref) = self.branches.iter_mut().find(|r| r.name == name) {
                branch_ref.is_head = true;
            }

            self.head_ref = Some(git_ref.clone());
            self.refs.insert("HEAD".to_string(), git_ref);
        }

        Ok(())
    }

    /// Get all branches (local and remote)
    pub fn get_branches(&self) -> Vec<&GitRef> {
        let mut all_branches = Vec::new();
        all_branches.extend(&self.branches);
        all_branches.extend(&self.remotes);
        all_branches
    }

    /// Get only local branches
    pub fn get_local_branches(&self) -> &[GitRef] {
        &self.branches
    }

    /// Get only remote branches
    pub fn get_remote_branches(&self) -> &[GitRef] {
        &self.remotes
    }

    /// Get all tags
    pub fn get_tags(&self) -> &[GitRef] {
        &self.tags
    }

    /// Get HEAD reference
    pub fn get_head(&self) -> Option<&GitRef> {
        self.head_ref.as_ref()
    }

    /// Get reference by name
    pub fn get_ref(&self, name: &str) -> Option<&GitRef> {
        self.refs.get(name)
    }

    /// Get all references
    pub fn get_all_refs(&self) -> Vec<&GitRef> {
        self.refs.values().collect()
    }

    /// Find references pointing to a specific commit
    pub fn get_refs_for_commit(&self, commit_sha: &str) -> Vec<&GitRef> {
        self.refs
            .values()
            .filter(|git_ref| git_ref.target == commit_sha)
            .collect()
    }

    /// Check if a commit is on a specific branch
    pub fn is_commit_on_branch(
        &self,
        repo: &GitRepository,
        commit_sha: &str,
        branch_name: &str,
    ) -> Result<bool> {
        if let Some(branch_ref) = self.refs.get(&format!("refs/heads/{}", branch_name)) {
            // Simple check: is this commit the branch tip?
            if branch_ref.target == commit_sha {
                return Ok(true);
            }

            // More complex check: walk the branch history
            let branch_oid = git2::Oid::from_str(&branch_ref.target)?;
            let commit_oid = git2::Oid::from_str(commit_sha)?;

            let mut revwalk = repo.repo().revwalk()?;
            revwalk.push(branch_oid)?;

            for oid in revwalk {
                let oid = oid?;
                if oid == commit_oid {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Get the current branch name
    pub fn get_current_branch(&self) -> Option<String> {
        self.head_ref
            .as_ref()
            .filter(|head| head.ref_type == RefType::LocalBranch)
            .map(|head| head.name.clone())
    }

    /// Check if repository is in detached HEAD state
    pub fn is_detached_head(&self) -> bool {
        self.head_ref
            .as_ref()
            .map(|head| head.ref_type == RefType::Head)
            .unwrap_or(false)
    }
}

impl GitRepository {
    /// Get a reference manager for this repository
    pub fn get_ref_manager(&self) -> Result<RefManager> {
        let mut ref_manager = RefManager::new();
        ref_manager.load_refs(self)?;
        Ok(ref_manager)
    }

    /// Get commits reachable from a reference
    pub fn get_commits_from_ref(
        &self,
        ref_name: &str,
        limit: Option<usize>,
    ) -> Result<Vec<GitCommit>> {
        let reference = self.repo().find_reference(ref_name)?;
        let commit = reference.peel_to_commit()?;

        let mut revwalk = self.repo().revwalk()?;
        revwalk.set_sorting(git2::Sort::TIME)?;
        revwalk.push(commit.id())?;

        let mut commits = Vec::new();
        let limit = limit.unwrap_or(1000);

        for (index, oid) in revwalk.enumerate() {
            if index >= limit {
                break;
            }

            let oid = oid?;
            let commit = self.repo().find_commit(oid)?;
            commits.push(GitCommit::new(&commit)?);
        }

        Ok(commits)
    }

    /// Create a new branch
    pub fn create_branch(&self, branch_name: &str, target_commit: &str) -> Result<()> {
        let target_oid = git2::Oid::from_str(target_commit)?;
        let target_commit = self.repo().find_commit(target_oid)?;

        self.repo().branch(branch_name, &target_commit, false)?;

        Ok(())
    }

    /// Delete a branch
    pub fn delete_branch(&self, branch_name: &str) -> Result<()> {
        let mut branch = self
            .repo()
            .find_branch(branch_name, git2::BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }

    /// Create a new tag
    pub fn create_tag(
        &self,
        tag_name: &str,
        target_commit: &str,
        message: Option<&str>,
    ) -> Result<()> {
        let target_oid = git2::Oid::from_str(target_commit)?;
        let target_commit = self.repo().find_commit(target_oid)?;

        if let Some(message) = message {
            // Create annotated tag
            let signature = self.repo().signature()?;
            self.repo().tag(
                tag_name,
                target_commit.as_object(),
                &signature,
                message,
                false,
            )?;
        } else {
            // Create lightweight tag
            self.repo()
                .tag_lightweight(tag_name, target_commit.as_object(), false)?;
        }

        Ok(())
    }

    /// Delete a tag
    pub fn delete_tag(&self, tag_name: &str) -> Result<()> {
        self.repo().tag_delete(tag_name)?;
        Ok(())
    }
}
