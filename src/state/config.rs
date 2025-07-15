use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub window_size: (f32, f32),
    pub window_position: Option<(f32, f32)>,
    pub recent_repositories: Vec<PathBuf>,
    pub max_recent_repos: usize,
    pub commit_limit: usize,
    pub font_size: f32,
    pub theme: Theme,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub tab_size: usize,
    // Enhanced UI preferences
    pub auto_refresh_interval: Option<u64>, // seconds, None = disabled
    pub confirm_destructive_actions: bool,
    pub show_relative_dates: bool,
    pub compact_view: bool,
    pub branch_colors: BranchColorSettings,
    pub diff_settings: DiffSettings,
    pub layout_settings: LayoutSettings,
    pub performance_settings: PerformanceSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchColorSettings {
    pub use_branch_colors: bool,
    pub main_branch_color: String,        // hex color
    pub feature_branch_color: String,
    pub release_branch_color: String,
    pub hotfix_branch_color: String,
    pub custom_patterns: Vec<BranchPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchPattern {
    pub pattern: String,    // regex pattern
    pub color: String,      // hex color
    pub name: String,       // display name
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSettings {
    pub context_lines: u32,
    pub ignore_whitespace: bool,
    pub show_word_diff: bool,
    pub syntax_highlighting: bool,
    pub max_file_size_kb: u64,  // Skip diffing files larger than this
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSettings {
    pub default_layout_mode: String,  // "three_pane", "two_pane_h", "two_pane_v", "single"
    pub left_panel_width_ratio: f32,
    pub right_panel_width_ratio: f32,
    pub remember_panel_states: bool,
    pub auto_hide_empty_panels: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSettings {
    pub max_commits_to_load: usize,
    pub commit_batch_size: usize,
    pub enable_commit_streaming: bool,
    pub cache_diffs: bool,
    pub max_cached_diffs: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window_size: (1200.0, 800.0),
            window_position: None,
            recent_repositories: Vec::new(),
            max_recent_repos: 10,
            commit_limit: 1000,
            font_size: 14.0,
            theme: Theme::Auto,
            show_line_numbers: true,
            word_wrap: false,
            tab_size: 4,
            // Enhanced UI preferences
            auto_refresh_interval: None,
            confirm_destructive_actions: true,
            show_relative_dates: true,
            compact_view: false,
            branch_colors: BranchColorSettings::default(),
            diff_settings: DiffSettings::default(),
            layout_settings: LayoutSettings::default(),
            performance_settings: PerformanceSettings::default(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("gitk-rust").join("config.json");
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(config_dir) = dirs::config_dir() {
            let app_config_dir = config_dir.join("gitk-rust");
            std::fs::create_dir_all(&app_config_dir)?;
            
            let config_path = app_config_dir.join("config.json");
            let content = serde_json::to_string_pretty(self)?;
            std::fs::write(&config_path, content)?;
        }
        Ok(())
    }

    pub fn add_recent_repository(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_repositories.retain(|p| p != &path);
        
        // Add to front
        self.recent_repositories.insert(0, path);
        
        // Limit to max_recent_repos
        if self.recent_repositories.len() > self.max_recent_repos {
            self.recent_repositories.truncate(self.max_recent_repos);
        }
    }

    pub fn remove_recent_repository(&mut self, path: &PathBuf) {
        self.recent_repositories.retain(|p| p != path);
    }
}

impl Default for BranchColorSettings {
    fn default() -> Self {
        Self {
            use_branch_colors: true,
            main_branch_color: "#2E7D32".to_string(),      // Green
            feature_branch_color: "#1976D2".to_string(),   // Blue
            release_branch_color: "#F57C00".to_string(),   // Orange
            hotfix_branch_color: "#D32F2F".to_string(),    // Red
            custom_patterns: vec![
                BranchPattern {
                    pattern: "feature/.*".to_string(),
                    color: "#1976D2".to_string(),
                    name: "Feature".to_string(),
                },
                BranchPattern {
                    pattern: "hotfix/.*".to_string(),
                    color: "#D32F2F".to_string(),
                    name: "Hotfix".to_string(),
                },
            ],
        }
    }
}

impl Default for DiffSettings {
    fn default() -> Self {
        Self {
            context_lines: 3,
            ignore_whitespace: false,
            show_word_diff: true,
            syntax_highlighting: true,
            max_file_size_kb: 1024, // 1MB
        }
    }
}

impl Default for LayoutSettings {
    fn default() -> Self {
        Self {
            default_layout_mode: "three_pane".to_string(),
            left_panel_width_ratio: 0.4,
            right_panel_width_ratio: 0.3,
            remember_panel_states: true,
            auto_hide_empty_panels: false,
        }
    }
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            max_commits_to_load: 2000,
            commit_batch_size: 100,
            enable_commit_streaming: true,
            cache_diffs: true,
            max_cached_diffs: 50,
        }
    }
}