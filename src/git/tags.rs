use anyhow::Result;
use git2::{Repository, Oid, ObjectType, Signature};
use tracing::{info, warn, error};
use crate::git::{GitRepository, InputValidator, InputSanitizer, ErrorReporter};
use crate::git::operations::{OperationRecord, OperationType};

/// Comprehensive tag management system
pub struct TagManager {
    repo: Repository,
    operation_history: Vec<OperationRecord>,
}

/// Tag operation result with detailed information
#[derive(Debug)]
pub struct TagOperationResult {
    pub success: bool,
    pub operation: OperationType,
    pub tag_name: String,
    pub target_commit: Option<String>,
    pub message: String,
    pub tag_type: TagType,
    pub signature: Option<TagSignature>,
}

/// Tag information structure
#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub target_oid: String,
    pub target_type: ObjectType,
    pub tag_type: TagType,
    pub message: Option<String>,
    pub tagger: Option<TagSignature>,
    pub created_date: Option<chrono::DateTime<chrono::Utc>>,
}

/// Tag type classification
#[derive(Debug, Clone, PartialEq)]
pub enum TagType {
    Lightweight,  // Direct reference to commit
    Annotated,    // Tag object with metadata
}

/// Tag signature information
#[derive(Debug, Clone)]
pub struct TagSignature {
    pub name: String,
    pub email: String,
    pub when: chrono::DateTime<chrono::Utc>,
}

/// Tag creation configuration
#[derive(Debug, Clone)]
pub struct TagCreateConfig {
    pub tag_type: TagType,
    pub message: Option<String>,
    pub force_overwrite: bool,
    pub sign_tag: bool,
    pub tagger: Option<TagSignature>,
}

/// Tag filtering and search options
#[derive(Debug, Clone)]
pub struct TagFilterOptions {
    pub pattern: Option<String>,          // Glob pattern for tag names
    pub include_lightweight: bool,
    pub include_annotated: bool,
    pub limit: Option<usize>,
    pub sort_by: TagSortBy,
    pub sort_order: SortOrder,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TagSortBy {
    Name,
    CreationDate,
    CommitDate,
    Version,  // Semantic version sorting
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl TagManager {
    /// Create a new tag manager
    pub fn new(git_repo: &GitRepository) -> Result<Self> {
        let repo_path = git_repo.get_repository().path();
        let repo = Repository::open(repo_path)?;
        
        Ok(Self {
            repo,
            operation_history: Vec::new(),
        })
    }
    
    /// Create a new tag
    pub fn create_tag(&mut self, tag_name: &str, target_commit: &str, config: TagCreateConfig) -> Result<TagOperationResult> {
        // Validate inputs
        if let Err(e) = InputValidator::validate_ref_name(tag_name) {
            ErrorReporter::log_error(&e, "tag creation validation");
            return Ok(TagOperationResult {
                success: false,
                operation: OperationType::TagCreate,
                tag_name: tag_name.to_string(),
                target_commit: None,
                message: format!("Invalid tag name: {}", e),
                tag_type: config.tag_type,
                signature: None,
            });
        }
        
        if let Err(e) = InputValidator::validate_commit_id(target_commit) {
            ErrorReporter::log_error(&e, "tag creation validation");
            return Ok(TagOperationResult {
                success: false,
                operation: OperationType::TagCreate,
                tag_name: tag_name.to_string(),
                target_commit: None,
                message: format!("Invalid commit ID: {}", e),
                tag_type: config.tag_type,
                signature: None,
            });
        }
        
        // Sanitize inputs
        let sanitized_name = match InputSanitizer::sanitize_ref_name(tag_name) {
            Ok(name) => name,
            Err(e) => {
                return Ok(TagOperationResult {
                    success: false,
                    operation: OperationType::TagCreate,
                    tag_name: tag_name.to_string(),
                    target_commit: None,
                    message: format!("Failed to sanitize tag name: {}", e),
                    tag_type: config.tag_type,
                    signature: None,
                });
            }
        };
        
        let sanitized_commit = match InputSanitizer::sanitize_commit_id(target_commit) {
            Ok(commit) => commit,
            Err(e) => {
                return Ok(TagOperationResult {
                    success: false,
                    operation: OperationType::TagCreate,
                    tag_name: tag_name.to_string(),
                    target_commit: None,
                    message: format!("Failed to sanitize commit ID: {}", e),
                    tag_type: config.tag_type,
                    signature: None,
                });
            }
        };
        
        // Check if tag already exists
        if !config.force_overwrite {
            if let Ok(_) = self.repo.find_reference(&format!("refs/tags/{}", sanitized_name)) {
                return Ok(TagOperationResult {
                    success: false,
                    operation: OperationType::TagCreate,
                    tag_name: sanitized_name.clone(),
                    target_commit: Some(sanitized_commit),
                    message: format!("Tag '{}' already exists. Use force to overwrite.", sanitized_name),
                    tag_type: config.tag_type,
                    signature: None,
                });
            }
        }
        
        // Find target commit
        let target_oid = match Oid::from_str(&sanitized_commit) {
            Ok(oid) => oid,
            Err(e) => {
                return Ok(TagOperationResult {
                    success: false,
                    operation: OperationType::TagCreate,
                    tag_name: sanitized_name,
                    target_commit: Some(sanitized_commit),
                    message: format!("Invalid commit OID: {}", e),
                    tag_type: config.tag_type,
                    signature: None,
                });
            }
        };
        
        // Create the tag in separate scope to avoid borrowing issues
        let tag_result = {
            let target_object = match self.repo.find_object(target_oid, None) {
                Ok(obj) => obj,
                Err(e) => {
                    return Ok(TagOperationResult {
                        success: false,
                        operation: OperationType::TagCreate,
                        tag_name: sanitized_name,
                        target_commit: Some(sanitized_commit),
                        message: format!("Target object not found: {}", e),
                        tag_type: config.tag_type,
                        signature: None,
                    });
                }
            };
            
            // Create signature
            let signature = match self.create_signature(&config) {
                Ok(sig) => sig,
                Err(e) => {
                    return Ok(TagOperationResult {
                        success: false,
                        operation: OperationType::TagCreate,
                        tag_name: sanitized_name,
                        target_commit: Some(sanitized_commit),
                        message: format!("Failed to create signature: {}", e),
                        tag_type: config.tag_type,
                        signature: None,
                    });
                }
            };
            
            // Create the tag
            match config.tag_type {
                TagType::Lightweight => {
                    // Create lightweight tag (direct reference)
                    match self.repo.reference(
                        &format!("refs/tags/{}", sanitized_name),
                        target_oid,
                        config.force_overwrite,
                        "Create lightweight tag",
                    ) {
                        Ok(_) => (true, None, String::new()),
                        Err(e) => (false, None, format!("Failed to create lightweight tag: {}", e)),
                    }
                }
                TagType::Annotated => {
                    // Create annotated tag object
                    let tag_message = config.message.as_deref().unwrap_or("Tag created");
                    match self.repo.tag(
                        &sanitized_name,
                        &target_object,
                        &signature,
                        tag_message,
                        config.force_overwrite,
                    ) {
                        Ok(_tag_oid) => {
                            let tag_sig = TagSignature {
                                name: signature.name().unwrap_or("Unknown").to_string(),
                                email: signature.email().unwrap_or("unknown@example.com").to_string(),
                                when: chrono::DateTime::from_timestamp(signature.when().seconds(), 0)
                                    .unwrap_or_else(chrono::Utc::now),
                            };
                            (true, Some(tag_sig), String::new())
                        }
                        Err(e) => (false, None, format!("Failed to create annotated tag: {}", e)),
                    }
                }
            }
        };
        
        if !tag_result.0 {
            return Ok(TagOperationResult {
                success: false,
                operation: OperationType::TagCreate,
                tag_name: sanitized_name,
                target_commit: Some(sanitized_commit),
                message: tag_result.2,
                tag_type: config.tag_type,
                signature: tag_result.1,
            });
        }
        
        // Record the operation
        self.record_operation(OperationRecord {
            operation_type: OperationType::TagCreate,
            timestamp: chrono::Utc::now(),
            description: format!("Created {} tag '{}' at commit {}", 
                if config.tag_type == TagType::Annotated { "annotated" } else { "lightweight" },
                sanitized_name, 
                &sanitized_commit[..8]
            ),
            original_state: None,
            new_state: Some(target_oid.to_string()),
            affected_refs: vec![format!("refs/tags/{}", sanitized_name)],
        });
        
        info!("Successfully created {} tag '{}' at commit {}", 
              if config.tag_type == TagType::Annotated { "annotated" } else { "lightweight" },
              sanitized_name, sanitized_commit);
        
        Ok(TagOperationResult {
            success: true,
            operation: OperationType::TagCreate,
            tag_name: sanitized_name,
            target_commit: Some(sanitized_commit),
            message: "Successfully created tag".to_string(),
            tag_type: config.tag_type,
            signature: tag_result.1,
        })
    }
    
    /// Delete a tag
    pub fn delete_tag(&mut self, tag_name: &str, force: bool) -> Result<TagOperationResult> {
        // Validate input
        if let Err(e) = InputValidator::validate_ref_name(tag_name) {
            ErrorReporter::log_error(&e, "tag deletion validation");
            return Ok(TagOperationResult {
                success: false,
                operation: OperationType::TagDelete,
                tag_name: tag_name.to_string(),
                target_commit: None,
                message: format!("Invalid tag name: {}", e),
                tag_type: TagType::Lightweight, // Default, will be determined
                signature: None,
            });
        }
        
        // Sanitize input
        let sanitized_name = match InputSanitizer::sanitize_ref_name(tag_name) {
            Ok(name) => name,
            Err(e) => {
                return Ok(TagOperationResult {
                    success: false,
                    operation: OperationType::TagDelete,
                    tag_name: tag_name.to_string(),
                    target_commit: None,
                    message: format!("Failed to sanitize tag name: {}", e),
                    tag_type: TagType::Lightweight,
                    signature: None,
                });
            }
        };
        
        // Get tag information before deletion
        let tag_info = match self.get_tag_info(&sanitized_name) {
            Ok(info) => info,
            Err(_) => {
                return Ok(TagOperationResult {
                    success: false,
                    operation: OperationType::TagDelete,
                    tag_name: sanitized_name.clone(),
                    target_commit: None,
                    message: format!("Tag '{}' not found", sanitized_name),
                    tag_type: TagType::Lightweight,
                    signature: None,
                });
            }
        };
        
        // Safety check for protected tags (unless force)
        if !force && self.is_protected_tag(&sanitized_name) {
            return Ok(TagOperationResult {
                success: false,
                operation: OperationType::TagDelete,
                tag_name: sanitized_name.clone(),
                target_commit: Some(tag_info.target_oid.clone()),
                message: format!("Tag '{}' is protected. Use force to delete.", sanitized_name),
                tag_type: tag_info.tag_type,
                signature: tag_info.tagger,
            });
        }
        
        // Delete the tag reference in separate scope
        let delete_result = {
            match self.repo.find_reference(&format!("refs/tags/{}", sanitized_name)) {
                Ok(mut tag_ref) => {
                    match tag_ref.delete() {
                        Ok(()) => (true, tag_info.target_oid.clone(), tag_info.tag_type.clone(), tag_info.tagger.clone(), String::new()),
                        Err(e) => (false, tag_info.target_oid.clone(), tag_info.tag_type.clone(), tag_info.tagger.clone(), format!("Failed to delete tag: {}", e)),
                    }
                }
                Err(e) => (false, String::new(), TagType::Lightweight, None, format!("Tag reference not found: {}", e)),
            }
        };
        
        if delete_result.0 {
            // Record the operation
            self.record_operation(OperationRecord {
                operation_type: OperationType::TagDelete,
                timestamp: chrono::Utc::now(),
                description: format!("Deleted tag '{}'", sanitized_name),
                original_state: Some(delete_result.1.clone()),
                new_state: None,
                affected_refs: vec![format!("refs/tags/{}", sanitized_name)],
            });
            
            info!("Successfully deleted tag '{}'", sanitized_name);
            
            Ok(TagOperationResult {
                success: true,
                operation: OperationType::TagDelete,
                tag_name: sanitized_name,
                target_commit: Some(delete_result.1),
                message: "Successfully deleted tag".to_string(),
                tag_type: delete_result.2,
                signature: delete_result.3,
            })
        } else {
            error!("Failed to delete tag '{}': {}", sanitized_name, delete_result.4);
            
            Ok(TagOperationResult {
                success: false,
                operation: OperationType::TagDelete,
                tag_name: sanitized_name,
                target_commit: if delete_result.1.is_empty() { None } else { Some(delete_result.1) },
                message: delete_result.4,
                tag_type: delete_result.2,
                signature: delete_result.3,
            })
        }
    }
    
    /// Get detailed information about a tag
    pub fn get_tag_info(&self, tag_name: &str) -> Result<TagInfo> {
        let tag_ref = self.repo.find_reference(&format!("refs/tags/{}", tag_name))?;
        let target_oid = tag_ref.target().ok_or_else(|| anyhow::anyhow!("Tag has no target"))?;
        
        // Try to get tag object first (for annotated tags)
        if let Ok(tag_obj) = self.repo.find_tag(target_oid) {
            // Annotated tag
            let tagger_sig = tag_obj.tagger();
            let tagger = tagger_sig.map(|sig| TagSignature {
                name: sig.name().unwrap_or("Unknown").to_string(),
                email: sig.email().unwrap_or("unknown@example.com").to_string(),
                when: chrono::DateTime::from_timestamp(sig.when().seconds(), 0)
                    .unwrap_or_else(chrono::Utc::now),
            });
            
            Ok(TagInfo {
                name: tag_name.to_string(),
                target_oid: tag_obj.target_id().to_string(),
                target_type: tag_obj.target_type().unwrap_or(ObjectType::Any),
                tag_type: TagType::Annotated,
                message: Some(tag_obj.message().unwrap_or("").to_string()),
                tagger: tagger.clone(),
                created_date: tagger.as_ref().map(|t| t.when),
            })
        } else {
            // Lightweight tag - direct reference to object
            let target_obj = self.repo.find_object(target_oid, None)?;
            
            Ok(TagInfo {
                name: tag_name.to_string(),
                target_oid: target_oid.to_string(),
                target_type: target_obj.kind().unwrap_or(ObjectType::Any),
                tag_type: TagType::Lightweight,
                message: None,
                tagger: None,
                created_date: None,
            })
        }
    }
    
    /// List all tags with filtering options
    pub fn list_tags(&self, filter: Option<TagFilterOptions>) -> Result<Vec<TagInfo>> {
        let filter = filter.unwrap_or_else(|| TagFilterOptions {
            pattern: None,
            include_lightweight: true,
            include_annotated: true,
            limit: None,
            sort_by: TagSortBy::Name,
            sort_order: SortOrder::Ascending,
        });
        
        let mut tags = Vec::new();
        
        // Iterate through all tag references
        let tag_refs = self.repo.references_glob("refs/tags/*")?;
        
        for tag_ref_result in tag_refs {
            if let Ok(tag_ref) = tag_ref_result {
                if let Some(tag_name) = tag_ref.shorthand() {
                    // Apply pattern filter
                    if let Some(ref pattern) = filter.pattern {
                        if !self.matches_pattern(tag_name, pattern) {
                            continue;
                        }
                    }
                    
                    // Get tag info
                    if let Ok(tag_info) = self.get_tag_info(tag_name) {
                        // Apply type filter
                        let include = match tag_info.tag_type {
                            TagType::Lightweight => filter.include_lightweight,
                            TagType::Annotated => filter.include_annotated,
                        };
                        
                        if include {
                            tags.push(tag_info);
                        }
                    }
                }
            }
        }
        
        // Sort tags
        self.sort_tags(&mut tags, filter.sort_by, filter.sort_order);
        
        // Apply limit
        if let Some(limit) = filter.limit {
            tags.truncate(limit);
        }
        
        Ok(tags)
    }
    
    /// Check if tag name matches a glob pattern
    fn matches_pattern(&self, tag_name: &str, pattern: &str) -> bool {
        // Simple glob matching - could be enhanced with a proper glob library
        if pattern.contains('*') {
            let pattern_parts: Vec<&str> = pattern.split('*').collect();
            if pattern_parts.len() == 2 {
                let prefix = pattern_parts[0];
                let suffix = pattern_parts[1];
                return tag_name.starts_with(prefix) && tag_name.ends_with(suffix);
            }
        }
        tag_name == pattern
    }
    
    /// Sort tags based on criteria
    fn sort_tags(&self, tags: &mut [TagInfo], sort_by: TagSortBy, order: SortOrder) {
        match sort_by {
            TagSortBy::Name => {
                tags.sort_by(|a, b| {
                    let cmp = a.name.cmp(&b.name);
                    if order == SortOrder::Descending { cmp.reverse() } else { cmp }
                });
            }
            TagSortBy::CreationDate => {
                tags.sort_by(|a, b| {
                    let a_date = a.created_date.unwrap_or_else(chrono::Utc::now);
                    let b_date = b.created_date.unwrap_or_else(chrono::Utc::now);
                    let cmp = a_date.cmp(&b_date);
                    if order == SortOrder::Descending { cmp.reverse() } else { cmp }
                });
            }
            TagSortBy::CommitDate => {
                // Would need to look up commit dates - simplified for now
                tags.sort_by(|a, b| {
                    let cmp = a.target_oid.cmp(&b.target_oid);
                    if order == SortOrder::Descending { cmp.reverse() } else { cmp }
                });
            }
            TagSortBy::Version => {
                // Semantic version sorting - simplified implementation
                tags.sort_by(|a, b| {
                    let cmp = self.compare_version_tags(&a.name, &b.name);
                    if order == SortOrder::Descending { cmp.reverse() } else { cmp }
                });
            }
        }
    }
    
    /// Compare version tags (simplified semantic versioning)
    fn compare_version_tags(&self, a: &str, b: &str) -> std::cmp::Ordering {
        // Extract version numbers from tag names (e.g., "v1.2.3" -> [1, 2, 3])
        let extract_version = |tag: &str| -> Vec<u32> {
            tag.trim_start_matches('v')
                .split('.')
                .filter_map(|s| s.parse().ok())
                .collect()
        };
        
        let version_a = extract_version(a);
        let version_b = extract_version(b);
        
        // Compare version components
        for (va, vb) in version_a.iter().zip(version_b.iter()) {
            match va.cmp(vb) {
                std::cmp::Ordering::Equal => continue,
                other => return other,
            }
        }
        
        // If all compared components are equal, longer version is greater
        version_a.len().cmp(&version_b.len())
    }
    
    /// Check if a tag is protected (e.g., release tags)
    fn is_protected_tag(&self, tag_name: &str) -> bool {
        // Simple protection rules - could be made configurable
        tag_name.starts_with("v") ||  // Version tags
        tag_name.starts_with("release-") ||  // Release tags
        tag_name == "latest"  // Latest tag
    }
    
    /// Create Git signature from config or use default
    fn create_signature(&self, config: &TagCreateConfig) -> Result<Signature> {
        if let Some(ref custom_sig) = config.tagger {
            Ok(Signature::now(&custom_sig.name, &custom_sig.email)?)
        } else {
            // Try to get signature from repository config
            match self.repo.signature() {
                Ok(sig) => Ok(sig),
                Err(_) => {
                    // Fallback to default signature
                    Ok(Signature::now("Git User", "user@example.com")?)
                }
            }
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
        
        info!("Recorded tag operation: {:?}", operation_type);
    }
    
    /// Get operation history
    pub fn get_operation_history(&self) -> &[OperationRecord] {
        &self.operation_history
    }
    
    /// Find tags that point to a specific commit
    pub fn get_tags_for_commit(&self, commit_id: &str) -> Result<Vec<TagInfo>> {
        let target_oid = Oid::from_str(commit_id)?;
        let mut matching_tags = Vec::new();
        
        let tag_refs = self.repo.references_glob("refs/tags/*")?;
        
        for tag_ref_result in tag_refs {
            if let Ok(tag_ref) = tag_ref_result {
                if let Some(tag_name) = tag_ref.shorthand() {
                    if let Ok(tag_info) = self.get_tag_info(tag_name) {
                        // Check if tag points to the target commit (directly or indirectly)
                        if let Ok(tag_target_oid) = Oid::from_str(&tag_info.target_oid) {
                            if tag_target_oid == target_oid {
                                matching_tags.push(tag_info);
                            } else if tag_info.tag_type == TagType::Annotated {
                                // For annotated tags, check if the tag object points to our commit
                                if let Ok(tag_obj) = self.repo.find_tag(tag_ref.target().unwrap()) {
                                    if tag_obj.target_id() == target_oid {
                                        matching_tags.push(tag_info);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(matching_tags)
    }
}

impl Default for TagCreateConfig {
    fn default() -> Self {
        Self {
            tag_type: TagType::Lightweight,
            message: None,
            force_overwrite: false,
            sign_tag: false,
            tagger: None,
        }
    }
}

impl Default for TagFilterOptions {
    fn default() -> Self {
        Self {
            pattern: None,
            include_lightweight: true,
            include_annotated: true,
            limit: None,
            sort_by: TagSortBy::Name,
            sort_order: SortOrder::Ascending,
        }
    }
}