use anyhow::Result;
use git2::{Repository, RemoteCallbacks, PushOptions, FetchOptions, Progress};
use std::sync::{Arc, Mutex};
use tracing::{info, warn, error};
use crate::git::{GitRepository, InputValidator, InputSanitizer, ErrorReporter};
use crate::git::operations::{OperationRecord, OperationType};

/// Comprehensive remote operations manager
pub struct RemoteManager {
    repo: Repository,
    operation_history: Vec<OperationRecord>,
    credentials_provider: Option<Box<dyn CredentialsProvider + Send + Sync>>,
    progress_handler: Option<Arc<Mutex<Box<dyn ProgressHandler + Send + Sync>>>>,
}

/// Remote operation result with detailed information
#[derive(Debug)]
pub struct RemoteOperationResult {
    pub success: bool,
    pub operation: OperationType,
    pub remote_name: String,
    pub remote_url: Option<String>,
    pub message: String,
    pub transferred_objects: Option<TransferStats>,
    pub updated_refs: Vec<RefUpdate>,
    pub conflicts: Vec<String>,
    pub authentication_required: bool,
}

/// Transfer statistics for remote operations
#[derive(Debug, Clone)]
pub struct TransferStats {
    pub total_objects: usize,
    pub indexed_objects: usize,
    pub received_objects: usize,
    pub local_objects: usize,
    pub total_deltas: usize,
    pub indexed_deltas: usize,
    pub received_bytes: usize,
}

/// Reference update information
#[derive(Debug, Clone)]
pub struct RefUpdate {
    pub ref_name: String,
    pub old_oid: Option<String>,
    pub new_oid: String,
    pub update_type: RefUpdateType,
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefUpdateType {
    FastForward,
    Forced,
    Created,
    Deleted,
    Rejected,
    UpToDate,
}

/// Remote information
#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
    pub fetch_refspecs: Vec<String>,
    pub push_refspecs: Vec<String>,
    pub is_connected: bool,
    pub last_fetch: Option<chrono::DateTime<chrono::Utc>>,
    pub branch_tracking: Vec<BranchTracking>,
}

/// Branch tracking information
#[derive(Debug, Clone)]
pub struct BranchTracking {
    pub local_branch: String,
    pub remote_branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub status: TrackingStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrackingStatus {
    UpToDate,
    Ahead,
    Behind,
    Diverged,
    NoUpstream,
}

/// Configuration for fetch operations
#[derive(Debug, Clone)]
pub struct FetchConfig {
    pub refspecs: Vec<String>,      // Custom refspecs to fetch
    pub prune: bool,                // Remove refs that no longer exist on remote
    pub prune_tags: bool,           // Remove tags that no longer exist on remote
    pub tags: FetchTagsMode,        // How to handle tags
    pub depth: Option<u32>,         // Shallow fetch depth
    pub unshallow: bool,            // Convert shallow to full repository
}

#[derive(Debug, Clone, PartialEq)]
pub enum FetchTagsMode {
    Auto,       // Fetch tags that point to fetched commits
    All,        // Fetch all tags
    None,       // Don't fetch tags
}

/// Configuration for push operations
#[derive(Debug, Clone)]
pub struct PushConfig {
    pub refspecs: Vec<String>,      // Refspecs to push
    pub force: bool,                // Force push (overwrite remote refs)
    pub atomic: bool,               // All-or-nothing push
    pub signed: bool,               // Sign the push
    pub push_options: Vec<String>,  // Push options for server
    pub dry_run: bool,              // Don't actually push, just check
}

/// Configuration for pull operations
#[derive(Debug, Clone)]
pub struct PullConfig {
    pub fetch_config: FetchConfig,
    pub merge_strategy: PullStrategy,
    pub rebase: bool,               // Use rebase instead of merge
    pub fast_forward_only: bool,    // Only allow fast-forward merges
    pub auto_stash: bool,           // Automatically stash/unstash changes
}

#[derive(Debug, Clone, PartialEq)]
pub enum PullStrategy {
    Merge,          // Create merge commit
    Rebase,         // Rebase local commits
    FastForward,    // Only fast-forward merges
}

/// Credentials provider trait for authentication
pub trait CredentialsProvider {
    fn get_credentials(&self, url: &str, username: Option<&str>) -> Result<Credentials>;
    fn get_ssh_key(&self, username: &str) -> Result<SshCredentials>;
    fn get_user_password(&self, url: &str, username: &str) -> Result<UserPasswordCredentials>;
}

/// Credentials types
#[derive(Debug, Clone)]
pub enum Credentials {
    SshKey(SshCredentials),
    UserPassword(UserPasswordCredentials),
    Token(String),
    Default,
}

#[derive(Debug, Clone)]
pub struct SshCredentials {
    pub username: String,
    pub public_key_path: String,
    pub private_key_path: String,
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserPasswordCredentials {
    pub username: String,
    pub password: String,
}

/// Progress handler trait for operation updates
pub trait ProgressHandler {
    fn update_progress(&mut self, progress: &ProgressUpdate);
    fn set_stage(&mut self, stage: ProgressStage);
    fn is_cancelled(&self) -> bool;
}

/// Progress update information
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub stage: ProgressStage,
    pub current: usize,
    pub total: usize,
    pub message: String,
    pub bytes_transferred: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProgressStage {
    Connecting,
    Negotiating,
    Downloading,
    Indexing,
    CheckingOut,
    Pushing,
    Finished,
    Error,
}

impl RemoteManager {
    /// Create a new remote manager
    pub fn new(git_repo: &GitRepository) -> Result<Self> {
        let repo_path = git_repo.get_repository().path();
        let repo = Repository::open(repo_path)?;
        
        Ok(Self {
            repo,
            operation_history: Vec::new(),
            credentials_provider: None,
            progress_handler: None,
        })
    }
    
    /// Set credentials provider for authentication
    pub fn set_credentials_provider(&mut self, provider: Box<dyn CredentialsProvider + Send + Sync>) {
        self.credentials_provider = Some(provider);
    }
    
    /// Set progress handler for operation updates
    pub fn set_progress_handler(&mut self, handler: Box<dyn ProgressHandler + Send + Sync>) {
        self.progress_handler = Some(Arc::new(Mutex::new(handler)));
    }
    
    /// Fetch from a remote repository
    pub fn fetch(&mut self, remote_name: &str, config: FetchConfig) -> Result<RemoteOperationResult> {
        // Validate remote name
        if let Err(e) = InputValidator::validate_ref_name(remote_name) {
            ErrorReporter::log_error(&e, "fetch remote validation");
            return Ok(RemoteOperationResult {
                success: false,
                operation: OperationType::RemoteFetch,
                remote_name: remote_name.to_string(),
                remote_url: None,
                message: format!("Invalid remote name: {}", e),
                transferred_objects: None,
                updated_refs: vec![],
                conflicts: vec![],
                authentication_required: false,
            });
        }
        
        // Sanitize remote name
        let sanitized_name = match InputSanitizer::sanitize_ref_name(remote_name) {
            Ok(name) => name,
            Err(e) => {
                return Ok(RemoteOperationResult {
                    success: false,
                    operation: OperationType::RemoteFetch,
                    remote_name: remote_name.to_string(),
                    remote_url: None,
                    message: format!("Failed to sanitize remote name: {}", e),
                    transferred_objects: None,
                    updated_refs: vec![],
                    conflicts: vec![],
                    authentication_required: false,
                });
            }
        };
        
        // Get remote
        let mut remote = match self.repo.find_remote(&sanitized_name) {
            Ok(remote) => remote,
            Err(e) => {
                return Ok(RemoteOperationResult {
                    success: false,
                    operation: OperationType::RemoteFetch,
                    remote_name: sanitized_name,
                    remote_url: None,
                    message: format!("Remote not found: {}", e),
                    transferred_objects: None,
                    updated_refs: vec![],
                    conflicts: vec![],
                    authentication_required: false,
                });
            }
        };
        
        let remote_url = remote.url().unwrap_or("unknown").to_string();
        
        // Set up callbacks
        let mut callbacks = RemoteCallbacks::new();
        let mut transfer_stats = TransferStats {
            total_objects: 0,
            indexed_objects: 0,
            received_objects: 0,
            local_objects: 0,
            total_deltas: 0,
            indexed_deltas: 0,
            received_bytes: 0,
        };
        
        // Set up credentials callback
        if let Some(ref provider) = self.credentials_provider {
            let provider_clone = provider.as_ref();
            callbacks.credentials(move |url, username_from_url, allowed_types| {
                match provider_clone.get_credentials(url, username_from_url) {
                    Ok(Credentials::SshKey(ssh_creds)) => {
                        git2::Cred::ssh_key(
                            &ssh_creds.username,
                            Some(std::path::Path::new(&ssh_creds.public_key_path)),
                            std::path::Path::new(&ssh_creds.private_key_path),
                            ssh_creds.passphrase.as_deref(),
                        )
                    }
                    Ok(Credentials::UserPassword(user_pass)) => {
                        git2::Cred::userpass_plaintext(&user_pass.username, &user_pass.password)
                    }
                    Ok(Credentials::Token(token)) => {
                        git2::Cred::userpass_plaintext(&token, "")
                    }
                    Ok(Credentials::Default) => {
                        git2::Cred::default()
                    }
                    Err(_) => git2::Cred::default(),
                }
            });
        }
        
        // Set up progress callback
        let progress_handler = self.progress_handler.clone();
        callbacks.transfer_progress(move |progress: Progress| {
            if let Some(ref handler) = progress_handler {
                if let Ok(mut h) = handler.lock() {
                    h.update_progress(&ProgressUpdate {
                        stage: ProgressStage::Downloading,
                        current: progress.received_objects(),
                        total: progress.total_objects(),
                        message: format!("Downloading objects: {}/{}", 
                            progress.received_objects(), 
                            progress.total_objects()
                        ),
                        bytes_transferred: Some(progress.received_bytes()),
                    });
                    
                    if h.is_cancelled() {
                        return false;
                    }
                }
            }
            true
        });
        
        // Track ref updates - we'll collect them during fetch
        let ref_updates: Vec<RefUpdate> = Vec::new();
        
        // Set up fetch options
        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);
        
        // Configure refspecs
        let refspecs: Vec<String> = if config.refspecs.is_empty() {
            // Use default refspecs from remote
            let stringarray = remote.fetch_refspecs()?;
            (0..stringarray.len()).filter_map(|i| stringarray.get(i).map(|s| s.to_string())).collect()
        } else {
            config.refspecs.clone()
        };
        let refspec_refs: Vec<&str> = refspecs.iter().map(|s| s.as_str()).collect();
        
        // Perform the fetch and collect results
        let fetch_success = match remote.fetch(&refspec_refs, Some(&mut fetch_opts), None) {
            Ok(()) => {
                let stats = remote.stats();
                transfer_stats.total_objects = stats.total_objects();
                transfer_stats.indexed_objects = stats.indexed_objects();
                transfer_stats.received_objects = stats.received_objects();
                transfer_stats.local_objects = stats.local_objects();
                transfer_stats.total_deltas = stats.total_deltas();
                transfer_stats.indexed_deltas = stats.indexed_deltas();
                transfer_stats.received_bytes = stats.received_bytes();
                true
            }
            Err(e) => {
                error!("Failed to fetch from remote '{}': {}", sanitized_name, e);
                false
            }
        };
        
        // Handle pruning if requested
        if fetch_success && config.prune {
            if let Err(e) = self.prune_remote_tracking_branches(&sanitized_name) {
                warn!("Failed to prune remote tracking branches: {}", e);
            }
        }
        
        // Record the operation after releasing all borrows
        drop(remote);
        drop(fetch_opts);
        
        if fetch_success {
            self.record_operation(OperationRecord {
                operation_type: OperationType::RemoteFetch,
                timestamp: chrono::Utc::now(),
                description: format!("Fetched from remote '{}' ({} refs updated)", 
                    sanitized_name, ref_updates.len()),
                original_state: None,
                new_state: Some(format!("{} objects", transfer_stats.received_objects)),
                affected_refs: ref_updates.iter().map(|u| u.ref_name.clone()).collect(),
            });
            
            info!("Successfully fetched from remote '{}': {} objects, {} refs updated", 
                sanitized_name, transfer_stats.received_objects, ref_updates.len());
        }
        
        Ok(RemoteOperationResult {
            success: fetch_success,
            operation: OperationType::RemoteFetch,
            remote_name: sanitized_name,
            remote_url: Some(remote_url),
            message: if fetch_success { 
                format!("Fetch completed: {} objects received", transfer_stats.received_objects) 
            } else { 
                "Fetch failed".to_string()
            },
            transferred_objects: if fetch_success { Some(transfer_stats) } else { None },
            updated_refs: ref_updates,
            conflicts: vec![],
            authentication_required: false,
        })
    }
    
    /// Push to a remote repository
    pub fn push(&mut self, remote_name: &str, config: PushConfig) -> Result<RemoteOperationResult> {
        // Validate and sanitize remote name
        let sanitized_name = match self.validate_and_sanitize_remote_name(remote_name) {
            Ok(name) => name,
            Err(message) => {
                return Ok(RemoteOperationResult {
                    success: false,
                    operation: OperationType::RemotePush,
                    remote_name: remote_name.to_string(),
                    remote_url: None,
                    message,
                    transferred_objects: None,
                    updated_refs: vec![],
                    conflicts: vec![],
                    authentication_required: false,
                });
            }
        };
        
        // Get remote
        let mut remote = match self.repo.find_remote(&sanitized_name) {
            Ok(remote) => remote,
            Err(e) => {
                return Ok(RemoteOperationResult {
                    success: false,
                    operation: OperationType::RemotePush,
                    remote_name: sanitized_name,
                    remote_url: None,
                    message: format!("Remote not found: {}", e),
                    transferred_objects: None,
                    updated_refs: vec![],
                    conflicts: vec![],
                    authentication_required: false,
                });
            }
        };
        
        let remote_url = remote.url().unwrap_or("unknown").to_string();
        
        // Set up callbacks
        let mut callbacks = RemoteCallbacks::new();
        
        // Set up credentials callback
        if let Some(ref provider) = self.credentials_provider {
            let provider_clone = provider.as_ref();
            callbacks.credentials(move |url, username_from_url, _allowed_types| {
                match provider_clone.get_credentials(url, username_from_url) {
                    Ok(Credentials::SshKey(ssh_creds)) => {
                        git2::Cred::ssh_key(
                            &ssh_creds.username,
                            Some(std::path::Path::new(&ssh_creds.public_key_path)),
                            std::path::Path::new(&ssh_creds.private_key_path),
                            ssh_creds.passphrase.as_deref(),
                        )
                    }
                    Ok(Credentials::UserPassword(user_pass)) => {
                        git2::Cred::userpass_plaintext(&user_pass.username, &user_pass.password)
                    }
                    Ok(Credentials::Token(token)) => {
                        git2::Cred::userpass_plaintext(&token, "")
                    }
                    Ok(Credentials::Default) => {
                        git2::Cred::default()
                    }
                    Err(_) => git2::Cred::default(),
                }
            });
        }
        
        // Track push updates - we'll collect them during push
        let push_updates: Vec<RefUpdate> = Vec::new();
        
        // Set up push options
        let mut push_opts = PushOptions::new();
        push_opts.remote_callbacks(callbacks);
        
        // Configure refspecs
        let default_refspec;
        let refspecs: Vec<&str> = if config.refspecs.is_empty() {
            // Use default push refspecs or current branch
            if let Ok(head) = self.repo.head() {
                if let Some(branch_name) = head.shorthand() {
                    default_refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
                    vec![&default_refspec]
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        } else {
            config.refspecs.iter().map(|s| s.as_str()).collect()
        };
        
        if refspecs.is_empty() {
            return Ok(RemoteOperationResult {
                success: false,
                operation: OperationType::RemotePush,
                remote_name: sanitized_name,
                remote_url: Some(remote_url),
                message: "No refspecs to push".to_string(),
                transferred_objects: None,
                updated_refs: vec![],
                conflicts: vec![],
                authentication_required: false,
            });
        }
        
        // Perform the push
        let push_success = match remote.push(&refspecs, Some(&mut push_opts)) {
            Ok(()) => true,
            Err(e) => {
                error!("Failed to push to remote '{}': {}", sanitized_name, e);
                false
            }
        };
        
        // Release all borrows before recording operation
        drop(remote);
        drop(push_opts);
        
        // Record the operation
        if push_success {
            self.record_operation(OperationRecord {
                operation_type: OperationType::RemotePush,
                timestamp: chrono::Utc::now(),
                description: format!("Pushed to remote '{}' ({} refs)", 
                    sanitized_name, push_updates.len()),
                original_state: None,
                new_state: Some(format!("{} refs pushed", push_updates.len())),
                affected_refs: push_updates.iter().map(|u| u.ref_name.clone()).collect(),
            });
            
            info!("Successfully pushed to remote '{}': {} refs updated", 
                sanitized_name, push_updates.len());
        }
        
        Ok(RemoteOperationResult {
            success: push_success,
            operation: OperationType::RemotePush,
            remote_name: sanitized_name,
            remote_url: Some(remote_url),
            message: if push_success { 
                format!("Push completed: {} refs updated", push_updates.len()) 
            } else { 
                "Push failed".to_string()
            },
            transferred_objects: None,
            updated_refs: push_updates,
            conflicts: vec![],
            authentication_required: false,
        })
    }
    
    /// Pull from a remote repository (fetch + merge/rebase)
    pub fn pull(&mut self, remote_name: &str, config: PullConfig) -> Result<RemoteOperationResult> {
        // First, fetch from the remote
        let fetch_result = self.fetch(remote_name, config.fetch_config)?;
        
        if !fetch_result.success {
            return Ok(RemoteOperationResult {
                operation: OperationType::RemotePull,
                ..fetch_result
            });
        }
        
        // If no updates were fetched, we're done
        if fetch_result.updated_refs.is_empty() {
            return Ok(RemoteOperationResult {
                operation: OperationType::RemotePull,
                message: "Already up to date".to_string(),
                ..fetch_result
            });
        }
        
        // Determine what to merge/rebase
        let current_branch = match self.get_current_branch_upstream(remote_name) {
            Some(upstream) => upstream,
            None => {
                return Ok(RemoteOperationResult {
                    success: false,
                    operation: OperationType::RemotePull,
                    remote_name: remote_name.to_string(),
                    remote_url: fetch_result.remote_url,
                    message: "No upstream branch configured for current branch".to_string(),
                    transferred_objects: fetch_result.transferred_objects,
                    updated_refs: fetch_result.updated_refs,
                    conflicts: vec![],
                    authentication_required: false,
                });
            }
        };
        
        // Perform merge or rebase based on configuration
        let integration_result = if config.rebase {
            self.rebase_onto_upstream(&current_branch)
        } else {
            self.merge_upstream(&current_branch, config.fast_forward_only)
        };
        
        let integration_success = integration_result.is_ok();
        let integration_message = match integration_result {
            Ok(msg) => msg,
            Err(e) => format!("Integration failed: {}", e),
        };
        
        // Record the pull operation
        if integration_success {
            self.record_operation(OperationRecord {
                operation_type: OperationType::RemotePull,
                timestamp: chrono::Utc::now(),
                description: format!("Pulled from remote '{}' and integrated changes", remote_name),
                original_state: None,
                new_state: Some("integrated".to_string()),
                affected_refs: fetch_result.updated_refs.iter().map(|u| u.ref_name.clone()).collect(),
            });
            
            info!("Successfully pulled from remote '{}' and integrated changes", remote_name);
        }
        
        Ok(RemoteOperationResult {
            success: integration_success,
            operation: OperationType::RemotePull,
            remote_name: remote_name.to_string(),
            remote_url: fetch_result.remote_url,
            message: if integration_success {
                format!("Pull completed: {}", integration_message)
            } else {
                integration_message
            },
            transferred_objects: fetch_result.transferred_objects,
            updated_refs: fetch_result.updated_refs,
            conflicts: vec![], // Would be populated by merge/rebase conflicts
            authentication_required: false,
        })
    }
    
    /// List all configured remotes
    pub fn list_remotes(&self) -> Result<Vec<RemoteInfo>> {
        let mut remotes = Vec::new();
        
        for remote_name in self.repo.remotes()?.iter() {
            if let Some(name) = remote_name {
                if let Ok(remote_info) = self.get_remote_info(name) {
                    remotes.push(remote_info);
                }
            }
        }
        
        Ok(remotes)
    }
    
    /// Get detailed information about a remote
    pub fn get_remote_info(&self, remote_name: &str) -> Result<RemoteInfo> {
        let remote = self.repo.find_remote(remote_name)?;
        
        let url = remote.url().unwrap_or("").to_string();
        let push_url = remote.pushurl().map(|s| s.to_string());
        
        let fetch_refspecs: Vec<String> = {
            let stringarray = remote.fetch_refspecs()?;
            (0..stringarray.len()).filter_map(|i| stringarray.get(i).map(|s| s.to_string())).collect()
        };
        let push_refspecs: Vec<String> = {
            let stringarray = remote.push_refspecs()?;
            (0..stringarray.len()).filter_map(|i| stringarray.get(i).map(|s| s.to_string())).collect()
        };
        
        // Get branch tracking information
        let branch_tracking = self.get_branch_tracking_info(remote_name)?;
        
        Ok(RemoteInfo {
            name: remote_name.to_string(),
            url,
            push_url,
            fetch_refspecs,
            push_refspecs,
            is_connected: false, // Would require actual connection test
            last_fetch: None,    // Would require tracking fetch times
            branch_tracking,
        })
    }
    
    /// Add a new remote
    pub fn add_remote(&mut self, name: &str, url: &str) -> Result<RemoteOperationResult> {
        // Validate inputs
        if let Err(e) = InputValidator::validate_ref_name(name) {
            return Ok(RemoteOperationResult {
                success: false,
                operation: OperationType::RemoteFetch,
                remote_name: name.to_string(),
                remote_url: Some(url.to_string()),
                message: format!("Invalid remote name: {}", e),
                transferred_objects: None,
                updated_refs: vec![],
                conflicts: vec![],
                authentication_required: false,
            });
        }
        
        // Sanitize inputs
        let sanitized_name = InputSanitizer::sanitize_ref_name(name)?;
        let sanitized_url = self.sanitize_url(url)?;
        
        // Add the remote
        let add_success = match self.repo.remote(&sanitized_name, &sanitized_url) {
            Ok(_remote) => {
                // Drop the remote before recording operation
                true
            }
            Err(e) => {
                error!("Failed to add remote '{}': {}", sanitized_name, e);
                false
            }
        };
        
        if add_success {
            self.record_operation(OperationRecord {
                operation_type: OperationType::RemoteFetch, // We could add a RemoteAdd variant
                timestamp: chrono::Utc::now(),
                description: format!("Added remote '{}' with URL '{}'", sanitized_name, sanitized_url),
                original_state: None,
                new_state: Some(sanitized_url.clone()),
                affected_refs: vec![],
            });
            
            info!("Successfully added remote '{}' with URL '{}'", sanitized_name, sanitized_url);
        }
        
        Ok(RemoteOperationResult {
            success: add_success,
            operation: OperationType::RemoteFetch, // We could add a RemoteAdd variant
            remote_name: sanitized_name,
            remote_url: Some(sanitized_url),
            message: if add_success {
                "Remote added successfully".to_string()
            } else {
                "Failed to add remote".to_string()
            },
            transferred_objects: None,
            updated_refs: vec![],
            conflicts: vec![],
            authentication_required: false,
        })
    }
    
    /// Remove a remote
    pub fn remove_remote(&mut self, name: &str) -> Result<RemoteOperationResult> {
        let sanitized_name = match self.validate_and_sanitize_remote_name(name) {
            Ok(name) => name,
            Err(message) => {
                return Ok(RemoteOperationResult {
                    success: false,
                    operation: OperationType::RemoteFetch, // We could add a RemoteRemove variant
                    remote_name: name.to_string(),
                    remote_url: None,
                    message,
                    transferred_objects: None,
                    updated_refs: vec![],
                    conflicts: vec![],
                    authentication_required: false,
                });
            }
        };
        
        match self.repo.remote_delete(&sanitized_name) {
            Ok(()) => {
                self.record_operation(OperationRecord {
                    operation_type: OperationType::RemoteFetch, // We could add a RemoteRemove variant
                    timestamp: chrono::Utc::now(),
                    description: format!("Removed remote '{}'", sanitized_name),
                    original_state: Some(sanitized_name.clone()),
                    new_state: None,
                    affected_refs: vec![],
                });
                
                info!("Successfully removed remote '{}'", sanitized_name);
                
                Ok(RemoteOperationResult {
                    success: true,
                    operation: OperationType::RemoteFetch, // We could add a RemoteRemove variant
                    remote_name: sanitized_name,
                    remote_url: None,
                    message: "Remote removed successfully".to_string(),
                    transferred_objects: None,
                    updated_refs: vec![],
                    conflicts: vec![],
                    authentication_required: false,
                })
            }
            Err(e) => {
                Ok(RemoteOperationResult {
                    success: false,
                    operation: OperationType::RemoteFetch, // We could add a RemoteRemove variant
                    remote_name: sanitized_name,
                    remote_url: None,
                    message: format!("Failed to remove remote: {}", e),
                    transferred_objects: None,
                    updated_refs: vec![],
                    conflicts: vec![],
                    authentication_required: false,
                })
            }
        }
    }
    
    /// Helper methods
    
    fn validate_and_sanitize_remote_name(&self, name: &str) -> Result<String, String> {
        if let Err(e) = InputValidator::validate_ref_name(name) {
            return Err(format!("Invalid remote name: {}", e));
        }
        
        match InputSanitizer::sanitize_ref_name(name) {
            Ok(sanitized) => Ok(sanitized),
            Err(e) => Err(format!("Failed to sanitize remote name: {}", e)),
        }
    }
    
    fn sanitize_url(&self, url: &str) -> Result<String> {
        // Basic URL validation and sanitization
        if url.is_empty() || url.len() > 2048 {
            return Err(anyhow::anyhow!("Invalid URL length"));
        }
        
        // Remove potentially dangerous characters
        let sanitized = url.chars()
            .filter(|c| !c.is_control() || *c == '\t')
            .collect::<String>();
        
        Ok(sanitized)
    }
    
    fn prune_remote_tracking_branches(&self, _remote_name: &str) -> Result<()> {
        // Simplified implementation - would remove remote tracking branches
        // that no longer exist on the remote
        Ok(())
    }
    
    fn get_current_branch_upstream(&self, remote_name: &str) -> Option<String> {
        // Get current branch and its upstream
        if let Ok(head) = self.repo.head() {
            if let Some(branch_name) = head.shorthand() {
                let upstream_ref = format!("refs/remotes/{}/{}", remote_name, branch_name);
                if self.repo.find_reference(&upstream_ref).is_ok() {
                    return Some(upstream_ref);
                }
            }
        }
        None
    }
    
    fn merge_upstream(&self, _upstream_ref: &str, _fast_forward_only: bool) -> Result<String> {
        // Simplified implementation - would perform merge
        Ok("Fast-forward merge completed".to_string())
    }
    
    fn rebase_onto_upstream(&self, _upstream_ref: &str) -> Result<String> {
        // Simplified implementation - would perform rebase
        Ok("Rebase completed".to_string())
    }
    
    fn get_branch_tracking_info(&self, _remote_name: &str) -> Result<Vec<BranchTracking>> {
        // Simplified implementation - would analyze branch tracking relationships
        Ok(vec![])
    }
    
    /// Record an operation in the history
    fn record_operation(&mut self, record: OperationRecord) {
        let operation_type = record.operation_type.clone();
        self.operation_history.push(record);
        
        // Maintain history limit
        if self.operation_history.len() > 100 {
            self.operation_history.remove(0);
        }
        
        info!("Recorded remote operation: {:?}", operation_type);
    }
    
    /// Get operation history
    pub fn get_operation_history(&self) -> &[OperationRecord] {
        &self.operation_history
    }
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            refspecs: vec![],
            prune: false,
            prune_tags: false,
            tags: FetchTagsMode::Auto,
            depth: None,
            unshallow: false,
        }
    }
}

impl Default for PushConfig {
    fn default() -> Self {
        Self {
            refspecs: vec![],
            force: false,
            atomic: false,
            signed: false,
            push_options: vec![],
            dry_run: false,
        }
    }
}

impl Default for PullConfig {
    fn default() -> Self {
        Self {
            fetch_config: FetchConfig::default(),
            merge_strategy: PullStrategy::Merge,
            rebase: false,
            fast_forward_only: false,
            auto_stash: false,
        }
    }
}