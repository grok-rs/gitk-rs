use crate::git::GitRepository;
use crate::models::GitCommit;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewFilter {
    pub name: String,
    pub description: String,
    pub author_filter: Option<String>,
    pub committer_filter: Option<String>,
    pub message_filter: Option<String>,
    pub file_filter: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub branch_filter: Option<String>,
    pub max_commits: Option<usize>,
    pub include_merges: bool,
    pub case_sensitive: bool,
    pub use_regex: bool,
}

impl Default for ViewFilter {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            description: "Show all commits".to_string(),
            author_filter: None,
            committer_filter: None,
            message_filter: None,
            file_filter: None,
            date_from: None,
            date_to: None,
            branch_filter: None,
            max_commits: Some(1000),
            include_merges: true,
            case_sensitive: false,
            use_regex: false,
        }
    }
}

impl ViewFilter {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: format!("Custom view: {}", name),
            ..Default::default()
        }
    }

    /// Check if a commit matches this filter
    pub fn matches_commit(&self, commit: &GitCommit) -> bool {
        // Author filter
        if let Some(ref author_filter) = self.author_filter {
            if !self.text_matches(&commit.author.name, author_filter)
                && !self.text_matches(&commit.author.email, author_filter)
            {
                return false;
            }
        }

        // Committer filter
        if let Some(ref committer_filter) = self.committer_filter {
            if !self.text_matches(&commit.committer.name, committer_filter)
                && !self.text_matches(&commit.committer.email, committer_filter)
            {
                return false;
            }
        }

        // Message filter
        if let Some(ref message_filter) = self.message_filter {
            if !self.text_matches(&commit.message, message_filter) {
                return false;
            }
        }

        // Merge filter
        if !self.include_merges && commit.parent_ids.len() > 1 {
            return false;
        }

        // Date filters would be implemented here
        // File filters would require additional commit analysis

        true
    }

    fn text_matches(&self, text: &str, filter: &str) -> bool {
        if self.use_regex {
            // TODO: Implement regex matching
            // For now, fall back to simple matching
            self.simple_text_match(text, filter)
        } else {
            self.simple_text_match(text, filter)
        }
    }

    fn simple_text_match(&self, text: &str, filter: &str) -> bool {
        if self.case_sensitive {
            text.contains(filter)
        } else {
            text.to_lowercase().contains(&filter.to_lowercase())
        }
    }

    /// Generate git rev-list arguments for this filter
    pub fn to_git_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Add branch filter
        if let Some(ref branch) = self.branch_filter {
            args.push(branch.clone());
        } else {
            args.push("HEAD".to_string());
        }

        // Add author filter
        if let Some(ref author) = self.author_filter {
            args.push(format!("--author={}", author));
        }

        // Add committer filter
        if let Some(ref committer) = self.committer_filter {
            args.push(format!("--committer={}", committer));
        }

        // Add message filter (grep)
        if let Some(ref message) = self.message_filter {
            args.push(format!("--grep={}", message));
        }

        // Add date filters
        if let Some(ref date_from) = self.date_from {
            args.push(format!("--since={}", date_from));
        }
        if let Some(ref date_to) = self.date_to {
            args.push(format!("--until={}", date_to));
        }

        // Add merge filter
        if !self.include_merges {
            args.push("--no-merges".to_string());
        }

        // Add max commits
        if let Some(max) = self.max_commits {
            args.push(format!("--max-count={}", max));
        }

        // Add file filter
        if let Some(ref file) = self.file_filter {
            args.push("--".to_string());
            args.push(file.clone());
        }

        args
    }
}

#[derive(Debug, Clone)]
pub struct GitView {
    pub filter: ViewFilter,
    pub commits: Vec<GitCommit>,
    pub is_loading: bool,
    pub last_updated: Option<std::time::SystemTime>,
}

impl GitView {
    pub fn new(filter: ViewFilter) -> Self {
        Self {
            filter,
            commits: Vec::new(),
            is_loading: false,
            last_updated: None,
        }
    }

    pub fn update_commits(&mut self, repo: &GitRepository) -> Result<()> {
        self.is_loading = true;

        // Use git rev-list with filter arguments for efficient filtering
        let args = self.filter.to_git_args();
        let git_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        match repo.get_commits_from_git_args(&git_args) {
            Ok(commits) => {
                // Apply additional filters that git can't handle
                self.commits = commits
                    .into_iter()
                    .filter(|commit| self.filter.matches_commit(commit))
                    .collect();

                self.last_updated = Some(std::time::SystemTime::now());
                self.is_loading = false;
                Ok(())
            }
            Err(e) => {
                self.is_loading = false;
                Err(e)
            }
        }
    }

    pub fn refresh(&mut self, repo: &GitRepository) -> Result<()> {
        self.update_commits(repo)
    }

    pub fn is_stale(&self, max_age: std::time::Duration) -> bool {
        match self.last_updated {
            Some(last_updated) => {
                std::time::SystemTime::now()
                    .duration_since(last_updated)
                    .unwrap_or(std::time::Duration::MAX)
                    > max_age
            }
            None => true,
        }
    }
}

#[derive(Debug)]
pub struct ViewManager {
    views: HashMap<String, GitView>,
    current_view: String,
    default_view: String,
}

impl ViewManager {
    pub fn new() -> Self {
        let mut views = HashMap::new();
        let default_filter = ViewFilter::default();
        let default_view = GitView::new(default_filter);
        let default_name = "Default".to_string();

        views.insert(default_name.clone(), default_view);

        Self {
            views,
            current_view: default_name.clone(),
            default_view: default_name,
        }
    }

    pub fn add_view(&mut self, name: String, filter: ViewFilter) {
        let view = GitView::new(filter);
        self.views.insert(name, view);
    }

    pub fn remove_view(&mut self, name: &str) -> Result<()> {
        if name == self.default_view {
            return Err(anyhow::anyhow!("Cannot remove default view"));
        }

        if name == self.current_view {
            self.current_view = self.default_view.clone();
        }

        self.views.remove(name);
        Ok(())
    }

    pub fn switch_view(&mut self, name: &str) -> Result<()> {
        if self.views.contains_key(name) {
            self.current_view = name.to_string();
            Ok(())
        } else {
            Err(anyhow::anyhow!("View '{}' not found", name))
        }
    }

    pub fn get_current_view(&self) -> Option<&GitView> {
        self.views.get(&self.current_view)
    }

    pub fn get_current_view_mut(&mut self) -> Option<&mut GitView> {
        self.views.get_mut(&self.current_view)
    }

    pub fn get_view(&self, name: &str) -> Option<&GitView> {
        self.views.get(name)
    }

    pub fn get_view_mut(&mut self, name: &str) -> Option<&mut GitView> {
        self.views.get_mut(name)
    }

    pub fn get_view_names(&self) -> Vec<String> {
        self.views.keys().cloned().collect()
    }

    pub fn get_current_view_name(&self) -> &str {
        &self.current_view
    }

    pub fn update_current_view(&mut self, repo: &GitRepository) -> Result<()> {
        if let Some(view) = self.get_current_view_mut() {
            view.update_commits(repo)
        } else {
            Err(anyhow::anyhow!("No current view"))
        }
    }

    pub fn refresh_view(&mut self, name: &str, repo: &GitRepository) -> Result<()> {
        if let Some(view) = self.get_view_mut(name) {
            view.refresh(repo)
        } else {
            Err(anyhow::anyhow!("View '{}' not found", name))
        }
    }

    pub fn refresh_all_views(&mut self, repo: &GitRepository) -> Result<()> {
        let view_names: Vec<String> = self.views.keys().cloned().collect();

        for name in view_names {
            if let Err(e) = self.refresh_view(&name, repo) {
                tracing::warn!("Failed to refresh view '{}': {}", name, e);
            }
        }

        Ok(())
    }

    pub fn cleanup_stale_views(&mut self, max_age: std::time::Duration) {
        for view in self.views.values_mut() {
            if view.is_stale(max_age) {
                view.commits.clear();
                view.last_updated = None;
            }
        }
    }
}

// Extension to GitRepository for view support
impl GitRepository {
    /// Get commits using git rev-list arguments
    pub fn get_commits_from_git_args(&self, args: &[&str]) -> Result<Vec<GitCommit>> {
        let output = self.commands().rev_list(args)?;

        let mut commits = Vec::new();
        for line in output.lines() {
            if let Ok(oid) = git2::Oid::from_str(line.trim()) {
                if let Ok(commit) = self.repo().find_commit(oid) {
                    if let Ok(git_commit) = GitCommit::new(&commit) {
                        commits.push(git_commit);
                    }
                }
            }
        }

        Ok(commits)
    }

    /// Create a view manager for this repository
    pub fn create_view_manager(&self) -> ViewManager {
        ViewManager::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewPreset {
    pub name: String,
    pub filter: ViewFilter,
}

impl ViewPreset {
    pub fn create_common_presets() -> Vec<ViewPreset> {
        vec![
            ViewPreset {
                name: "Recent".to_string(),
                filter: ViewFilter {
                    name: "Recent".to_string(),
                    description: "Commits from the last 30 days".to_string(),
                    date_from: Some("30.days.ago".to_string()),
                    max_commits: Some(500),
                    ..Default::default()
                },
            },
            ViewPreset {
                name: "My Commits".to_string(),
                filter: ViewFilter {
                    name: "My Commits".to_string(),
                    description: "Commits by current user".to_string(),
                    // author_filter would be set to current user
                    max_commits: Some(1000),
                    ..Default::default()
                },
            },
            ViewPreset {
                name: "No Merges".to_string(),
                filter: ViewFilter {
                    name: "No Merges".to_string(),
                    description: "All commits excluding merges".to_string(),
                    include_merges: false,
                    max_commits: Some(1000),
                    ..Default::default()
                },
            },
            ViewPreset {
                name: "Bug Fixes".to_string(),
                filter: ViewFilter {
                    name: "Bug Fixes".to_string(),
                    description: "Commits containing 'fix' or 'bug'".to_string(),
                    message_filter: Some("fix|bug".to_string()),
                    use_regex: true,
                    case_sensitive: false,
                    max_commits: Some(500),
                    ..Default::default()
                },
            },
            ViewPreset {
                name: "Features".to_string(),
                filter: ViewFilter {
                    name: "Features".to_string(),
                    description: "Commits containing 'feat' or 'feature'".to_string(),
                    message_filter: Some("feat|feature".to_string()),
                    use_regex: true,
                    case_sensitive: false,
                    max_commits: Some(500),
                    ..Default::default()
                },
            },
        ]
    }
}
