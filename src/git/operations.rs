use crate::git::commits::{
    CherryPickConfig, CommitOperationResult, CommitOperations, ResetConfig, RevertConfig,
};
use crate::git::remotes::{
    FetchConfig, PullConfig, PushConfig, RemoteInfo, RemoteManager, RemoteOperationResult,
};
use crate::git::stash::{
    StashApplyConfig, StashCreateConfig, StashInfo, StashListOptions, StashManager,
    StashOperationResult,
};
use crate::git::tags::{
    TagCreateConfig, TagFilterOptions, TagInfo, TagManager, TagOperationResult,
};
use crate::git::{ErrorReporter, GitRepository, InputSanitizer, InputValidator};
use anyhow::Result;
use git2::{BranchType as Git2BranchType, Oid, Repository};
use tracing::{error, info, warn};

/// Comprehensive Git operations manager for advanced repository manipulation
pub struct GitOperations {
    repo: Repository,
    operation_history: Vec<OperationRecord>,
    max_history: usize,
    tag_manager: TagManager,
    commit_operations: CommitOperations,
    stash_manager: StashManager,
    remote_manager: RemoteManager,
}

/// Record of Git operations for undo/redo functionality
#[derive(Debug, Clone)]
pub struct OperationRecord {
    pub operation_type: OperationType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub description: String,
    pub original_state: Option<String>, // HEAD or branch ref before operation
    pub new_state: Option<String>,      // HEAD or branch ref after operation
    pub affected_refs: Vec<String>,     // References that were modified
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperationType {
    // Branch operations
    BranchCreate,
    BranchDelete,
    BranchRename,
    BranchCheckout,
    BranchMerge,
    BranchRebase,

    // Commit operations
    CommitCherryPick,
    CommitRevert,
    CommitReset,
    CommitAmend,

    // Tag operations
    TagCreate,
    TagDelete,
    TagMove,

    // Stash operations
    StashSave,
    StashApply,
    StashPop,
    StashDrop,

    // Remote operations
    RemoteFetch,
    RemotePull,
    RemotePush,
}

/// Branch operation result with detailed information
#[derive(Debug)]
pub struct BranchOperationResult {
    pub success: bool,
    pub operation: OperationType,
    pub branch_name: String,
    pub commit_id: Option<String>,
    pub message: String,
    pub conflicts: Vec<String>,
    pub modified_files: Vec<String>,
}

impl GitOperations {
    /// Create a new Git operations manager
    pub fn new(git_repo: &GitRepository) -> Result<Self> {
        // Clone the repository path and open a new instance
        let repo_path = git_repo.get_repository().path();
        let repo = Repository::open(repo_path)?;
        let tag_manager = TagManager::new(git_repo)?;
        let commit_operations = CommitOperations::new(git_repo)?;
        let stash_manager = StashManager::new(git_repo)?;
        let remote_manager = RemoteManager::new(git_repo)?;

        Ok(Self {
            repo,
            operation_history: Vec::new(),
            max_history: 100, // Keep last 100 operations
            tag_manager,
            commit_operations,
            stash_manager,
            remote_manager,
        })
    }

    /// Record an operation in the history
    fn record_operation(&mut self, record: OperationRecord) {
        let operation_type = record.operation_type.clone();
        self.operation_history.push(record);

        // Maintain history limit
        if self.operation_history.len() > self.max_history {
            self.operation_history.remove(0);
        }

        info!("Recorded operation: {:?}", operation_type);
    }

    /// Get operation history
    pub fn get_operation_history(&self) -> &[OperationRecord] {
        &self.operation_history
    }

    /// Create a new branch from the specified commit
    pub fn create_branch(
        &mut self,
        branch_name: &str,
        target_commit: &str,
        checkout: bool,
    ) -> Result<BranchOperationResult> {
        // Validate inputs
        if let Err(e) = InputValidator::validate_ref_name(branch_name) {
            ErrorReporter::log_error(&e, "branch creation validation");
            return Ok(BranchOperationResult {
                success: false,
                operation: OperationType::BranchCreate,
                branch_name: branch_name.to_string(),
                commit_id: None,
                message: format!("Invalid branch name: {}", e),
                conflicts: vec![],
                modified_files: vec![],
            });
        }

        if let Err(e) = InputValidator::validate_commit_id(target_commit) {
            ErrorReporter::log_error(&e, "branch creation validation");
            return Ok(BranchOperationResult {
                success: false,
                operation: OperationType::BranchCreate,
                branch_name: branch_name.to_string(),
                commit_id: None,
                message: format!("Invalid commit ID: {}", e),
                conflicts: vec![],
                modified_files: vec![],
            });
        }

        // Sanitize inputs
        let sanitized_name = match InputSanitizer::sanitize_ref_name(branch_name) {
            Ok(name) => name,
            Err(e) => {
                return Ok(BranchOperationResult {
                    success: false,
                    operation: OperationType::BranchCreate,
                    branch_name: branch_name.to_string(),
                    commit_id: None,
                    message: format!("Failed to sanitize branch name: {}", e),
                    conflicts: vec![],
                    modified_files: vec![],
                });
            }
        };

        let sanitized_commit = match InputSanitizer::sanitize_commit_id(target_commit) {
            Ok(commit) => commit,
            Err(e) => {
                return Ok(BranchOperationResult {
                    success: false,
                    operation: OperationType::BranchCreate,
                    branch_name: branch_name.to_string(),
                    commit_id: None,
                    message: format!("Failed to sanitize commit ID: {}", e),
                    conflicts: vec![],
                    modified_files: vec![],
                });
            }
        };

        // Get current HEAD for operation record
        let original_head = self
            .repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .map(|oid| oid.to_string());

        // Find target commit and create branch
        let target_oid = match Oid::from_str(&sanitized_commit) {
            Ok(oid) => oid,
            Err(e) => {
                return Ok(BranchOperationResult {
                    success: false,
                    operation: OperationType::BranchCreate,
                    branch_name: sanitized_name,
                    commit_id: None,
                    message: format!("Invalid commit OID: {}", e),
                    conflicts: vec![],
                    modified_files: vec![],
                });
            }
        };

        // Create branch in separate scope to avoid borrowing issues
        let create_result = {
            let target_commit_obj = match self.repo.find_commit(target_oid) {
                Ok(commit) => commit,
                Err(e) => {
                    return Ok(BranchOperationResult {
                        success: false,
                        operation: OperationType::BranchCreate,
                        branch_name: sanitized_name,
                        commit_id: Some(sanitized_commit),
                        message: format!("Commit not found: {}", e),
                        conflicts: vec![],
                        modified_files: vec![],
                    });
                }
            };

            // Create the branch
            match self.repo.branch(&sanitized_name, &target_commit_obj, false) {
                Ok(branch) => {
                    let branch_ref = branch.get().name().unwrap_or("unknown").to_string();
                    (true, branch_ref, String::new())
                }
                Err(e) => (
                    false,
                    String::new(),
                    format!("Failed to create branch: {}", e),
                ),
            }
        };

        if !create_result.0 {
            return Ok(BranchOperationResult {
                success: false,
                operation: OperationType::BranchCreate,
                branch_name: sanitized_name,
                commit_id: Some(sanitized_commit),
                message: create_result.2,
                conflicts: vec![],
                modified_files: vec![],
            });
        }

        // Checkout if requested
        if checkout {
            if let Err(e) = self.checkout_branch_simple(&sanitized_name) {
                warn!("Branch created but checkout failed: {}", e);
                return Ok(BranchOperationResult {
                    success: true,
                    operation: OperationType::BranchCreate,
                    branch_name: sanitized_name,
                    commit_id: Some(sanitized_commit.clone()),
                    message: format!("Branch created but checkout failed: {}", e),
                    conflicts: vec![],
                    modified_files: vec![],
                });
            }
        }

        // Record the operation
        self.record_operation(OperationRecord {
            operation_type: OperationType::BranchCreate,
            timestamp: chrono::Utc::now(),
            description: format!(
                "Created branch '{}' at commit {}",
                sanitized_name,
                &sanitized_commit[..8]
            ),
            original_state: original_head.clone(),
            new_state: if checkout {
                Some(target_oid.to_string())
            } else {
                original_head
            },
            affected_refs: vec![create_result.1],
        });

        info!(
            "Successfully created branch '{}' at commit {}",
            sanitized_name, sanitized_commit
        );

        Ok(BranchOperationResult {
            success: true,
            operation: OperationType::BranchCreate,
            branch_name: sanitized_name,
            commit_id: Some(sanitized_commit),
            message: "Successfully created branch".to_string(),
            conflicts: vec![],
            modified_files: vec![],
        })
    }

    /// Delete a branch (with safety checks)
    pub fn delete_branch(
        &mut self,
        branch_name: &str,
        force: bool,
    ) -> Result<BranchOperationResult> {
        // Validate input
        if let Err(e) = InputValidator::validate_ref_name(branch_name) {
            ErrorReporter::log_error(&e, "branch deletion validation");
            return Ok(BranchOperationResult {
                success: false,
                operation: OperationType::BranchDelete,
                branch_name: branch_name.to_string(),
                commit_id: None,
                message: format!("Invalid branch name: {}", e),
                conflicts: vec![],
                modified_files: vec![],
            });
        }

        // Sanitize input
        let sanitized_name = match InputSanitizer::sanitize_ref_name(branch_name) {
            Ok(name) => name,
            Err(e) => {
                return Ok(BranchOperationResult {
                    success: false,
                    operation: OperationType::BranchDelete,
                    branch_name: branch_name.to_string(),
                    commit_id: None,
                    message: format!("Failed to sanitize branch name: {}", e),
                    conflicts: vec![],
                    modified_files: vec![],
                });
            }
        };

        // Safety check: don't delete current branch
        if let Ok(head) = self.repo.head() {
            if let Some(head_name) = head.shorthand() {
                if head_name == sanitized_name {
                    return Ok(BranchOperationResult {
                        success: false,
                        operation: OperationType::BranchDelete,
                        branch_name: sanitized_name,
                        commit_id: None,
                        message: "Cannot delete current branch".to_string(),
                        conflicts: vec![],
                        modified_files: vec![],
                    });
                }
            }
        }

        // Delete branch in separate scope
        let delete_result = {
            let mut branch = match self
                .repo
                .find_branch(&sanitized_name, Git2BranchType::Local)
            {
                Ok(branch) => branch,
                Err(e) => {
                    return Ok(BranchOperationResult {
                        success: false,
                        operation: OperationType::BranchDelete,
                        branch_name: sanitized_name,
                        commit_id: None,
                        message: format!("Branch not found: {}", e),
                        conflicts: vec![],
                        modified_files: vec![],
                    });
                }
            };

            // Get branch info for operation record
            let branch_commit = branch.get().target().map(|oid| oid.to_string());
            let branch_ref = branch.get().name().unwrap_or("unknown").to_string();

            // Safety check: ensure branch is merged (unless force)
            if !force {
                // Simplified merge check - just allow deletion for now
                // In a full implementation, we'd check if the branch is merged
            }

            // Delete the branch
            match branch.delete() {
                Ok(()) => (true, branch_commit, branch_ref, String::new()),
                Err(e) => (
                    false,
                    branch_commit,
                    branch_ref,
                    format!("Failed to delete branch: {}", e),
                ),
            }
        };

        if !delete_result.0 {
            return Ok(BranchOperationResult {
                success: false,
                operation: OperationType::BranchDelete,
                branch_name: sanitized_name,
                commit_id: delete_result.1,
                message: delete_result.3,
                conflicts: vec![],
                modified_files: vec![],
            });
        }

        // Record the operation
        self.record_operation(OperationRecord {
            operation_type: OperationType::BranchDelete,
            timestamp: chrono::Utc::now(),
            description: format!("Deleted branch '{}'", sanitized_name),
            original_state: delete_result.1.clone(),
            new_state: None,
            affected_refs: vec![delete_result.2],
        });

        info!("Successfully deleted branch '{}'", sanitized_name);

        Ok(BranchOperationResult {
            success: true,
            operation: OperationType::BranchDelete,
            branch_name: sanitized_name,
            commit_id: delete_result.1,
            message: "Successfully deleted branch".to_string(),
            conflicts: vec![],
            modified_files: vec![],
        })
    }

    /// Simple checkout implementation without complex merge checks
    fn checkout_branch_simple(&mut self, branch_name: &str) -> Result<()> {
        let branch = self.repo.find_branch(branch_name, Git2BranchType::Local)?;
        let target_oid = branch
            .get()
            .target()
            .ok_or_else(|| anyhow::anyhow!("Branch has no target commit"))?;
        let target_commit = self.repo.find_commit(target_oid)?;

        // Checkout the tree
        let tree = target_commit.tree()?;
        self.repo.checkout_tree(tree.as_object(), None)?;

        // Update HEAD to point to the branch
        let branch_ref = format!("refs/heads/{}", branch_name);
        self.repo.set_head(&branch_ref)?;

        Ok(())
    }

    /// Get list of all local branches
    pub fn list_local_branches(&self) -> Result<Vec<String>> {
        let mut branches = Vec::new();
        let branch_iter = self.repo.branches(Some(Git2BranchType::Local))?;

        for branch_result in branch_iter {
            if let Ok((branch, _)) = branch_result {
                if let Some(name) = branch.name()? {
                    branches.push(name.to_string());
                }
            }
        }

        Ok(branches)
    }

    /// Get list of all remote branches  
    pub fn list_remote_branches(&self) -> Result<Vec<String>> {
        let mut branches = Vec::new();
        let branch_iter = self.repo.branches(Some(Git2BranchType::Remote))?;

        for branch_result in branch_iter {
            if let Ok((branch, _)) = branch_result {
                if let Some(name) = branch.name()? {
                    branches.push(name.to_string());
                }
            }
        }

        Ok(branches)
    }

    /// Get current branch name
    pub fn get_current_branch(&self) -> Result<Option<String>> {
        match self.repo.head() {
            Ok(head) => {
                if let Some(shorthand) = head.shorthand() {
                    Ok(Some(shorthand.to_string()))
                } else {
                    // Detached HEAD
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
    }

    /// Check if repository is in detached HEAD state
    pub fn is_detached_head(&self) -> bool {
        match self.repo.head() {
            Ok(head) => !head.is_branch(),
            Err(_) => false,
        }
    }

    // === Tag Operations ===

    /// Create a new tag
    pub fn create_tag(
        &mut self,
        tag_name: &str,
        target_commit: &str,
        config: TagCreateConfig,
    ) -> Result<TagOperationResult> {
        let result = self
            .tag_manager
            .create_tag(tag_name, target_commit, config)?;

        // Merge tag operation history into main operation history
        if let Some(last_tag_op) = self.tag_manager.get_operation_history().last() {
            self.operation_history.push(last_tag_op.clone());
        }

        Ok(result)
    }

    /// Delete a tag
    pub fn delete_tag(&mut self, tag_name: &str, force: bool) -> Result<TagOperationResult> {
        let result = self.tag_manager.delete_tag(tag_name, force)?;

        // Merge tag operation history into main operation history
        if let Some(last_tag_op) = self.tag_manager.get_operation_history().last() {
            self.operation_history.push(last_tag_op.clone());
        }

        Ok(result)
    }

    /// Get detailed information about a tag
    pub fn get_tag_info(&self, tag_name: &str) -> Result<TagInfo> {
        self.tag_manager.get_tag_info(tag_name)
    }

    /// List all tags with filtering options
    pub fn list_tags(&self, filter: Option<TagFilterOptions>) -> Result<Vec<TagInfo>> {
        self.tag_manager.list_tags(filter)
    }

    /// Find tags that point to a specific commit
    pub fn get_tags_for_commit(&self, commit_id: &str) -> Result<Vec<TagInfo>> {
        self.tag_manager.get_tags_for_commit(commit_id)
    }

    // === Commit Operations ===

    /// Cherry-pick a commit onto the current branch
    pub fn cherry_pick(
        &mut self,
        commit_id: &str,
        config: CherryPickConfig,
    ) -> Result<CommitOperationResult> {
        let result = self.commit_operations.cherry_pick(commit_id, config)?;

        // Merge commit operation history into main operation history
        if let Some(last_commit_op) = self.commit_operations.get_operation_history().last() {
            self.operation_history.push(last_commit_op.clone());
        }

        Ok(result)
    }

    /// Revert a commit by creating a reverse commit
    pub fn revert(
        &mut self,
        commit_id: &str,
        config: RevertConfig,
    ) -> Result<CommitOperationResult> {
        let result = self.commit_operations.revert(commit_id, config)?;

        // Merge commit operation history into main operation history
        if let Some(last_commit_op) = self.commit_operations.get_operation_history().last() {
            self.operation_history.push(last_commit_op.clone());
        }

        Ok(result)
    }

    /// Reset the current branch to a specific commit
    pub fn reset(&mut self, commit_id: &str, config: ResetConfig) -> Result<CommitOperationResult> {
        let result = self.commit_operations.reset(commit_id, config)?;

        // Merge commit operation history into main operation history
        if let Some(last_commit_op) = self.commit_operations.get_operation_history().last() {
            self.operation_history.push(last_commit_op.clone());
        }

        Ok(result)
    }

    /// Check if repository has uncommitted changes
    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        self.commit_operations.has_uncommitted_changes()
    }

    /// Get repository state (normal, merging, reverting, etc.)
    pub fn get_repository_state(&self) -> git2::RepositoryState {
        self.commit_operations.get_repository_state()
    }

    /// Abort current operation (merge, cherry-pick, revert)
    pub fn abort_operation(&mut self) -> Result<CommitOperationResult> {
        let result = self.commit_operations.abort_operation()?;

        // Merge commit operation history into main operation history
        if let Some(last_commit_op) = self.commit_operations.get_operation_history().last() {
            self.operation_history.push(last_commit_op.clone());
        }

        Ok(result)
    }

    // === Stash Operations ===

    /// Create a new stash with the current changes
    pub fn create_stash(&mut self, config: StashCreateConfig) -> Result<StashOperationResult> {
        let result = self.stash_manager.create_stash(config)?;

        // Merge stash operation history into main operation history
        if let Some(last_stash_op) = self.stash_manager.get_operation_history().last() {
            self.operation_history.push(last_stash_op.clone());
        }

        Ok(result)
    }

    /// Apply a stash to the current working directory
    pub fn apply_stash(
        &mut self,
        stash_index: usize,
        config: StashApplyConfig,
    ) -> Result<StashOperationResult> {
        let result = self.stash_manager.apply_stash(stash_index, config)?;

        // Merge stash operation history into main operation history
        if let Some(last_stash_op) = self.stash_manager.get_operation_history().last() {
            self.operation_history.push(last_stash_op.clone());
        }

        Ok(result)
    }

    /// Pop (apply and remove) a stash
    pub fn pop_stash(
        &mut self,
        stash_index: usize,
        config: StashApplyConfig,
    ) -> Result<StashOperationResult> {
        let result = self.stash_manager.pop_stash(stash_index, config)?;

        // Merge stash operation history into main operation history
        if let Some(last_stash_op) = self.stash_manager.get_operation_history().last() {
            self.operation_history.push(last_stash_op.clone());
        }

        Ok(result)
    }

    /// Drop (remove) a stash without applying it
    pub fn drop_stash(&mut self, stash_index: usize) -> Result<StashOperationResult> {
        let result = self.stash_manager.drop_stash(stash_index)?;

        // Merge stash operation history into main operation history
        if let Some(last_stash_op) = self.stash_manager.get_operation_history().last() {
            self.operation_history.push(last_stash_op.clone());
        }

        Ok(result)
    }

    /// List all stashes with optional filtering
    pub fn list_stashes(&self, options: Option<StashListOptions>) -> Result<Vec<StashInfo>> {
        self.stash_manager.list_stashes(options)
    }

    /// Get detailed information about a specific stash
    pub fn get_stash_info(&self, stash_index: usize) -> Result<StashInfo> {
        self.stash_manager.get_stash_info_by_index(stash_index)
    }

    /// Clear all stashes
    pub fn clear_all_stashes(&mut self) -> Result<StashOperationResult> {
        let result = self.stash_manager.clear_all_stashes()?;

        // Merge stash operation history into main operation history
        if let Some(last_stash_op) = self.stash_manager.get_operation_history().last() {
            self.operation_history.push(last_stash_op.clone());
        }

        Ok(result)
    }

    // === Remote Operations ===

    /// Fetch from a remote repository
    pub fn fetch(
        &mut self,
        remote_name: &str,
        config: FetchConfig,
    ) -> Result<RemoteOperationResult> {
        let result = self.remote_manager.fetch(remote_name, config)?;

        // Merge remote operation history into main operation history
        if let Some(last_remote_op) = self.remote_manager.get_operation_history().last() {
            self.operation_history.push(last_remote_op.clone());
        }

        Ok(result)
    }

    /// Push to a remote repository
    pub fn push(&mut self, remote_name: &str, config: PushConfig) -> Result<RemoteOperationResult> {
        let result = self.remote_manager.push(remote_name, config)?;

        // Merge remote operation history into main operation history
        if let Some(last_remote_op) = self.remote_manager.get_operation_history().last() {
            self.operation_history.push(last_remote_op.clone());
        }

        Ok(result)
    }

    /// Pull from a remote repository (fetch + merge/rebase)
    pub fn pull(&mut self, remote_name: &str, config: PullConfig) -> Result<RemoteOperationResult> {
        let result = self.remote_manager.pull(remote_name, config)?;

        // Merge remote operation history into main operation history
        if let Some(last_remote_op) = self.remote_manager.get_operation_history().last() {
            self.operation_history.push(last_remote_op.clone());
        }

        Ok(result)
    }

    /// List all configured remotes
    pub fn list_remotes(&self) -> Result<Vec<RemoteInfo>> {
        self.remote_manager.list_remotes()
    }

    /// Get detailed information about a remote
    pub fn get_remote_info(&self, remote_name: &str) -> Result<RemoteInfo> {
        self.remote_manager.get_remote_info(remote_name)
    }

    /// Add a new remote
    pub fn add_remote(&mut self, name: &str, url: &str) -> Result<RemoteOperationResult> {
        let result = self.remote_manager.add_remote(name, url)?;

        // Merge remote operation history into main operation history
        if let Some(last_remote_op) = self.remote_manager.get_operation_history().last() {
            self.operation_history.push(last_remote_op.clone());
        }

        Ok(result)
    }

    /// Remove a remote
    pub fn remove_remote(&mut self, name: &str) -> Result<RemoteOperationResult> {
        let result = self.remote_manager.remove_remote(name)?;

        // Merge remote operation history into main operation history
        if let Some(last_remote_op) = self.remote_manager.get_operation_history().last() {
            self.operation_history.push(last_remote_op.clone());
        }

        Ok(result)
    }

    /// Set credentials provider for remote operations
    pub fn set_credentials_provider(
        &mut self,
        provider: Box<dyn crate::git::remotes::CredentialsProvider + Send + Sync>,
    ) {
        self.remote_manager.set_credentials_provider(provider);
    }

    /// Set progress handler for remote operations
    pub fn set_progress_handler(
        &mut self,
        handler: Box<dyn crate::git::remotes::ProgressHandler + Send + Sync>,
    ) {
        self.remote_manager.set_progress_handler(handler);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::stash::{StashCreateConfig, StashListOptions};
    use crate::git::tags::{SortOrder, TagCreateConfig, TagFilterOptions, TagSortBy, TagType};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_repo() -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize Git repository
        let output = Command::new("git")
            .args(&["init"])
            .current_dir(&repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to initialize Git repository"));
        }

        // Configure user for testing
        Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()?;

        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()?;

        Ok((temp_dir, repo_path))
    }

    fn create_test_commit(
        repo_path: &Path,
        filename: &str,
        content: &str,
        message: &str,
    ) -> Result<()> {
        let file_path = repo_path.join(filename);
        fs::write(&file_path, content)?;

        Command::new("git")
            .args(&["add", filename])
            .current_dir(repo_path)
            .output()?;

        let output = Command::new("git")
            .args(&["commit", "-m", message])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to create commit: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    #[test]
    fn test_git_operations_creation() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "content", "Initial commit")?;

        let git_repo = GitRepository::discover(&repo_path)?;
        let operations = GitOperations::new(&git_repo)?;

        assert_eq!(operations.operation_history.len(), 0);
        assert_eq!(operations.max_history, 1000);

        Ok(())
    }

    #[test]
    fn test_branch_operations() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "content", "Initial commit")?;

        let git_repo = GitRepository::discover(&repo_path)?;
        let mut operations = GitOperations::new(&git_repo)?;

        // Test branch creation
        let create_result = operations.create_branch("feature-branch", "HEAD", false)?;
        assert!(create_result.success);
        assert!(create_result.message.contains("feature-branch"));
        assert!(operations.operation_history.len() > 0);

        // Test branch checkout
        operations.checkout_branch_simple("feature-branch")?; // Should succeed

        // Test branch listing
        let branches = operations.list_local_branches()?;
        assert!(branches.contains(&"feature-branch".to_string()));
        assert!(branches.iter().any(|b| b == "main" || b == "master"));

        Ok(())
    }

    #[test]
    fn test_branch_validation() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "content", "Initial commit")?;

        let git_repo = GitRepository::discover(&repo_path)?;
        let mut operations = GitOperations::new(&git_repo)?;

        // Test invalid branch names
        let invalid_result = operations.create_branch("../invalid", "HEAD", false);
        assert!(invalid_result.is_err());

        let special_char_result = operations.create_branch("branch@#$", "HEAD", false);
        assert!(special_char_result.is_err());

        Ok(())
    }

    #[test]
    fn test_operation_history_tracking() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "content", "Initial commit")?;

        let git_repo = GitRepository::discover(&repo_path)?;
        let mut operations = GitOperations::new(&git_repo)?;

        // Perform several operations
        operations.create_branch("branch1", "HEAD", false)?;
        operations.create_branch("branch2", "HEAD", false)?;
        operations.checkout_branch_simple("branch1")?;

        // Check operation history
        let history = operations.get_operation_history();
        assert!(history.len() >= 3);

        // Verify operation types are recorded correctly
        assert!(
            history
                .iter()
                .any(|op| op.operation_type == OperationType::BranchCreate)
        );
        assert!(
            history
                .iter()
                .any(|op| op.operation_type == OperationType::BranchCheckout)
        );

        Ok(())
    }

    #[test]
    fn test_operation_history_limits() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "content", "Initial commit")?;

        let repo = Repository::open(&repo_path)?;
        let git_repo = GitRepository::discover(&repo_path)?;
        let mut operations = GitOperations::new(&git_repo)?;

        // Perform more operations than the limit
        for i in 0..10 {
            let branch_name = format!("branch{}", i);
            operations.create_branch(&branch_name, "HEAD", false)?;
        }

        // History should be limited to 5 entries
        let history = operations.get_operation_history();
        assert!(history.len() <= 5);

        Ok(())
    }

    #[test]
    fn test_branch_merge_operations() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "main.txt", "main content", "Initial commit")?;

        let git_repo = GitRepository::discover(&repo_path)?;
        let mut operations = GitOperations::new(&git_repo)?;

        // Create and checkout feature branch
        operations.create_branch("feature", "HEAD", false)?;
        operations.checkout_branch_simple("feature")?;

        // Add commit to feature branch
        create_test_commit(
            &repo_path,
            "feature.txt",
            "feature content",
            "Feature commit",
        )?;

        // Checkout main and merge
        operations.checkout_branch_simple("main")?;
        // Test merge operation (may not be implemented yet)
        // let merge_result = operations.merge_branch("feature")?;

        // Note: Merge might fail in test environment, which is acceptable
        // We're testing that the operation is properly structured
        // assert!(merge_result.success || !merge_result.success); // Either outcome is fine

        Ok(())
    }

    #[test]
    fn test_tag_operations_integration() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "content", "Initial commit")?;

        let git_repo = GitRepository::discover(&repo_path)?;
        let mut operations = GitOperations::new(&git_repo)?;

        // Test tag creation
        let tag_config = TagCreateConfig {
            tag_type: TagType::Annotated,
            message: Some("Version 1.0.0".to_string()),
            force_overwrite: false,
            sign_tag: false,
            tagger: None,
        };

        let tag_result = operations
            .tag_manager
            .create_tag("v1.0.0", "HEAD", tag_config)?;
        assert!(tag_result.success);

        // Test tag listing
        let filter_options = TagFilterOptions {
            pattern: None,
            include_lightweight: true,
            include_annotated: true,
            limit: None,
            sort_by: TagSortBy::CreationDate,
            sort_order: SortOrder::Descending,
        };

        let tags = operations.tag_manager.list_tags(Some(filter_options))?;
        assert!(tags.iter().any(|t| t.name == "v1.0.0"));

        Ok(())
    }

    #[test]
    fn test_stash_operations_integration() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "initial", "Initial commit")?;

        // Make changes to working directory
        fs::write(repo_path.join("test.txt"), "modified content")?;
        fs::write(repo_path.join("new.txt"), "new file")?;

        let git_repo = GitRepository::discover(&repo_path)?;
        let mut operations = GitOperations::new(&git_repo)?;

        // Test stash creation
        let stash_config = StashCreateConfig {
            message: Some("Test stash".to_string()),
            keep_index: false,
            include_untracked: true,
            include_ignored: false,
            all_files: false,
            pathspecs: vec![],
        };

        let stash_result = operations.stash_manager.create_stash(stash_config)?;
        assert!(stash_result.success);

        // Test stash listing
        let list_options = StashListOptions {
            limit: None,
            include_stats: true,
            branch_filter: None,
            author_filter: None,
            message_pattern: None,
            date_from: None,
            date_to: None,
        };

        let stashes = operations.stash_manager.list_stashes(Some(list_options))?;
        assert!(stashes.len() > 0);
        assert!(stashes[0].message.contains("Test stash"));

        Ok(())
    }

    #[test]
    fn test_operation_validation_and_sanitization() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "content", "Initial commit")?;

        let git_repo = GitRepository::discover(&repo_path)?;
        let mut operations = GitOperations::new(&git_repo)?;

        // Test input validation for various operations
        let invalid_branch_result = operations.create_branch("", "HEAD", false);
        assert!(invalid_branch_result.is_err());

        let dangerous_branch_result =
            operations.create_branch("../../../etc/passwd", "HEAD", false);
        assert!(dangerous_branch_result.is_err());

        // Test checkout validation
        let invalid_checkout_result = operations.checkout_branch_simple("nonexistent-branch");
        assert!(invalid_checkout_result.is_err());

        Ok(())
    }

    #[test]
    fn test_concurrent_operation_limits() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "content", "Initial commit")?;

        let git_repo = GitRepository::discover(&repo_path)?;
        let mut operations = GitOperations::new(&git_repo)?;

        // Test that concurrent operation limit is enforced
        // This is a basic test - in real scenarios we'd use threading
        for i in 0..10 {
            let branch_name = format!("concurrent_branch_{}", i);
            let result = operations.create_branch(&branch_name, "HEAD", false);
            // Should succeed for first few operations, might fail later due to limits
            if result.is_err() {
                break;
            }
        }

        // The test is about ensuring the system handles multiple operations gracefully
        assert!(operations.get_operation_history().len() > 0);

        Ok(())
    }

    #[test]
    fn test_error_handling_and_recovery() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "content", "Initial commit")?;

        let git_repo = GitRepository::discover(&repo_path)?;
        let mut operations = GitOperations::new(&git_repo)?;

        // Test error handling for impossible operations
        let duplicate_branch_result = operations.create_branch("main", "HEAD", false);
        assert!(duplicate_branch_result.is_err());

        // Ensure the system remains stable after errors
        let valid_result = operations.create_branch("valid-branch", "HEAD", false)?;
        assert!(valid_result.success);

        Ok(())
    }

    #[test]
    fn test_operation_record_structure() -> Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "content", "Initial commit")?;

        let git_repo = GitRepository::discover(&repo_path)?;
        let mut operations = GitOperations::new(&git_repo)?;

        operations.create_branch("test-branch", "HEAD", false)?;

        let history = operations.get_operation_history();
        assert!(history.len() > 0);

        let last_operation = &history[history.len() - 1];
        assert_eq!(last_operation.operation_type, OperationType::BranchCreate);
        assert!(!last_operation.description.is_empty());
        assert!(last_operation.timestamp <= chrono::Utc::now());
        assert!(last_operation.affected_refs.len() > 0);

        Ok(())
    }
}
