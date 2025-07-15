use crate::git::{
    CommitStream, ErrorRecovery, ErrorReporter, GitError, GitRepository, InputSanitizer,
    InputValidator, RefManager, ViewManager,
};
use crate::models::{GitCommit, GitDiff, RepositoryInfo};

#[derive(Debug)]
pub struct AppState {
    pub repository: Option<GitRepository>,
    pub commits: Vec<GitCommit>,
    pub selected_commit: Option<String>,
    pub selected_files: Vec<String>,
    pub current_diff: Option<GitDiff>,
    pub search_query: String,
    pub filter_author: String,
    pub filter_branch: String,
    pub show_all_branches: bool,
    pub commit_limit: usize,
    pub loading: bool,
    pub error_message: Option<String>,
    pub commit_stream: Option<CommitStream>,
    pub stream_complete: bool,
    pub ref_manager: Option<RefManager>,
    pub selected_branch: Option<String>,
    pub show_remote_branches: bool,
    pub view_manager: Option<ViewManager>,
    pub focus_search: bool,
    pub selected_commit_index: Option<usize>,
    pub show_shortcuts_dialog: bool,
    pub show_about_dialog: bool,
    pub show_settings_dialog: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            repository: None,
            commits: Vec::new(),
            selected_commit: None,
            selected_files: Vec::new(),
            current_diff: None,
            search_query: String::new(),
            filter_author: String::new(),
            filter_branch: String::new(),
            show_all_branches: false,
            commit_limit: 1000,
            loading: false,
            error_message: None,
            commit_stream: None,
            stream_complete: false,
            ref_manager: None,
            selected_branch: None,
            show_remote_branches: false,
            view_manager: None,
            focus_search: false,
            selected_commit_index: None,
            show_shortcuts_dialog: false,
            show_about_dialog: false,
            show_settings_dialog: false,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_repository(&mut self, repo: GitRepository) {
        self.repository = Some(repo);
        self.load_references();
        self.initialize_views();
        self.start_streaming_commits();
    }

    pub fn refresh_commits(&mut self) {
        if let Some(ref repo) = self.repository {
            self.loading = true;
            match repo.get_commits(Some(self.commit_limit)) {
                Ok(commits) => {
                    self.commits = commits;
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to load commits: {}", e));
                }
            }
            self.loading = false;
        }
    }

    pub fn select_commit(&mut self, commit_id: String) {
        // Enhanced validation and error handling
        if let Err(e) = InputValidator::validate_commit_id(&commit_id) {
            ErrorReporter::log_error(&e, "commit selection");
            self.error_message = Some(ErrorRecovery::user_friendly_message(&e));
            return;
        }

        // Sanitize commit ID for security
        match InputSanitizer::sanitize_commit_id(&commit_id) {
            Ok(sanitized_id) => {
                self.selected_commit = Some(sanitized_id.clone());
                self.load_commit_diff(&sanitized_id);
            }
            Err(e) => {
                let git_error = GitError::invalid_input(commit_id, e.to_string());
                ErrorReporter::log_error(&git_error, "commit ID sanitization");
                self.error_message = Some(ErrorRecovery::user_friendly_message(&git_error));
            }
        }
    }

    pub fn load_commit_diff(&mut self, commit_id: &str) {
        if let Some(ref repo) = self.repository {
            // Sanitize commit ID again for safety
            match InputSanitizer::sanitize_commit_id(commit_id) {
                Ok(sanitized_id) => match repo.get_commit_diff_enhanced(&sanitized_id) {
                    Ok(diffs) => {
                        if let Some(first_diff) = diffs.into_iter().next() {
                            self.current_diff = Some(first_diff);
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to load diff: {}", e));
                    }
                },
                Err(e) => {
                    self.error_message = Some(format!("Invalid commit ID: {}", e));
                }
            }
        }
    }

    pub fn search_commits(&mut self, query: &str) {
        if let Some(ref repo) = self.repository {
            // Enhanced validation
            if let Err(e) = InputValidator::validate_search_query(query) {
                ErrorReporter::log_error(&e, "search query validation");
                self.error_message = Some(ErrorRecovery::user_friendly_message(&e));
                return;
            }

            // Sanitize search query for security
            match InputSanitizer::sanitize_search_query(query) {
                Ok(sanitized_query) => {
                    self.search_query = sanitized_query.clone();
                    if sanitized_query.is_empty() {
                        self.refresh_commits();
                    } else {
                        self.loading = true;
                        match repo.search_commits(&sanitized_query, Some(self.commit_limit)) {
                            Ok(commits) => {
                                self.commits = commits;
                                self.error_message = None;
                            }
                            Err(e) => {
                                let git_error = GitError::command_failed("search", e.to_string());
                                ErrorReporter::log_error(&git_error, "commit search");
                                self.error_message =
                                    Some(ErrorRecovery::user_friendly_message(&git_error));
                            }
                        }
                        self.loading = false;
                    }
                }
                Err(e) => {
                    let git_error = GitError::invalid_input(query, e.to_string());
                    ErrorReporter::log_error(&git_error, "search query sanitization");
                    self.error_message = Some(ErrorRecovery::user_friendly_message(&git_error));
                }
            }
        }
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn get_selected_commit(&self) -> Option<&GitCommit> {
        if let Some(ref selected_id) = self.selected_commit {
            self.commits.iter().find(|c| &c.id == selected_id)
        } else {
            None
        }
    }

    pub fn has_repository(&self) -> bool {
        self.repository.is_some()
    }

    pub fn repository_info(&self) -> Option<&RepositoryInfo> {
        self.repository.as_ref().map(|r| r.info())
    }

    pub fn start_streaming_commits(&mut self) {
        if let Some(ref repo) = self.repository {
            self.loading = true;
            self.commits.clear();
            self.stream_complete = false;

            match repo.get_commits_streaming(Some(self.commit_limit)) {
                Ok(stream) => {
                    self.commit_stream = Some(stream);
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to start streaming commits: {}", e));
                    self.loading = false;
                }
            }
        }
    }

    pub fn poll_commit_stream(&mut self) -> bool {
        if let Some(ref mut stream) = self.commit_stream {
            let mut progress_made = false;
            // Poll for new commits (non-blocking)
            while let Some(commit_result) = stream.try_next() {
                match commit_result {
                    Ok(commit) => {
                        tracing::debug!("Polled commit: {} - {}", commit.id, commit.message.lines().next().unwrap_or(""));
                        self.commits.push(commit);
                        progress_made = true;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Error loading commit: {}", e));
                        self.loading = false;
                        self.commit_stream = None;
                        return false;
                    }
                }
            }

            // Check if stream is complete
            if stream.is_complete() {
                tracing::debug!("Commit stream completed, total commits loaded: {}", self.commits.len());
                self.stream_complete = true;
                self.loading = false;
                self.commit_stream = None;
            }

            progress_made
        } else {
            false
        }
    }

    pub fn is_streaming(&self) -> bool {
        self.commit_stream.is_some()
    }

    pub fn load_references(&mut self) {
        if let Some(ref repo) = self.repository {
            match repo.get_ref_manager() {
                Ok(ref_manager) => {
                    // Set current branch if available
                    self.selected_branch = ref_manager.get_current_branch();
                    self.ref_manager = Some(ref_manager);
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to load references: {}", e));
                }
            }
        }
    }

    pub fn get_branches(&self) -> Vec<String> {
        self.ref_manager
            .as_ref()
            .map(|rm| {
                let mut branches = Vec::new();

                // Add local branches
                for branch in rm.get_local_branches() {
                    branches.push(branch.name.clone());
                }

                // Add remote branches if enabled
                if self.show_remote_branches {
                    for branch in rm.get_remote_branches() {
                        branches.push(branch.name.clone());
                    }
                }

                branches
            })
            .unwrap_or_default()
    }

    pub fn get_tags(&self) -> Vec<String> {
        self.ref_manager
            .as_ref()
            .map(|rm| rm.get_tags().iter().map(|tag| tag.name.clone()).collect())
            .unwrap_or_default()
    }

    pub fn get_current_branch(&self) -> Option<&String> {
        self.selected_branch.as_ref()
    }

    pub fn is_detached_head(&self) -> bool {
        self.ref_manager
            .as_ref()
            .map(|rm| rm.is_detached_head())
            .unwrap_or(false)
    }

    pub fn switch_to_branch(&mut self, branch_name: &str) {
        if let Some(ref repo) = self.repository {
            // Sanitize branch name for security
            match InputSanitizer::sanitize_ref_name(branch_name) {
                Ok(sanitized_name) => {
                    // Check if branch exists
                    if let Some(ref rm) = self.ref_manager {
                        let full_name = format!("refs/heads/{}", sanitized_name);
                        if let Some(_branch_ref) = rm.get_ref(&full_name) {
                            self.selected_branch = Some(sanitized_name);
                            // Start streaming commits from this branch
                            self.start_streaming_commits();
                        } else {
                            self.error_message =
                                Some(format!("Branch '{}' not found", sanitized_name));
                        }
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Invalid branch name: {}", e));
                }
            }
        }
    }

    pub fn get_refs_for_commit(&self, commit_sha: &str) -> Vec<String> {
        self.ref_manager
            .as_ref()
            .map(|rm| {
                rm.get_refs_for_commit(commit_sha)
                    .into_iter()
                    .map(|git_ref| git_ref.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn refresh_references(&mut self) {
        self.load_references();
    }

    pub fn initialize_views(&mut self) {
        if let Some(ref repo) = self.repository {
            let mut view_manager = repo.create_view_manager();

            // Initialize the default view with current repository commits
            if let Err(e) = view_manager.update_current_view(repo) {
                self.error_message = Some(format!("Failed to initialize views: {}", e));
            }

            self.view_manager = Some(view_manager);
        }
    }

    pub fn get_filtered_commits(&self) -> &[GitCommit] {
        if let Some(ref view_manager) = self.view_manager {
            if let Some(current_view) = view_manager.get_current_view() {
                tracing::debug!("Using view manager commits: {} commits", current_view.commits.len());
                // If view manager has empty commits but we have loaded commits, use loaded commits instead
                if current_view.commits.is_empty() && !self.commits.is_empty() {
                    tracing::debug!("View manager commits empty, falling back to loaded commits: {} commits", self.commits.len());
                    &self.commits
                } else {
                    &current_view.commits
                }
            } else {
                tracing::debug!("No current view, using direct commits: {} commits", self.commits.len());
                &self.commits
            }
        } else {
            tracing::debug!("No view manager, using direct commits: {} commits", self.commits.len());
            &self.commits
        }
    }

    pub fn update_current_view(&mut self) {
        if let Some(ref mut view_manager) = self.view_manager {
            if let Some(ref repo) = self.repository {
                if let Err(e) = view_manager.update_current_view(repo) {
                    self.error_message = Some(format!("Failed to update view: {}", e));
                }
            }
        }
    }

    // Navigation methods for keyboard shortcuts
    pub fn navigate_commits(&mut self, delta: i32) {
        let commits_len = self.get_filtered_commits().len();
        if commits_len == 0 {
            return;
        }

        let current_index = self.selected_commit_index.unwrap_or(0);
        let new_index = if delta < 0 {
            current_index.saturating_sub((-delta) as usize)
        } else {
            (current_index + delta as usize).min(commits_len - 1)
        };

        if new_index < commits_len {
            let commit_id = self.get_filtered_commits()[new_index].id.clone();
            self.selected_commit_index = Some(new_index);
            self.select_commit(commit_id);
        }
    }

    pub fn navigate_to_first_commit(&mut self) {
        let commits = self.get_filtered_commits();
        if !commits.is_empty() {
            let commit_id = commits[0].id.clone();
            self.selected_commit_index = Some(0);
            self.select_commit(commit_id);
        }
    }

    pub fn navigate_to_last_commit(&mut self) {
        let commits = self.get_filtered_commits();
        if !commits.is_empty() {
            let last_index = commits.len() - 1;
            let commit_id = commits[last_index].id.clone();
            self.selected_commit_index = Some(last_index);
            self.select_commit(commit_id);
        }
    }
}
