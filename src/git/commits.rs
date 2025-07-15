use anyhow::Result;
use git2::{Repository, Oid, ResetType, CherrypickOptions, RevertOptions};
use tracing::{info, warn, error};
use crate::git::{GitRepository, InputValidator, InputSanitizer, ErrorReporter};
use crate::git::operations::{OperationRecord, OperationType};

/// Comprehensive commit operations manager
pub struct CommitOperations {
    repo: Repository,
    operation_history: Vec<OperationRecord>,
}

/// Commit operation result with detailed information
#[derive(Debug)]
pub struct CommitOperationResult {
    pub success: bool,
    pub operation: OperationType,
    pub commit_id: Option<String>,
    pub new_commit_id: Option<String>,
    pub message: String,
    pub conflicts: Vec<String>,
    pub modified_files: Vec<String>,
    pub reverted_files: Vec<String>,
}

/// Configuration for cherry-pick operations
#[derive(Debug, Clone)]
pub struct CherryPickConfig {
    pub mainline: Option<usize>,      // For merge commits, which parent to use
    pub no_commit: bool,              // Stage changes but don't commit
    pub edit_message: bool,           // Allow editing commit message
    pub sign_off: bool,               // Add Signed-off-by line
    pub strategy: MergeStrategy,      // Merge strategy to use
    pub strategy_options: Vec<String>, // Additional strategy options
}

/// Configuration for revert operations
#[derive(Debug, Clone)]
pub struct RevertConfig {
    pub mainline: Option<usize>,      // For merge commits, which parent to use
    pub no_commit: bool,              // Stage changes but don't commit
    pub edit_message: bool,           // Allow editing commit message
    pub sign_off: bool,               // Add Signed-off-by line
    pub strategy: MergeStrategy,      // Merge strategy to use
}

/// Configuration for reset operations
#[derive(Debug, Clone)]
pub struct ResetConfig {
    pub reset_type: GitResetType,     // Type of reset to perform
    pub pathspecs: Vec<String>,       // Specific files to reset (for mixed/soft)
}

/// Git reset types
#[derive(Debug, Clone, PartialEq)]
pub enum GitResetType {
    Soft,    // Move HEAD only, keep index and working tree
    Mixed,   // Move HEAD and reset index, keep working tree
    Hard,    // Move HEAD, reset index and working tree
    Merge,   // Like hard, but safe for merges
    Keep,    // Like hard, but keep local changes
}

/// Merge strategy options
#[derive(Debug, Clone, PartialEq)]
pub enum MergeStrategy {
    Recursive,      // Default 3-way merge
    Resolve,        // Simple 3-way merge
    Octopus,        // For merging more than 2 branches
    Ours,           // Keep our version for conflicts
    Subtree,        // Modified recursive for subtree merges
}

/// Commit conflict information
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub path: String,
    pub ancestor_id: Option<String>,
    pub our_id: Option<String>,
    pub their_id: Option<String>,
    pub conflict_type: ConflictType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    Content,        // Content conflict
    AddAdd,         // Both sides added same file
    DeleteModify,   // One side deleted, other modified
    ModifyDelete,   // One side modified, other deleted
    Rename,         // Rename conflicts
    Mode,           // File mode conflicts
}

impl CommitOperations {
    /// Create a new commit operations manager
    pub fn new(git_repo: &GitRepository) -> Result<Self> {
        let repo_path = git_repo.get_repository().path();
        let repo = Repository::open(repo_path)?;
        
        Ok(Self {
            repo,
            operation_history: Vec::new(),
        })
    }
    
    /// Cherry-pick a commit onto the current branch
    pub fn cherry_pick(&mut self, commit_id: &str, config: CherryPickConfig) -> Result<CommitOperationResult> {
        // Validate input
        if let Err(e) = InputValidator::validate_commit_id(commit_id) {
            ErrorReporter::log_error(&e, "cherry-pick validation");
            return Ok(CommitOperationResult {
                success: false,
                operation: OperationType::CommitCherryPick,
                commit_id: None,
                new_commit_id: None,
                message: format!("Invalid commit ID: {}", e),
                conflicts: vec![],
                modified_files: vec![],
                reverted_files: vec![],
            });
        }
        
        // Sanitize input
        let sanitized_commit = match InputSanitizer::sanitize_commit_id(commit_id) {
            Ok(commit) => commit,
            Err(e) => {
                return Ok(CommitOperationResult {
                    success: false,
                    operation: OperationType::CommitCherryPick,
                    commit_id: None,
                    new_commit_id: None,
                    message: format!("Failed to sanitize commit ID: {}", e),
                    conflicts: vec![],
                    modified_files: vec![],
                    reverted_files: vec![],
                });
            }
        };
        
        // Get current HEAD for operation record
        let original_head = self.repo.head()
            .ok()
            .and_then(|head| head.target())
            .map(|oid| oid.to_string());
        
        // Perform cherry-pick in separate scope
        let cherry_pick_result = {
            let target_oid = match Oid::from_str(&sanitized_commit) {
                Ok(oid) => oid,
                Err(e) => {
                    return Ok(CommitOperationResult {
                        success: false,
                        operation: OperationType::CommitCherryPick,
                        commit_id: Some(sanitized_commit),
                        new_commit_id: None,
                        message: format!("Invalid commit OID: {}", e),
                        conflicts: vec![],
                        modified_files: vec![],
                        reverted_files: vec![],
                    });
                }
            };
            
            let target_commit = match self.repo.find_commit(target_oid) {
                Ok(commit) => commit,
                Err(e) => {
                    return Ok(CommitOperationResult {
                        success: false,
                        operation: OperationType::CommitCherryPick,
                        commit_id: Some(sanitized_commit),
                        new_commit_id: None,
                        message: format!("Commit not found: {}", e),
                        conflicts: vec![],
                        modified_files: vec![],
                        reverted_files: vec![],
                    });
                }
            };
            
            // Set up cherry-pick options
            let mut cherrypick_opts = CherrypickOptions::new();
            self.configure_cherrypick_strategy(&mut cherrypick_opts, &config.strategy);
            
            // Perform the cherry-pick
            match self.repo.cherrypick(&target_commit, Some(&mut cherrypick_opts)) {
                Ok(()) => {
                    // Check for conflicts
                    let conflicts = self.get_conflicts()?;
                    let modified_files = self.get_modified_files()?;
                    
                    if conflicts.is_empty() && !config.no_commit {
                        // Auto-commit if no conflicts and commit is requested
                        match self.create_cherry_pick_commit(&target_commit, &config) {
                            Ok(new_commit_oid) => {
                                (true, Some(new_commit_oid.to_string()), conflicts, modified_files, String::new())
                            }
                            Err(e) => {
                                (false, None, conflicts, modified_files, format!("Failed to create commit: {}", e))
                            }
                        }
                    } else {
                        // Conflicts exist or no-commit requested
                        let status_msg = if !conflicts.is_empty() {
                            format!("Cherry-pick completed with {} conflicts", conflicts.len())
                        } else {
                            "Cherry-pick staged (no commit requested)".to_string()
                        };
                        (true, None, conflicts, modified_files, status_msg)
                    }
                }
                Err(e) => {
                    (false, None, vec![], vec![], format!("Cherry-pick failed: {}", e))
                }
            }
        };
        
        let success = cherry_pick_result.0;
        let new_commit_id = cherry_pick_result.1;
        let conflicts: Vec<String> = cherry_pick_result.2.into_iter().map(|c| c.path).collect();
        let modified_files = cherry_pick_result.3;
        let error_message = cherry_pick_result.4;
        
        // Record the operation
        if success {
            self.record_operation(OperationRecord {
                operation_type: OperationType::CommitCherryPick,
                timestamp: chrono::Utc::now(),
                description: format!("Cherry-picked commit {} -> {}", 
                    &sanitized_commit[..8], 
                    new_commit_id.as_ref().map(|s| &s[..8]).unwrap_or("staged")
                ),
                original_state: original_head,
                new_state: new_commit_id.clone(),
                affected_refs: if new_commit_id.is_some() { vec!["HEAD".to_string()] } else { vec![] },
            });
            
            info!("Cherry-pick successful: {} -> {:?}", sanitized_commit, new_commit_id);
        } else {
            error!("Cherry-pick failed: {}", error_message);
        }
        
        Ok(CommitOperationResult {
            success,
            operation: OperationType::CommitCherryPick,
            commit_id: Some(sanitized_commit),
            new_commit_id,
            message: if success { "Cherry-pick successful".to_string() } else { error_message },
            conflicts,
            modified_files,
            reverted_files: vec![],
        })
    }
    
    /// Revert a commit by creating a reverse commit
    pub fn revert(&mut self, commit_id: &str, config: RevertConfig) -> Result<CommitOperationResult> {
        // Validate input
        if let Err(e) = InputValidator::validate_commit_id(commit_id) {
            ErrorReporter::log_error(&e, "revert validation");
            return Ok(CommitOperationResult {
                success: false,
                operation: OperationType::CommitRevert,
                commit_id: None,
                new_commit_id: None,
                message: format!("Invalid commit ID: {}", e),
                conflicts: vec![],
                modified_files: vec![],
                reverted_files: vec![],
            });
        }
        
        // Sanitize input
        let sanitized_commit = match InputSanitizer::sanitize_commit_id(commit_id) {
            Ok(commit) => commit,
            Err(e) => {
                return Ok(CommitOperationResult {
                    success: false,
                    operation: OperationType::CommitRevert,
                    commit_id: None,
                    new_commit_id: None,
                    message: format!("Failed to sanitize commit ID: {}", e),
                    conflicts: vec![],
                    modified_files: vec![],
                    reverted_files: vec![],
                });
            }
        };
        
        // Get current HEAD for operation record
        let original_head = self.repo.head()
            .ok()
            .and_then(|head| head.target())
            .map(|oid| oid.to_string());
        
        // Perform revert in separate scope
        let revert_result = {
            let target_oid = match Oid::from_str(&sanitized_commit) {
                Ok(oid) => oid,
                Err(e) => {
                    return Ok(CommitOperationResult {
                        success: false,
                        operation: OperationType::CommitRevert,
                        commit_id: Some(sanitized_commit),
                        new_commit_id: None,
                        message: format!("Invalid commit OID: {}", e),
                        conflicts: vec![],
                        modified_files: vec![],
                        reverted_files: vec![],
                    });
                }
            };
            
            let target_commit = match self.repo.find_commit(target_oid) {
                Ok(commit) => commit,
                Err(e) => {
                    return Ok(CommitOperationResult {
                        success: false,
                        operation: OperationType::CommitRevert,
                        commit_id: Some(sanitized_commit),
                        new_commit_id: None,
                        message: format!("Commit not found: {}", e),
                        conflicts: vec![],
                        modified_files: vec![],
                        reverted_files: vec![],
                    });
                }
            };
            
            // Set up revert options
            let mut revert_opts = RevertOptions::new();
            self.configure_revert_strategy(&mut revert_opts, &config.strategy);
            
            // Perform the revert
            match self.repo.revert(&target_commit, Some(&mut revert_opts)) {
                Ok(()) => {
                    // Check for conflicts
                    let conflicts = self.get_conflicts()?;
                    let modified_files = self.get_modified_files()?;
                    let reverted_files = self.get_reverted_files(&target_commit)?;
                    
                    if conflicts.is_empty() && !config.no_commit {
                        // Auto-commit if no conflicts and commit is requested
                        match self.create_revert_commit(&target_commit, &config) {
                            Ok(new_commit_oid) => {
                                (true, Some(new_commit_oid.to_string()), conflicts, modified_files, reverted_files, String::new())
                            }
                            Err(e) => {
                                (false, None, conflicts, modified_files, reverted_files, format!("Failed to create commit: {}", e))
                            }
                        }
                    } else {
                        // Conflicts exist or no-commit requested
                        let status_msg = if !conflicts.is_empty() {
                            format!("Revert completed with {} conflicts", conflicts.len())
                        } else {
                            "Revert staged (no commit requested)".to_string()
                        };
                        (true, None, conflicts, modified_files, reverted_files, status_msg)
                    }
                }
                Err(e) => {
                    (false, None, vec![], vec![], vec![], format!("Revert failed: {}", e))
                }
            }
        };
        
        let success = revert_result.0;
        let new_commit_id = revert_result.1;
        let conflicts: Vec<String> = revert_result.2.into_iter().map(|c| c.path).collect();
        let modified_files = revert_result.3;
        let reverted_files = revert_result.4;
        let error_message = revert_result.5;
        
        // Record the operation
        if success {
            self.record_operation(OperationRecord {
                operation_type: OperationType::CommitRevert,
                timestamp: chrono::Utc::now(),
                description: format!("Reverted commit {} -> {}", 
                    &sanitized_commit[..8], 
                    new_commit_id.as_ref().map(|s| &s[..8]).unwrap_or("staged")
                ),
                original_state: original_head,
                new_state: new_commit_id.clone(),
                affected_refs: if new_commit_id.is_some() { vec!["HEAD".to_string()] } else { vec![] },
            });
            
            info!("Revert successful: {} -> {:?}", sanitized_commit, new_commit_id);
        } else {
            error!("Revert failed: {}", error_message);
        }
        
        Ok(CommitOperationResult {
            success,
            operation: OperationType::CommitRevert,
            commit_id: Some(sanitized_commit),
            new_commit_id,
            message: if success { "Revert successful".to_string() } else { error_message },
            conflicts,
            modified_files,
            reverted_files,
        })
    }
    
    /// Reset the current branch to a specific commit
    pub fn reset(&mut self, commit_id: &str, config: ResetConfig) -> Result<CommitOperationResult> {
        // Validate input
        if let Err(e) = InputValidator::validate_commit_id(commit_id) {
            ErrorReporter::log_error(&e, "reset validation");
            return Ok(CommitOperationResult {
                success: false,
                operation: OperationType::CommitReset,
                commit_id: None,
                new_commit_id: None,
                message: format!("Invalid commit ID: {}", e),
                conflicts: vec![],
                modified_files: vec![],
                reverted_files: vec![],
            });
        }
        
        // Sanitize input
        let sanitized_commit = match InputSanitizer::sanitize_commit_id(commit_id) {
            Ok(commit) => commit,
            Err(e) => {
                return Ok(CommitOperationResult {
                    success: false,
                    operation: OperationType::CommitReset,
                    commit_id: None,
                    new_commit_id: None,
                    message: format!("Failed to sanitize commit ID: {}", e),
                    conflicts: vec![],
                    modified_files: vec![],
                    reverted_files: vec![],
                });
            }
        };
        
        // Get current HEAD for operation record
        let original_head = self.repo.head()
            .ok()
            .and_then(|head| head.target())
            .map(|oid| oid.to_string());
        
        // Perform reset in separate scope
        let reset_result = {
            let target_oid = match Oid::from_str(&sanitized_commit) {
                Ok(oid) => oid,
                Err(e) => {
                    return Ok(CommitOperationResult {
                        success: false,
                        operation: OperationType::CommitReset,
                        commit_id: Some(sanitized_commit),
                        new_commit_id: None,
                        message: format!("Invalid commit OID: {}", e),
                        conflicts: vec![],
                        modified_files: vec![],
                        reverted_files: vec![],
                    });
                }
            };
            
            let target_commit = match self.repo.find_commit(target_oid) {
                Ok(commit) => commit,
                Err(e) => {
                    return Ok(CommitOperationResult {
                        success: false,
                        operation: OperationType::CommitReset,
                        commit_id: Some(sanitized_commit),
                        new_commit_id: None,
                        message: format!("Commit not found: {}", e),
                        conflicts: vec![],
                        modified_files: vec![],
                        reverted_files: vec![],
                    });
                }
            };
            
            // Convert reset type
            let reset_type = match config.reset_type {
                GitResetType::Soft => ResetType::Soft,
                GitResetType::Mixed => ResetType::Mixed,
                GitResetType::Hard => ResetType::Hard,
                _ => ResetType::Mixed, // Fallback for unsupported types
            };
            
            // Perform the reset
            match self.repo.reset(target_commit.as_object(), reset_type, None) {
                Ok(()) => {
                    let modified_files = self.get_modified_files()?;
                    (true, target_oid.to_string(), modified_files, String::new())
                }
                Err(e) => {
                    (false, String::new(), vec![], format!("Reset failed: {}", e))
                }
            }
        };
        
        let success = reset_result.0;
        let new_commit_id = if success { Some(reset_result.1) } else { None };
        let modified_files = reset_result.2;
        let error_message = reset_result.3;
        
        // Record the operation
        if success {
            self.record_operation(OperationRecord {
                operation_type: OperationType::CommitReset,
                timestamp: chrono::Utc::now(),
                description: format!("Reset to commit {} ({})", 
                    &sanitized_commit[..8], 
                    format!("{:?}", config.reset_type).to_lowercase()
                ),
                original_state: original_head,
                new_state: new_commit_id.clone(),
                affected_refs: vec!["HEAD".to_string()],
            });
            
            info!("Reset successful to commit: {}", sanitized_commit);
        } else {
            error!("Reset failed: {}", error_message);
        }
        
        Ok(CommitOperationResult {
            success,
            operation: OperationType::CommitReset,
            commit_id: Some(sanitized_commit),
            new_commit_id,
            message: if success { format!("Reset successful ({:?})", config.reset_type) } else { error_message },
            conflicts: vec![],
            modified_files,
            reverted_files: vec![],
        })
    }
    
    /// Get current conflicts in the repository
    fn get_conflicts(&self) -> Result<Vec<ConflictInfo>> {
        let mut conflicts = Vec::new();
        
        let index = self.repo.index()?;
        let conflict_iter = index.conflicts()?;
        
        for conflict in conflict_iter {
            if let Ok(conflict_data) = conflict {
                let path = conflict_data.ancestor.as_ref()
                    .or(conflict_data.our.as_ref())
                    .or(conflict_data.their.as_ref())
                    .and_then(|entry| std::str::from_utf8(&entry.path).ok())
                    .unwrap_or("unknown")
                    .to_string();
                
                let conflict_info = ConflictInfo {
                    path,
                    ancestor_id: conflict_data.ancestor.as_ref().map(|e| e.id.to_string()),
                    our_id: conflict_data.our.as_ref().map(|e| e.id.to_string()),
                    their_id: conflict_data.their.as_ref().map(|e| e.id.to_string()),
                    conflict_type: self.classify_conflict_type(&conflict_data.ancestor, &conflict_data.our, &conflict_data.their),
                };
                
                conflicts.push(conflict_info);
            }
        }
        
        Ok(conflicts)
    }
    
    /// Get list of modified files
    fn get_modified_files(&self) -> Result<Vec<String>> {
        let mut modified_files = Vec::new();
        
        let statuses = self.repo.statuses(None)?;
        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                modified_files.push(path.to_string());
            }
        }
        
        Ok(modified_files)
    }
    
    /// Get list of files that were reverted
    fn get_reverted_files(&self, reverted_commit: &git2::Commit) -> Result<Vec<String>> {
        let mut reverted_files = Vec::new();
        
        // Get the diff between the reverted commit and its parent
        let parent = reverted_commit.parent(0)?;
        let parent_tree = parent.tree()?;
        let commit_tree = reverted_commit.tree()?;
        
        let diff = self.repo.diff_tree_to_tree(
            Some(&parent_tree),
            Some(&commit_tree),
            None,
        )?;
        
        diff.foreach(
            &mut |diff_delta, _progress| {
                if let Some(path) = diff_delta.old_file().path() {
                    if let Some(path_str) = path.to_str() {
                        reverted_files.push(path_str.to_string());
                    }
                }
                true
            },
            None,
            None,
            None,
        )?;
        
        Ok(reverted_files)
    }
    
    /// Configure cherry-pick strategy options
    fn configure_cherrypick_strategy(&self, _cherrypick_opts: &mut CherrypickOptions, strategy: &MergeStrategy) {
        match strategy {
            MergeStrategy::Recursive => {
                // Default strategy - no special configuration needed
            }
            MergeStrategy::Resolve => {
                // Simple 3-way merge - limited configuration in git2
            }
            MergeStrategy::Ours => {
                // Favor our side for conflicts - limited configuration in git2
            }
            _ => {
                // Other strategies not directly supported by git2
                warn!("Merge strategy {:?} not fully supported, using default", strategy);
            }
        }
    }
    
    /// Configure revert strategy options
    fn configure_revert_strategy(&self, _revert_opts: &mut RevertOptions, strategy: &MergeStrategy) {
        match strategy {
            MergeStrategy::Recursive => {
                // Default strategy - no special configuration needed
            }
            MergeStrategy::Resolve => {
                // Simple 3-way merge - limited configuration in git2
            }
            MergeStrategy::Ours => {
                // Favor our side for conflicts - limited configuration in git2
            }
            _ => {
                // Other strategies not directly supported by git2
                warn!("Merge strategy {:?} not fully supported, using default", strategy);
            }
        }
    }
    
    /// Create commit for cherry-pick operation
    fn create_cherry_pick_commit(&self, original_commit: &git2::Commit, config: &CherryPickConfig) -> Result<Oid> {
        let signature = self.repo.signature()?;
        let tree_id = self.repo.index()?.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let head = self.repo.head()?.target().unwrap();
        let parent = self.repo.find_commit(head)?;
        
        let mut message = original_commit.message().unwrap_or("Cherry-picked commit").to_string();
        if config.sign_off {
            message.push_str(&format!("\n\nSigned-off-by: {} <{}>", 
                signature.name().unwrap_or("Unknown"),
                signature.email().unwrap_or("unknown@example.com")
            ));
        }
        
        self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &[&parent],
        ).map_err(Into::into)
    }
    
    /// Create commit for revert operation  
    fn create_revert_commit(&self, original_commit: &git2::Commit, config: &RevertConfig) -> Result<Oid> {
        let signature = self.repo.signature()?;
        let tree_id = self.repo.index()?.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let head = self.repo.head()?.target().unwrap();
        let parent = self.repo.find_commit(head)?;
        
        let original_message = original_commit.message().unwrap_or("Unknown commit");
        let mut message = format!("Revert \"{}\"\n\nThis reverts commit {}.", 
            original_message.lines().next().unwrap_or("Unknown commit"),
            original_commit.id()
        );
        
        if config.sign_off {
            message.push_str(&format!("\n\nSigned-off-by: {} <{}>", 
                signature.name().unwrap_or("Unknown"),
                signature.email().unwrap_or("unknown@example.com")
            ));
        }
        
        self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &[&parent],
        ).map_err(Into::into)
    }
    
    /// Classify the type of conflict
    fn classify_conflict_type(
        &self,
        ancestor: &Option<git2::IndexEntry>,
        our: &Option<git2::IndexEntry>,
        their: &Option<git2::IndexEntry>
    ) -> ConflictType {
        match (ancestor.is_some(), our.is_some(), their.is_some()) {
            (false, true, true) => ConflictType::AddAdd,
            (true, false, true) => ConflictType::DeleteModify,
            (true, true, false) => ConflictType::ModifyDelete,
            (true, true, true) => {
                // Check if it's a mode conflict
                if let (Some(our_entry), Some(their_entry)) = (our, their) {
                    if our_entry.mode != their_entry.mode {
                        ConflictType::Mode
                    } else {
                        ConflictType::Content
                    }
                } else {
                    ConflictType::Content
                }
            }
            _ => ConflictType::Content,
        }
    }
    
    /// Record an operation in the history
    fn record_operation(&mut self, record: OperationRecord) {
        let operation_type = record.operation_type.clone();
        self.operation_history.push(record);
        
        // Maintain history limit
        if self.operation_history.len() > 100 {
            self.operation_history.remove(0);
        }
        
        info!("Recorded commit operation: {:?}", operation_type);
    }
    
    /// Get operation history
    pub fn get_operation_history(&self) -> &[OperationRecord] {
        &self.operation_history
    }
    
    /// Check if repository has uncommitted changes
    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        let statuses = self.repo.statuses(None)?;
        Ok(!statuses.is_empty())
    }
    
    /// Get repository state (normal, merging, reverting, etc.)
    pub fn get_repository_state(&self) -> git2::RepositoryState {
        self.repo.state()
    }
    
    /// Abort current operation (merge, cherry-pick, revert)
    pub fn abort_operation(&mut self) -> Result<CommitOperationResult> {
        let repo_state = self.repo.state();
        
        match repo_state {
            git2::RepositoryState::CherryPickSequence => {
                // Abort cherry-pick - git2 doesn't have cherrypick_abort, so we reset
                let head_commit = self.repo.find_commit(self.repo.head()?.target().unwrap())?;
                match self.repo.reset(head_commit.as_object(), ResetType::Hard, None) {
                    Ok(()) => {
                        info!("Cherry-pick aborted successfully");
                        Ok(CommitOperationResult {
                            success: true,
                            operation: OperationType::CommitCherryPick,
                            commit_id: None,
                            new_commit_id: None,
                            message: "Cherry-pick aborted".to_string(),
                            conflicts: vec![],
                            modified_files: vec![],
                            reverted_files: vec![],
                        })
                    }
                    Err(e) => {
                        error!("Failed to abort cherry-pick: {}", e);
                        Ok(CommitOperationResult {
                            success: false,
                            operation: OperationType::CommitCherryPick,
                            commit_id: None,
                            new_commit_id: None,
                            message: format!("Failed to abort cherry-pick: {}", e),
                            conflicts: vec![],
                            modified_files: vec![],
                            reverted_files: vec![],
                        })
                    }
                }
            }
            git2::RepositoryState::RevertSequence => {
                // Abort revert - git2 doesn't have revert_abort, so we reset
                let head_commit = self.repo.find_commit(self.repo.head()?.target().unwrap())?;
                match self.repo.reset(head_commit.as_object(), ResetType::Hard, None) {
                    Ok(()) => {
                        info!("Revert aborted successfully");
                        Ok(CommitOperationResult {
                            success: true,
                            operation: OperationType::CommitRevert,
                            commit_id: None,
                            new_commit_id: None,
                            message: "Revert aborted".to_string(),
                            conflicts: vec![],
                            modified_files: vec![],
                            reverted_files: vec![],
                        })
                    }
                    Err(e) => {
                        error!("Failed to abort revert: {}", e);
                        Ok(CommitOperationResult {
                            success: false,
                            operation: OperationType::CommitRevert,
                            commit_id: None,
                            new_commit_id: None,
                            message: format!("Failed to abort revert: {}", e),
                            conflicts: vec![],
                            modified_files: vec![],
                            reverted_files: vec![],
                        })
                    }
                }
            }
            _ => {
                Ok(CommitOperationResult {
                    success: false,
                    operation: OperationType::CommitReset,
                    commit_id: None,
                    new_commit_id: None,
                    message: "No operation to abort".to_string(),
                    conflicts: vec![],
                    modified_files: vec![],
                    reverted_files: vec![],
                })
            }
        }
    }
}

impl Default for CherryPickConfig {
    fn default() -> Self {
        Self {
            mainline: None,
            no_commit: false,
            edit_message: false,
            sign_off: false,
            strategy: MergeStrategy::Recursive,
            strategy_options: vec![],
        }
    }
}

impl Default for RevertConfig {
    fn default() -> Self {
        Self {
            mainline: None,
            no_commit: false,
            edit_message: false,
            sign_off: false,
            strategy: MergeStrategy::Recursive,
        }
    }
}

impl Default for ResetConfig {
    fn default() -> Self {
        Self {
            reset_type: GitResetType::Mixed,
            pathspecs: vec![],
        }
    }
}