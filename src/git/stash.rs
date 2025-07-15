use anyhow::Result;
use git2::{Repository, Signature, StashFlags, StashApplyOptions};
use tracing::{info, warn, error};
use crate::git::{GitRepository, InputValidator, InputSanitizer, ErrorReporter};
use crate::git::operations::{OperationRecord, OperationType};

/// Comprehensive stash management system
pub struct StashManager {
    repo: Repository,
    operation_history: Vec<OperationRecord>,
}

/// Stash operation result with detailed information
#[derive(Debug)]
pub struct StashOperationResult {
    pub success: bool,
    pub operation: OperationType,
    pub stash_index: Option<usize>,
    pub stash_id: Option<String>,
    pub message: String,
    pub conflicts: Vec<String>,
    pub modified_files: Vec<String>,
    pub stash_info: Option<StashInfo>,
}

/// Detailed information about a stash entry
#[derive(Debug, Clone)]
pub struct StashInfo {
    pub index: usize,
    pub id: String,
    pub message: String,
    pub author: StashAuthor,
    pub created_date: chrono::DateTime<chrono::Utc>,
    pub branch_name: Option<String>,
    pub has_untracked: bool,
    pub has_ignored: bool,
    pub file_count: usize,
    pub description: String,
}

/// Stash author information
#[derive(Debug, Clone)]
pub struct StashAuthor {
    pub name: String,
    pub email: String,
}

/// Configuration for stash creation
#[derive(Debug, Clone)]
pub struct StashCreateConfig {
    pub message: Option<String>,
    pub keep_index: bool,           // Keep staged changes in index
    pub include_untracked: bool,    // Include untracked files
    pub include_ignored: bool,      // Include ignored files
    pub all_files: bool,            // Include all files (untracked + ignored)
    pub pathspecs: Vec<String>,     // Specific files/patterns to stash
}

/// Configuration for stash application
#[derive(Debug, Clone)]
pub struct StashApplyConfig {
    pub check_conflicts: bool,      // Check for conflicts before applying
    pub reinstate_index: bool,      // Restore staged changes to index
    pub ignore_whitespace: bool,    // Ignore whitespace changes
    pub strategy: StashApplyStrategy,
}

/// Stash application strategy
#[derive(Debug, Clone, PartialEq)]
pub enum StashApplyStrategy {
    Normal,        // Standard 3-way merge
    Ours,          // Keep our changes for conflicts
    Theirs,        // Keep stash changes for conflicts
    IgnoreSpace,   // Ignore whitespace differences
}

/// Stash listing and filtering options
#[derive(Debug, Clone)]
pub struct StashListOptions {
    pub limit: Option<usize>,
    pub include_stats: bool,        // Include file change statistics
    pub branch_filter: Option<String>, // Filter by branch name
    pub author_filter: Option<String>, // Filter by author
    pub message_pattern: Option<String>, // Pattern match in message
    pub date_from: Option<chrono::DateTime<chrono::Utc>>,
    pub date_to: Option<chrono::DateTime<chrono::Utc>>,
}

/// Stash conflict information
#[derive(Debug, Clone)]
pub struct StashConflict {
    pub path: String,
    pub conflict_type: StashConflictType,
    pub our_content: Option<String>,
    pub their_content: Option<String>,
    pub base_content: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StashConflictType {
    Content,        // Content conflict
    FileMode,       // File permission conflict
    DeleteModify,   // File deleted vs modified
    AddAdd,         // Same file added differently
    Rename,         // Rename conflict
}

impl StashManager {
    /// Create a new stash manager
    pub fn new(git_repo: &GitRepository) -> Result<Self> {
        let repo_path = git_repo.get_repository().path();
        let repo = Repository::open(repo_path)?;
        
        Ok(Self {
            repo,
            operation_history: Vec::new(),
        })
    }
    
    /// Create a new stash with the current changes
    pub fn create_stash(&mut self, config: StashCreateConfig) -> Result<StashOperationResult> {
        // Check if there are changes to stash
        if !self.has_changes_to_stash(&config)? {
            return Ok(StashOperationResult {
                success: false,
                operation: OperationType::StashSave,
                stash_index: None,
                stash_id: None,
                message: "No changes to stash".to_string(),
                conflicts: vec![],
                modified_files: vec![],
                stash_info: None,
            });
        }
        
        // Get current branch for stash info
        let current_branch = self.get_current_branch_name();
        
        // Validate message if provided
        let message = if let Some(ref msg) = config.message {
            if let Err(e) = InputValidator::validate_commit_message(msg) {
                ErrorReporter::log_error(&e, "stash message validation");
                return Ok(StashOperationResult {
                    success: false,
                    operation: OperationType::StashSave,
                    stash_index: None,
                    stash_id: None,
                    message: format!("Invalid stash message: {}", e),
                    conflicts: vec![],
                    modified_files: vec![],
                    stash_info: None,
                });
            }
            
            match InputSanitizer::sanitize_commit_message(msg) {
                Ok(sanitized) => sanitized,
                Err(e) => {
                    return Ok(StashOperationResult {
                        success: false,
                        operation: OperationType::StashSave,
                        stash_index: None,
                        stash_id: None,
                        message: format!("Failed to sanitize stash message: {}", e),
                        conflicts: vec![],
                        modified_files: vec![],
                        stash_info: None,
                    });
                }
            }
        } else {
            format!("WIP on {}: {}", 
                current_branch.unwrap_or_else(|| "detached".to_string()),
                self.get_latest_commit_summary().unwrap_or_else(|| "no commits".to_string())
            )
        };
        
        // Get list of files that will be stashed
        let modified_files = self.get_files_to_stash(&config)?;
        
        // Create stash flags based on configuration
        let mut flags = StashFlags::DEFAULT;
        if config.keep_index {
            flags |= StashFlags::KEEP_INDEX;
        }
        if config.include_untracked {
            flags |= StashFlags::INCLUDE_UNTRACKED;
        }
        if config.include_ignored {
            flags |= StashFlags::INCLUDE_IGNORED;
        }
        if config.all_files {
            flags |= StashFlags::INCLUDE_UNTRACKED | StashFlags::INCLUDE_IGNORED;
        }
        
        // Create the stash
        let stash_result = {
            let signature = match self.repo.signature() {
                Ok(sig) => sig,
                Err(_) => {
                    // Fallback signature
                    match Signature::now("Git User", "user@example.com") {
                        Ok(sig) => sig,
                        Err(e) => {
                            return Ok(StashOperationResult {
                                success: false,
                                operation: OperationType::StashSave,
                                stash_index: None,
                                stash_id: None,
                                message: format!("Failed to create signature: {}", e),
                                conflicts: vec![],
                                modified_files: vec![],
                                stash_info: None,
                            });
                        }
                    }
                }
            };
            
            // Perform the stash operation
            match self.repo.stash_save(&signature, &message, Some(flags)) {
                Ok(stash_oid) => (true, Some(stash_oid), String::new()),
                Err(e) => (false, None, format!("Failed to create stash: {}", e)),
            }
        };
        
        if !stash_result.0 {
            error!("Stash creation failed: {}", stash_result.2);
            return Ok(StashOperationResult {
                success: false,
                operation: OperationType::StashSave,
                stash_index: None,
                stash_id: None,
                message: stash_result.2,
                conflicts: vec![],
                modified_files: vec![],
                stash_info: None,
            });
        }
        
        let stash_oid = stash_result.1.unwrap();
        let stash_id = stash_oid.to_string();
        
        // Get stash info for the newly created stash
        let stash_info = self.get_stash_info_by_index(0)?;
        
        // Record the operation
        self.record_operation(OperationRecord {
            operation_type: OperationType::StashSave,
            timestamp: chrono::Utc::now(),
            description: format!("Created stash: {}", message),
            original_state: None,
            new_state: Some(stash_id.clone()),
            affected_refs: vec!["refs/stash".to_string()],
        });
        
        info!("Successfully created stash: {} ({})", message, &stash_id[..8]);
        
        Ok(StashOperationResult {
            success: true,
            operation: OperationType::StashSave,
            stash_index: Some(0),
            stash_id: Some(stash_id),
            message: "Stash created successfully".to_string(),
            conflicts: vec![],
            modified_files,
            stash_info: Some(stash_info),
        })
    }
    
    /// Apply a stash to the current working directory
    pub fn apply_stash(&mut self, stash_index: usize, config: StashApplyConfig) -> Result<StashOperationResult> {
        // Validate stash index
        let stash_count = self.get_stash_count()?;
        if stash_index >= stash_count {
            return Ok(StashOperationResult {
                success: false,
                operation: OperationType::StashApply,
                stash_index: Some(stash_index),
                stash_id: None,
                message: format!("Stash index {} out of range (0-{})", stash_index, stash_count.saturating_sub(1)),
                conflicts: vec![],
                modified_files: vec![],
                stash_info: None,
            });
        }
        
        // Get stash info before applying
        let stash_info = self.get_stash_info_by_index(stash_index)?;
        
        // Check for conflicts if requested
        if config.check_conflicts {
            if let Some(conflicts) = self.check_stash_conflicts(stash_index)? {
                if !conflicts.is_empty() {
                    return Ok(StashOperationResult {
                        success: false,
                        operation: OperationType::StashApply,
                        stash_index: Some(stash_index),
                        stash_id: Some(stash_info.id.clone()),
                        message: format!("Applying stash would cause {} conflicts", conflicts.len()),
                        conflicts: conflicts.into_iter().map(|c| c.path).collect(),
                        modified_files: vec![],
                        stash_info: Some(stash_info),
                    });
                }
            }
        }
        
        // Set up apply options
        let mut apply_opts = StashApplyOptions::new();
        if config.reinstate_index {
            apply_opts.reinstantiate_index();
        }
        
        // Configure strategy (limited configuration available in git2)
        match config.strategy {
            StashApplyStrategy::Normal => {
                // Default behavior
            }
            StashApplyStrategy::Ours => {
                // Limited support in git2 for conflict resolution strategy
            }
            StashApplyStrategy::Theirs => {
                // Limited support in git2 for conflict resolution strategy
            }
            StashApplyStrategy::IgnoreSpace => {
                // Limited support in git2 for whitespace handling
            }
        }
        
        // Apply the stash
        let apply_result = {
            match self.repo.stash_apply(stash_index, Some(&mut apply_opts)) {
                Ok(()) => {
                    let conflicts = self.get_current_conflicts()?;
                    let modified_files = self.get_modified_files()?;
                    (true, conflicts, modified_files, String::new())
                }
                Err(e) => {
                    (false, vec![], vec![], format!("Failed to apply stash: {}", e))
                }
            }
        };
        
        let success = apply_result.0;
        let conflicts = apply_result.1;
        let modified_files = apply_result.2;
        let error_message = apply_result.3;
        
        // Record the operation
        if success {
            self.record_operation(OperationRecord {
                operation_type: OperationType::StashApply,
                timestamp: chrono::Utc::now(),
                description: format!("Applied stash {}: {}", stash_index, stash_info.message),
                original_state: None,
                new_state: Some(stash_info.id.clone()),
                affected_refs: vec!["HEAD".to_string()],
            });
            
            info!("Successfully applied stash {}: {}", stash_index, stash_info.message);
        } else {
            error!("Failed to apply stash {}: {}", stash_index, error_message);
        }
        
        Ok(StashOperationResult {
            success,
            operation: OperationType::StashApply,
            stash_index: Some(stash_index),
            stash_id: Some(stash_info.id.clone()),
            message: if success { 
                if conflicts.is_empty() { 
                    "Stash applied successfully".to_string() 
                } else { 
                    format!("Stash applied with {} conflicts", conflicts.len()) 
                }
            } else { 
                error_message 
            },
            conflicts: conflicts.into_iter().map(|c| c.path).collect(),
            modified_files,
            stash_info: Some(stash_info),
        })
    }
    
    /// Pop (apply and remove) a stash
    pub fn pop_stash(&mut self, stash_index: usize, config: StashApplyConfig) -> Result<StashOperationResult> {
        // First apply the stash
        let apply_result = self.apply_stash(stash_index, config)?;
        
        if !apply_result.success {
            // Return apply failure result with pop operation type
            return Ok(StashOperationResult {
                operation: OperationType::StashPop,
                ..apply_result
            });
        }
        
        // If apply was successful, remove the stash
        let drop_result = self.drop_stash(stash_index)?;
        
        if !drop_result.success {
            warn!("Stash applied but failed to remove: {}", drop_result.message);
            return Ok(StashOperationResult {
                operation: OperationType::StashPop,
                message: format!("Stash applied but failed to remove: {}", drop_result.message),
                ..apply_result
            });
        }
        
        // Update operation record for successful pop
        if let Some(last_op) = self.operation_history.last_mut() {
            last_op.operation_type = OperationType::StashPop;
            last_op.description = format!("Popped stash {}: {}", 
                stash_index, 
                apply_result.stash_info.as_ref().map(|s| &s.message).unwrap_or(&"unknown".to_string())
            );
        }
        
        Ok(StashOperationResult {
            operation: OperationType::StashPop,
            message: "Stash popped successfully".to_string(),
            ..apply_result
        })
    }
    
    /// Drop (remove) a stash without applying it
    pub fn drop_stash(&mut self, stash_index: usize) -> Result<StashOperationResult> {
        // Validate stash index
        let stash_count = self.get_stash_count()?;
        if stash_index >= stash_count {
            return Ok(StashOperationResult {
                success: false,
                operation: OperationType::StashDrop,
                stash_index: Some(stash_index),
                stash_id: None,
                message: format!("Stash index {} out of range (0-{})", stash_index, stash_count.saturating_sub(1)),
                conflicts: vec![],
                modified_files: vec![],
                stash_info: None,
            });
        }
        
        // Get stash info before dropping
        let stash_info = self.get_stash_info_by_index(stash_index)?;
        
        // Drop the stash
        let drop_result = {
            match self.repo.stash_drop(stash_index) {
                Ok(()) => (true, String::new()),
                Err(e) => (false, format!("Failed to drop stash: {}", e)),
            }
        };
        
        let success = drop_result.0;
        let error_message = drop_result.1;
        
        // Record the operation
        if success {
            self.record_operation(OperationRecord {
                operation_type: OperationType::StashDrop,
                timestamp: chrono::Utc::now(),
                description: format!("Dropped stash {}: {}", stash_index, stash_info.message),
                original_state: Some(stash_info.id.clone()),
                new_state: None,
                affected_refs: vec!["refs/stash".to_string()],
            });
            
            info!("Successfully dropped stash {}: {}", stash_index, stash_info.message);
        } else {
            error!("Failed to drop stash {}: {}", stash_index, error_message);
        }
        
        Ok(StashOperationResult {
            success,
            operation: OperationType::StashDrop,
            stash_index: Some(stash_index),
            stash_id: Some(stash_info.id.clone()),
            message: if success { "Stash dropped successfully".to_string() } else { error_message },
            conflicts: vec![],
            modified_files: vec![],
            stash_info: Some(stash_info),
        })
    }
    
    /// List all stashes with optional filtering
    pub fn list_stashes(&self, options: Option<StashListOptions>) -> Result<Vec<StashInfo>> {
        let options = options.unwrap_or_default();
        let mut stashes = Vec::new();
        
        let stash_count = self.get_stash_count()?;
        if stash_count == 0 {
            return Ok(stashes);
        }
        
        // Iterate through stashes
        for index in 0..stash_count {
            if let Some(limit) = options.limit {
                if stashes.len() >= limit {
                    break;
                }
            }
            
            if let Ok(stash_info) = self.get_stash_info_by_index(index) {
                // Apply filters
                if self.matches_filters(&stash_info, &options) {
                    stashes.push(stash_info);
                }
            }
        }
        
        Ok(stashes)
    }
    
    /// Get detailed information about a specific stash
    pub fn get_stash_info_by_index(&self, index: usize) -> Result<StashInfo> {
        // Get stash commit
        let stash_ref = format!("stash@{{{}}}", index);
        let stash_commit = self.repo.revparse_single(&stash_ref)?.into_commit()
            .map_err(|_| anyhow::anyhow!("Stash is not a commit"))?;
        
        // Extract author information
        let author = stash_commit.author();
        let stash_author = StashAuthor {
            name: author.name().unwrap_or("Unknown").to_string(),
            email: author.email().unwrap_or("unknown@example.com").to_string(),
        };
        
        // Get stash message
        let message = stash_commit.message().unwrap_or("No message").to_string();
        
        // Try to extract branch name from message
        let branch_name = self.extract_branch_from_message(&message);
        
        // Get file statistics
        let file_count = self.get_stash_file_count(index)?;
        
        // Create description
        let description = format!("Stash@{{{}}}: {}", index, message);
        
        Ok(StashInfo {
            index,
            id: stash_commit.id().to_string(),
            message,
            author: stash_author,
            created_date: chrono::DateTime::from_timestamp(author.when().seconds(), 0)
                .unwrap_or_else(chrono::Utc::now),
            branch_name,
            has_untracked: self.stash_has_untracked(index)?,
            has_ignored: self.stash_has_ignored(index)?,
            file_count,
            description,
        })
    }
    
    /// Clear all stashes
    pub fn clear_all_stashes(&mut self) -> Result<StashOperationResult> {
        let stash_count = self.get_stash_count()?;
        if stash_count == 0 {
            return Ok(StashOperationResult {
                success: true,
                operation: OperationType::StashDrop,
                stash_index: None,
                stash_id: None,
                message: "No stashes to clear".to_string(),
                conflicts: vec![],
                modified_files: vec![],
                stash_info: None,
            });
        }
        
        // Drop all stashes from highest index to lowest to maintain indices
        let mut dropped_count = 0;
        for index in (0..stash_count).rev() {
            if let Ok(result) = self.drop_stash(index) {
                if result.success {
                    dropped_count += 1;
                }
            }
        }
        
        // Record the operation
        self.record_operation(OperationRecord {
            operation_type: OperationType::StashDrop,
            timestamp: chrono::Utc::now(),
            description: format!("Cleared {} stashes", dropped_count),
            original_state: Some(format!("{} stashes", stash_count)),
            new_state: Some("0 stashes".to_string()),
            affected_refs: vec!["refs/stash".to_string()],
        });
        
        info!("Successfully cleared {} stashes", dropped_count);
        
        Ok(StashOperationResult {
            success: true,
            operation: OperationType::StashDrop,
            stash_index: None,
            stash_id: None,
            message: format!("Cleared {} stashes", dropped_count),
            conflicts: vec![],
            modified_files: vec![],
            stash_info: None,
        })
    }
    
    /// Check if there are changes that can be stashed
    fn has_changes_to_stash(&self, config: &StashCreateConfig) -> Result<bool> {
        let statuses = self.repo.statuses(None)?;
        
        for entry in statuses.iter() {
            let status = entry.status();
            
            // Check for modified/staged files
            if status.is_wt_modified() || status.is_index_modified() || 
               status.is_wt_deleted() || status.is_index_deleted() ||
               status.is_wt_renamed() || status.is_index_renamed() {
                return Ok(true);
            }
            
            // Check for untracked files if configured
            if config.include_untracked && status.is_wt_new() {
                return Ok(true);
            }
            
            // Check for ignored files if configured
            if config.include_ignored && status.is_ignored() {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// Get list of files that will be stashed
    fn get_files_to_stash(&self, config: &StashCreateConfig) -> Result<Vec<String>> {
        let mut files = Vec::new();
        let statuses = self.repo.statuses(None)?;
        
        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let status = entry.status();
                let mut include = false;
                
                // Include modified/staged files
                if status.is_wt_modified() || status.is_index_modified() || 
                   status.is_wt_deleted() || status.is_index_deleted() ||
                   status.is_wt_renamed() || status.is_index_renamed() {
                    include = true;
                }
                
                // Include untracked files if configured
                if config.include_untracked && status.is_wt_new() {
                    include = true;
                }
                
                // Include ignored files if configured
                if config.include_ignored && status.is_ignored() {
                    include = true;
                }
                
                if include {
                    files.push(path.to_string());
                }
            }
        }
        
        Ok(files)
    }
    
    /// Get current branch name
    fn get_current_branch_name(&self) -> Option<String> {
        self.repo.head()
            .ok()
            .and_then(|head| head.shorthand().map(|s| s.to_string()))
    }
    
    /// Get latest commit summary
    fn get_latest_commit_summary(&self) -> Option<String> {
        self.repo.head()
            .ok()
            .and_then(|head| head.target())
            .and_then(|oid| self.repo.find_commit(oid).ok())
            .and_then(|commit| commit.summary().map(|s| s.to_string()))
    }
    
    /// Get number of stashes
    fn get_stash_count(&self) -> Result<usize> {
        let mut count = 0;
        
        // Count stash entries by trying to resolve stash@{n}
        loop {
            let stash_ref = format!("stash@{{{}}}", count);
            if self.repo.revparse_single(&stash_ref).is_err() {
                break;
            }
            count += 1;
        }
        
        Ok(count)
    }
    
    /// Get current conflicts
    fn get_current_conflicts(&self) -> Result<Vec<StashConflict>> {
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
                
                let conflict_info = StashConflict {
                    path,
                    conflict_type: self.classify_stash_conflict_type(&conflict_data.ancestor, &conflict_data.our, &conflict_data.their),
                    our_content: None,    // Could be populated by reading blob content
                    their_content: None,  // Could be populated by reading blob content
                    base_content: None,   // Could be populated by reading blob content
                };
                
                conflicts.push(conflict_info);
            }
        }
        
        Ok(conflicts)
    }
    
    /// Get modified files in working directory
    fn get_modified_files(&self) -> Result<Vec<String>> {
        let mut files = Vec::new();
        let statuses = self.repo.statuses(None)?;
        
        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                files.push(path.to_string());
            }
        }
        
        Ok(files)
    }
    
    /// Check for potential conflicts when applying a stash
    fn check_stash_conflicts(&self, _stash_index: usize) -> Result<Option<Vec<StashConflict>>> {
        // Simplified conflict checking - in a full implementation,
        // we would simulate the apply operation to detect conflicts
        Ok(None)
    }
    
    /// Check if stash matches the given filters
    fn matches_filters(&self, stash_info: &StashInfo, options: &StashListOptions) -> bool {
        // Branch filter
        if let Some(ref branch_filter) = options.branch_filter {
            if let Some(ref branch_name) = stash_info.branch_name {
                if !branch_name.contains(branch_filter) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Author filter
        if let Some(ref author_filter) = options.author_filter {
            if !stash_info.author.name.contains(author_filter) && 
               !stash_info.author.email.contains(author_filter) {
                return false;
            }
        }
        
        // Message pattern filter
        if let Some(ref pattern) = options.message_pattern {
            if !stash_info.message.contains(pattern) {
                return false;
            }
        }
        
        // Date range filters
        if let Some(date_from) = options.date_from {
            if stash_info.created_date < date_from {
                return false;
            }
        }
        
        if let Some(date_to) = options.date_to {
            if stash_info.created_date > date_to {
                return false;
            }
        }
        
        true
    }
    
    /// Extract branch name from stash message
    fn extract_branch_from_message(&self, message: &str) -> Option<String> {
        // Parse "WIP on branch_name: commit_message" format
        if message.starts_with("WIP on ") {
            let parts: Vec<&str> = message.split(':').collect();
            if let Some(first_part) = parts.first() {
                let branch_part = first_part.strip_prefix("WIP on ").unwrap_or("");
                if !branch_part.is_empty() {
                    return Some(branch_part.to_string());
                }
            }
        }
        None
    }
    
    /// Get file count for a stash
    fn get_stash_file_count(&self, _index: usize) -> Result<usize> {
        // Simplified implementation - could be enhanced to count actual changed files
        Ok(0)
    }
    
    /// Check if stash includes untracked files
    fn stash_has_untracked(&self, _index: usize) -> Result<bool> {
        // Simplified implementation - could be enhanced to analyze stash content
        Ok(false)
    }
    
    /// Check if stash includes ignored files  
    fn stash_has_ignored(&self, _index: usize) -> Result<bool> {
        // Simplified implementation - could be enhanced to analyze stash content
        Ok(false)
    }
    
    /// Classify stash conflict type
    fn classify_stash_conflict_type(
        &self,
        ancestor: &Option<git2::IndexEntry>,
        our: &Option<git2::IndexEntry>,
        their: &Option<git2::IndexEntry>
    ) -> StashConflictType {
        match (ancestor.is_some(), our.is_some(), their.is_some()) {
            (false, true, true) => StashConflictType::AddAdd,
            (true, false, true) => StashConflictType::DeleteModify,
            (true, true, true) => {
                // Check if it's a mode conflict
                if let (Some(our_entry), Some(their_entry)) = (our, their) {
                    if our_entry.mode != their_entry.mode {
                        StashConflictType::FileMode
                    } else {
                        StashConflictType::Content
                    }
                } else {
                    StashConflictType::Content
                }
            }
            _ => StashConflictType::Content,
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
        
        info!("Recorded stash operation: {:?}", operation_type);
    }
    
    /// Get operation history
    pub fn get_operation_history(&self) -> &[OperationRecord] {
        &self.operation_history
    }
}

impl Default for StashCreateConfig {
    fn default() -> Self {
        Self {
            message: None,
            keep_index: false,
            include_untracked: false,
            include_ignored: false,
            all_files: false,
            pathspecs: vec![],
        }
    }
}

impl Default for StashApplyConfig {
    fn default() -> Self {
        Self {
            check_conflicts: true,
            reinstate_index: false,
            ignore_whitespace: false,
            strategy: StashApplyStrategy::Normal,
        }
    }
}

impl Default for StashListOptions {
    fn default() -> Self {
        Self {
            limit: None,
            include_stats: false,
            branch_filter: None,
            author_filter: None,
            message_pattern: None,
            date_from: None,
            date_to: None,
        }
    }
}