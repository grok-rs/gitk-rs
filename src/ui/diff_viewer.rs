#![allow(dead_code)]

use crate::models::{DiffStatus, GitDiff, GitDiffLine};
use crate::state::{AppConfig, AppState};
use eframe::egui;
use regex::Regex;
use std::collections::HashMap;

pub struct DiffViewer {
    show_line_numbers: bool,
    font_size: f32,
    view_mode: DiffViewMode,
    word_wrap: bool,
    show_whitespace: bool,
    context_lines: u32,
    split_ratio: f32,
    current_file_index: usize,
    file_tree_width: f32,
    scroll_sync: bool,
    file_tree_expanded: HashMap<String, bool>,
    show_file_tree: bool,
    left_scroll: egui::Vec2,
    right_scroll: egui::Vec2,
    syntax_highlight: bool,
    folded_hunks: HashMap<usize, bool>,
    search_text: String,
    search_matches: Vec<SearchMatch>,
    current_match: usize,
    syntax_highlighter: SyntaxHighlighter,
    word_diff_engine: WordDiffEngine,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiffViewMode {
    Unified,       // Traditional unified diff
    SideBySide,    // Side-by-side comparison
    Split,         // Split view with file tree
    InlineChanges, // Inline word-level changes
}

#[derive(Debug, Clone)]
struct SearchMatch {
    file_index: usize,
    hunk_index: usize,
    line_index: usize,
    start_char: usize,
    end_char: usize,
    side: DiffSide,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DiffSide {
    Left,
    Right,
    Both,
}

#[derive(Debug, Clone)]

struct DiffLine {
    old_line: Option<String>,
    new_line: Option<String>,
    old_line_no: Option<u32>,
    new_line_no: Option<u32>,
    change_type: LineChangeType,
    word_changes: Vec<WordChange>,
}

#[derive(Debug, Clone, PartialEq)]

enum LineChangeType {
    Context,  // Unchanged line
    Added,    // Added line
    Removed,  // Removed line
    Modified, // Modified line (has both old and new)
}

#[derive(Debug, Clone)]

struct WordChange {
    start: usize,
    end: usize,
    change_type: WordChangeType,
}

#[derive(Debug, Clone, PartialEq)]

enum WordChangeType {
    Added,
    Removed,
    Modified,
}

/// Syntax highlighter for various programming languages
#[derive(Debug)]

struct SyntaxHighlighter {
    language_patterns: HashMap<String, LanguageHighlighter>,
    cache: HashMap<String, Vec<SyntaxToken>>,
}

/// Language-specific syntax highlighting patterns
#[derive(Debug, Clone)]

struct LanguageHighlighter {
    language: ProgrammingLanguage,
    keywords: Vec<String>,
    operators: Vec<String>,
    patterns: Vec<SyntaxPattern>,
}

/// Supported programming languages
#[derive(Debug, Clone, PartialEq)]

enum ProgrammingLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Html,
    Css,
    Json,
    Yaml,
    Markdown,
    Shell,
    Sql,
    Unknown,
}

/// Syntax highlighting patterns
#[derive(Debug, Clone)]

struct SyntaxPattern {
    regex: Regex,
    token_type: TokenType,
    priority: u8,
}

/// Types of syntax tokens
#[derive(Debug, Clone, PartialEq)]

enum TokenType {
    Keyword,
    String,
    Comment,
    Number,
    Operator,
    Function,
    Type,
    Variable,
    Constant,
    Preprocessor,
    Error,
    Normal,
}

/// Syntax token with position and type
#[derive(Debug, Clone)]

struct SyntaxToken {
    start: usize,
    end: usize,
    token_type: TokenType,
    text: String,
}

/// Word-level diff engine for detecting fine-grained changes
#[derive(Debug)]

struct WordDiffEngine {
    word_boundary_regex: Regex,
    cache: HashMap<String, WordDiffResult>,
}

/// Result of word-level diff comparison
#[derive(Debug, Clone)]

struct WordDiffResult {
    old_words: Vec<DiffWord>,
    new_words: Vec<DiffWord>,
    operations: Vec<DiffOperation>,
}

/// Individual word in diff with change status
#[derive(Debug, Clone)]

struct DiffWord {
    text: String,
    start_pos: usize,
    end_pos: usize,
    change_type: WordChangeType,
}

/// Diff operation for word-level changes
#[derive(Debug, Clone)]

enum DiffOperation {
    Equal(String),
    Insert(String),
    Delete(String),
    Replace(String, String),
}

/// Hierarchical file tree for enhanced navigation
#[derive(Debug)]

struct FileTree {
    root: FileTreeNode,
}

/// Node in the file tree
#[derive(Debug)]

struct FileTreeNode {
    children: HashMap<String, FileTreeNode>,
    file_index: Option<usize>,
    is_dir: bool,
}

impl FileTree {
    fn new() -> Self {
        Self {
            root: FileTreeNode::new_dir(),
        }
    }

    fn insert_file(&mut self, path_parts: Vec<&str>, file_idx: usize, _diff: &GitDiff) {
        let mut current = &mut self.root;

        // Navigate/create directories
        for (i, part) in path_parts.iter().enumerate() {
            if i == path_parts.len() - 1 {
                // Last part is the file
                current
                    .children
                    .insert(part.to_string(), FileTreeNode::new_file(file_idx));
            } else {
                // Intermediate directories
                let part_string = part.to_string();
                current
                    .children
                    .entry(part_string.clone())
                    .or_insert_with(FileTreeNode::new_dir);
                current = current.children.get_mut(&part_string).unwrap();
            }
        }
    }
}

impl FileTreeNode {
    fn new_dir() -> Self {
        Self {
            children: HashMap::new(),
            file_index: None,
            is_dir: true,
        }
    }

    fn new_file(file_idx: usize) -> Self {
        Self {
            children: HashMap::new(),
            file_index: Some(file_idx),
            is_dir: false,
        }
    }

    fn is_directory(&self) -> bool {
        self.is_dir
    }
}

impl DiffViewer {
    pub fn new() -> Self {
        Self {
            show_line_numbers: true,
            font_size: 14.0,
            view_mode: DiffViewMode::SideBySide,
            word_wrap: false,
            show_whitespace: false,
            context_lines: 3,
            split_ratio: 0.5,
            current_file_index: 0,
            file_tree_width: 300.0,
            scroll_sync: true,
            file_tree_expanded: HashMap::new(),
            show_file_tree: true,
            left_scroll: egui::Vec2::ZERO,
            right_scroll: egui::Vec2::ZERO,
            syntax_highlight: true,
            folded_hunks: HashMap::new(),
            search_text: String::new(),
            search_matches: Vec::new(),
            current_match: 0,
            syntax_highlighter: SyntaxHighlighter::new(),
            word_diff_engine: WordDiffEngine::new(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, state: &mut AppState, config: &AppConfig) {
        // Update configuration
        self.show_line_numbers = config.show_line_numbers;
        self.font_size = config.font_size;

        // Header with controls
        ui.horizontal(|ui| {
            ui.heading("Diff Viewer");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // View mode selector
                egui::ComboBox::from_label("View")
                    .selected_text(format!("{:?}", self.view_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.view_mode, DiffViewMode::Unified, "Unified");
                        ui.selectable_value(
                            &mut self.view_mode,
                            DiffViewMode::SideBySide,
                            "Side by Side",
                        );
                        ui.selectable_value(&mut self.view_mode, DiffViewMode::Split, "Split View");
                        ui.selectable_value(
                            &mut self.view_mode,
                            DiffViewMode::InlineChanges,
                            "Inline Changes",
                        );
                    });

                ui.separator();

                // View options
                ui.checkbox(&mut self.syntax_highlight, "🎨 Syntax");
                ui.checkbox(&mut self.show_whitespace, "⎵ Whitespace");
                ui.checkbox(&mut self.word_wrap, "📄 Wrap");
                ui.checkbox(&mut self.scroll_sync, "🔗 Sync Scroll");
                ui.checkbox(&mut self.show_file_tree, "🌳 Files");

                ui.separator();

                // Search
                ui.label("🔍");
                let search_response = ui.text_edit_singleline(&mut self.search_text);
                if search_response.changed() {
                    self.update_search_matches(state);
                }

                if !self.search_matches.is_empty() {
                    ui.label(format!(
                        "{}/{}",
                        self.current_match + 1,
                        self.search_matches.len()
                    ));
                    if ui.button("⏪").clicked() && self.current_match > 0 {
                        self.current_match -= 1;
                    }
                    if ui.button("⏩").clicked()
                        && self.current_match < self.search_matches.len() - 1
                    {
                        self.current_match += 1;
                    }
                }
            });
        });

        ui.separator();

        // Get available diffs
        let diffs = if let Some(ref selected_commit) = state.get_selected_commit() {
            if let Some(ref repo) = state.repository {
                match repo.get_commit_diff_enhanced(&selected_commit.id) {
                    Ok(diffs) => diffs,
                    Err(e) => {
                        ui.label(format!("Error loading diffs: {}", e));
                        return;
                    }
                }
            } else {
                Vec::new()
            }
        } else if let Some(ref current_diff) = state.current_diff {
            vec![current_diff.clone()]
        } else {
            ui.vertical_centered(|ui| {
                ui.label("Select a commit to view diff");
            });
            return;
        };

        if diffs.is_empty() {
            ui.vertical_centered(|ui| {
                ui.label("No changes to display");
            });
            return;
        }

        // Show file navigation bar (except for split view which has its own tree)
        if self.view_mode != DiffViewMode::Split && diffs.len() > 1 {
            self.show_file_navigation_bar(ui, &diffs);
            ui.separator();
        }

        // Show diff content based on view mode
        match self.view_mode {
            DiffViewMode::Unified => self.show_unified_view(ui, &diffs, state),
            DiffViewMode::SideBySide => self.show_side_by_side_view(ui, &diffs, state),
            DiffViewMode::Split => self.show_split_view(ui, &diffs, state),
            DiffViewMode::InlineChanges => self.show_inline_changes_view(ui, &diffs, state),
        }
    }

    /// Show unified diff view (traditional single-column diff)
    fn show_unified_view(&self, ui: &mut egui::Ui, diffs: &[GitDiff], _state: &AppState) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (file_idx, diff) in diffs.iter().enumerate() {
                self.show_file_header(ui, diff, file_idx);

                if diff.is_binary {
                    self.show_binary_file_info(ui, diff);
                    continue;
                }

                for (hunk_idx, hunk) in diff.hunks.iter().enumerate() {
                    self.show_hunk_header(ui, hunk, file_idx, hunk_idx);

                    if !self
                        .folded_hunks
                        .get(&(file_idx * 1000 + hunk_idx))
                        .unwrap_or(&false)
                    {
                        for line in &hunk.lines {
                            self.show_unified_diff_line(ui, line);
                        }
                    }
                    ui.separator();
                }
            }
        });
    }

    /// Show side-by-side diff view
    fn show_side_by_side_view(&mut self, ui: &mut egui::Ui, diffs: &[GitDiff], _state: &AppState) {
        if diffs.is_empty() {
            return;
        }

        let diff = &diffs[self.current_file_index.min(diffs.len() - 1)];

        // File navigation if multiple files
        if diffs.len() > 1 {
            ui.horizontal(|ui| {
                ui.label("File:");
                if ui.button("⏪").clicked() && self.current_file_index > 0 {
                    self.current_file_index -= 1;
                }
                ui.label(format!("{}/{}", self.current_file_index + 1, diffs.len()));
                if ui.button("⏩").clicked() && self.current_file_index < diffs.len() - 1 {
                    self.current_file_index += 1;
                }

                ui.separator();
                ui.label(self.get_file_display_name(diff));
            });
            ui.separator();
        }

        if diff.is_binary {
            self.show_binary_file_info(ui, diff);
            return;
        }

        // Side-by-side layout
        ui.horizontal(|ui| {
            let available_width = ui.available_width();
            let left_width = available_width * self.split_ratio;
            let right_width = available_width - left_width - 10.0; // 10px for separator
                                                                   // Left side (old/removed)
            ui.allocate_ui_with_layout(
                egui::vec2(left_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.label("Before");
                    ui.separator();
                    self.show_side_by_side_content(ui, diff, DiffSide::Left);
                },
            );

            ui.separator();

            // Right side (new/added)
            ui.allocate_ui_with_layout(
                egui::vec2(right_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.label("After");
                    ui.separator();
                    self.show_side_by_side_content(ui, diff, DiffSide::Right);
                },
            );
        });
    }

    /// Show split view with file tree
    fn show_split_view(&mut self, ui: &mut egui::Ui, diffs: &[GitDiff], _state: &AppState) {
        ui.horizontal(|ui| {
            // File tree panel (only show if enabled)
            if self.show_file_tree {
                ui.allocate_ui_with_layout(
                    egui::vec2(self.file_tree_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        self.show_enhanced_file_tree(ui, diffs);
                    },
                );

                ui.separator();
            }

            // Main diff content
            if self.current_file_index < diffs.len() {
                self.show_side_by_side_view(ui, &[diffs[self.current_file_index].clone()], _state);
            }
        });
    }

    /// Show inline changes view with word-level highlighting
    fn show_inline_changes_view(&self, ui: &mut egui::Ui, diffs: &[GitDiff], _state: &AppState) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (file_idx, diff) in diffs.iter().enumerate() {
                self.show_file_header(ui, diff, file_idx);

                if diff.is_binary {
                    self.show_binary_file_info(ui, diff);
                    continue;
                }

                // Process diff into line-by-line changes with word-level detection
                let processed_lines = self.process_diff_for_inline_changes(diff);

                for line in processed_lines {
                    self.show_inline_change_line(ui, &line);
                }
            }
        });
    }

    /// Show file header with status and stats
    fn show_file_header(&self, ui: &mut egui::Ui, diff: &GitDiff, file_idx: usize) {
        ui.horizontal(|ui| {
            // Expand/collapse button for file
            let is_expanded = !self.folded_hunks.get(&(file_idx * 10000)).unwrap_or(&false);
            let expand_icon = if is_expanded { "▼" } else { "▶" };
            if ui.button(expand_icon).clicked() {
                // Toggle file expansion (would need mutable access)
            }

            // File status and name
            match diff.status {
                DiffStatus::Added => {
                    ui.colored_label(egui::Color32::GREEN, "➕ Added:");
                }
                DiffStatus::Deleted => {
                    ui.colored_label(egui::Color32::RED, "❌ Deleted:");
                }
                DiffStatus::Renamed => {
                    ui.colored_label(egui::Color32::BLUE, "📝 Renamed:");
                }
                DiffStatus::Copied => {
                    ui.colored_label(egui::Color32::LIGHT_BLUE, "📋 Copied:");
                }
                DiffStatus::Modified => {
                    ui.colored_label(egui::Color32::YELLOW, "📄 Modified:");
                }
                DiffStatus::Typechange => {
                    ui.colored_label(egui::Color32::ORANGE, "🔄 Type changed:");
                }
                _ => {
                    ui.label("📄");
                }
            }

            ui.label(self.get_file_display_name(diff));

            // Stats
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if diff.stats.deletions > 0 {
                    ui.colored_label(egui::Color32::RED, format!("-{}", diff.stats.deletions));
                }
                if diff.stats.insertions > 0 {
                    ui.colored_label(egui::Color32::GREEN, format!("+{}", diff.stats.insertions));
                }
            });
        });
        ui.separator();
    }

    /// Show binary file information
    fn show_binary_file_info(&self, ui: &mut egui::Ui, diff: &GitDiff) {
        ui.indent("binary_info", |ui| {
            ui.colored_label(
                egui::Color32::LIGHT_GRAY,
                "📁 Binary file - content not shown",
            );
            if diff.stats.files_changed > 0 {
                ui.label(format!("File size changed"));
            }
        });
    }

    /// Show hunk header with folding capability
    fn show_hunk_header(
        &self,
        ui: &mut egui::Ui,
        hunk: &crate::models::GitHunk,
        file_idx: usize,
        hunk_idx: usize,
    ) {
        ui.horizontal(|ui| {
            // Fold/unfold button
            let fold_key = file_idx * 1000 + hunk_idx;
            let is_folded = self.folded_hunks.get(&fold_key).unwrap_or(&false);
            let fold_icon = if *is_folded { "▶" } else { "▼" };
            if ui.button(fold_icon).clicked() {
                // Toggle hunk folding (would need mutable access)
            }

            // Hunk info
            ui.colored_label(
                egui::Color32::from_rgb(100, 100, 200),
                format!(
                    "@@ -{},{} +{},{} @@",
                    hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines
                ),
            );

            // Context information if available (placeholder for future enhancement)
            // if !hunk.context.is_empty() {
            //     ui.colored_label(egui::Color32::GRAY, &hunk.context);
            // }
        });
    }

    /// Show unified diff line
    fn show_unified_diff_line(&self, ui: &mut egui::Ui, line: &GitDiffLine) {
        let (background_color, text_color, prefix) = match line.origin {
            '+' => (
                egui::Color32::from_rgba_unmultiplied(0, 100, 0, 30),
                egui::Color32::from_rgb(0, 150, 0),
                "+",
            ),
            '-' => (
                egui::Color32::from_rgba_unmultiplied(100, 0, 0, 30),
                egui::Color32::from_rgb(150, 0, 0),
                "-",
            ),
            _ => (egui::Color32::TRANSPARENT, ui.visuals().text_color(), " "),
        };

        // Highlight search matches
        let is_search_match = self.is_line_search_match(line);
        let final_background = if is_search_match {
            egui::Color32::YELLOW.gamma_multiply(0.3)
        } else {
            background_color
        };

        let rect = ui.available_rect_before_wrap();
        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(rect.width(), self.font_size + 4.0),
            egui::Sense::hover(),
        );

        // Draw background
        if final_background != egui::Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, 0.0, final_background);
        }

        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.horizontal(|ui| {
                // Line numbers
                if self.show_line_numbers {
                    let old_num = line
                        .old_lineno
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "   ".to_string());
                    let new_num = line
                        .new_lineno
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "   ".to_string());

                    ui.monospace(format!("{:>4} {:>4}", old_num, new_num));
                    ui.separator();
                }

                // Content with syntax highlighting if enabled
                let content = format!("{}{}", prefix, line.content.trim_end());
                if self.syntax_highlight {
                    self.show_syntax_highlighted_text(ui, &content, text_color);
                } else {
                    ui.colored_label(text_color, content);
                }
            });
        });
    }

    /// Show side-by-side content for one side
    fn show_side_by_side_content(&self, ui: &mut egui::Ui, diff: &GitDiff, side: DiffSide) {
        let scroll_area = egui::ScrollArea::both().id_salt(format!("diff_{:?}", side));

        scroll_area.show(ui, |ui| {
            for hunk in &diff.hunks {
                for line in &hunk.lines {
                    let should_show = match (side, line.origin) {
                        (DiffSide::Left, '-' | ' ') => true,
                        (DiffSide::Right, '+' | ' ') => true,
                        _ => false,
                    };

                    if should_show {
                        self.show_side_by_side_line(ui, line, side);
                    } else if matches!(side, DiffSide::Left) && line.origin == '+' {
                        // Show empty line on left for additions
                        self.show_empty_line(ui);
                    } else if matches!(side, DiffSide::Right) && line.origin == '-' {
                        // Show empty line on right for deletions
                        self.show_empty_line(ui);
                    }
                }
            }
        });
    }

    /// Show a line in side-by-side view
    fn show_side_by_side_line(&self, ui: &mut egui::Ui, line: &GitDiffLine, side: DiffSide) {
        let (background_color, text_color) = match line.origin {
            '+' => (
                egui::Color32::from_rgba_unmultiplied(0, 100, 0, 30),
                egui::Color32::from_rgb(0, 150, 0),
            ),
            '-' => (
                egui::Color32::from_rgba_unmultiplied(100, 0, 0, 30),
                egui::Color32::from_rgb(150, 0, 0),
            ),
            _ => (egui::Color32::TRANSPARENT, ui.visuals().text_color()),
        };

        let rect = ui.available_rect_before_wrap();
        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(rect.width(), self.font_size + 4.0),
            egui::Sense::hover(),
        );

        if background_color != egui::Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, 0.0, background_color);
        }

        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.horizontal(|ui| {
                // Line number
                if self.show_line_numbers {
                    let line_num = match side {
                        DiffSide::Left => line.old_lineno,
                        DiffSide::Right => line.new_lineno,
                        DiffSide::Both => line.new_lineno.or(line.old_lineno),
                    };

                    let num_str = line_num
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "   ".to_string());
                    ui.monospace(format!("{:>4}", num_str));
                    ui.separator();
                }

                // Content
                if self.syntax_highlight {
                    self.show_syntax_highlighted_text(ui, line.content.trim_end(), text_color);
                } else {
                    ui.colored_label(text_color, line.content.trim_end());
                }
            });
        });
    }

    /// Show empty line placeholder
    fn show_empty_line(&self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let (_rect, _response) = ui.allocate_exact_size(
            egui::vec2(rect.width(), self.font_size + 4.0),
            egui::Sense::hover(),
        );
    }

    /// Get display name for a file
    fn get_file_display_name(&self, diff: &GitDiff) -> String {
        match (&diff.old_file, &diff.new_file) {
            (Some(old), Some(new)) if old != new => format!("{} → {}", old, new),
            (Some(file), None) => file.clone(),
            (None, Some(file)) => file.clone(),
            (Some(file), Some(_)) => file.clone(),
            _ => "Unknown file".to_string(),
        }
    }

    /// Process diff for inline changes view
    fn process_diff_for_inline_changes(&self, diff: &GitDiff) -> Vec<DiffLine> {
        let mut result = Vec::new();

        for hunk in &diff.hunks {
            for line in &hunk.lines {
                match line.origin {
                    '+' => {
                        result.push(DiffLine {
                            old_line: None,
                            new_line: Some(line.content.clone()),
                            old_line_no: None,
                            new_line_no: line.new_lineno,
                            change_type: LineChangeType::Added,
                            word_changes: Vec::new(),
                        });
                    }
                    '-' => {
                        result.push(DiffLine {
                            old_line: Some(line.content.clone()),
                            new_line: None,
                            old_line_no: line.old_lineno,
                            new_line_no: None,
                            change_type: LineChangeType::Removed,
                            word_changes: Vec::new(),
                        });
                    }
                    _ => {
                        result.push(DiffLine {
                            old_line: Some(line.content.clone()),
                            new_line: Some(line.content.clone()),
                            old_line_no: line.old_lineno,
                            new_line_no: line.new_lineno,
                            change_type: LineChangeType::Context,
                            word_changes: Vec::new(),
                        });
                    }
                }
            }
        }

        result
    }

    /// Show inline change line with word-level highlighting
    fn show_inline_change_line(&self, ui: &mut egui::Ui, line: &DiffLine) {
        let background_color = match line.change_type {
            LineChangeType::Added => egui::Color32::from_rgba_unmultiplied(0, 100, 0, 30),
            LineChangeType::Removed => egui::Color32::from_rgba_unmultiplied(100, 0, 0, 30),
            LineChangeType::Modified => egui::Color32::from_rgba_unmultiplied(100, 100, 0, 30),
            LineChangeType::Context => egui::Color32::TRANSPARENT,
        };

        let rect = ui.available_rect_before_wrap();
        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(rect.width(), self.font_size + 4.0),
            egui::Sense::hover(),
        );

        if background_color != egui::Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, 0.0, background_color);
        }

        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.horizontal(|ui| {
                // Line numbers
                if self.show_line_numbers {
                    let old_num = line
                        .old_line_no
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "   ".to_string());
                    let new_num = line
                        .new_line_no
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "   ".to_string());
                    ui.monospace(format!("{:>4} {:>4}", old_num, new_num));
                    ui.separator();
                }

                // Content
                let default_content = String::new();
                let content = line
                    .new_line
                    .as_ref()
                    .or(line.old_line.as_ref())
                    .unwrap_or(&default_content);
                let text_color = match line.change_type {
                    LineChangeType::Added => egui::Color32::from_rgb(0, 150, 0),
                    LineChangeType::Removed => egui::Color32::from_rgb(150, 0, 0),
                    LineChangeType::Modified => egui::Color32::from_rgb(150, 150, 0),
                    LineChangeType::Context => ui.visuals().text_color(),
                };

                if self.syntax_highlight {
                    self.show_syntax_highlighted_text(ui, content.trim_end(), text_color);
                } else {
                    ui.colored_label(text_color, content.trim_end());
                }
            });
        });
    }

    /// Show syntax highlighted text (basic implementation)
    fn show_syntax_highlighted_text(
        &self,
        ui: &mut egui::Ui,
        text: &str,
        base_color: egui::Color32,
    ) {
        // For now, just show regular text
        // In a full implementation, this would parse the text based on file extension
        // and apply appropriate syntax highlighting
        ui.colored_label(base_color, text);
    }

    /// Check if a line matches current search
    fn is_line_search_match(&self, line: &GitDiffLine) -> bool {
        if self.search_text.is_empty() {
            return false;
        }
        line.content
            .to_lowercase()
            .contains(&self.search_text.to_lowercase())
    }

    /// Update search matches (placeholder implementation)
    fn update_search_matches(&mut self, _state: &AppState) {
        // This would search through all diff content and populate search_matches
        self.search_matches.clear();
        self.current_match = 0;
    }

    /// Build hierarchical file tree structure
    fn build_file_tree(&self, diffs: &[GitDiff]) -> FileTree {
        let mut tree = FileTree::new();

        for (idx, diff) in diffs.iter().enumerate() {
            let file_path = self.get_file_display_name(diff);
            let path_parts: Vec<&str> = file_path.split('/').collect();

            tree.insert_file(path_parts, idx, diff);
        }

        tree
    }

    /// Show enhanced file tree with hierarchical structure
    fn show_enhanced_file_tree(&mut self, ui: &mut egui::Ui, diffs: &[GitDiff]) {
        ui.label("Files Changed");
        ui.separator();

        let tree = self.build_file_tree(diffs);

        egui::ScrollArea::vertical().show(ui, |ui| {
            self.show_file_tree_node(ui, &tree.root, "", diffs);
        });
    }

    /// Recursively show file tree nodes
    fn show_file_tree_node(
        &mut self,
        ui: &mut egui::Ui,
        node: &FileTreeNode,
        path: &str,
        diffs: &[GitDiff],
    ) {
        // Show subdirectories first
        let mut dirs: Vec<_> = node.children.iter().collect();
        dirs.sort_by(|a, b| a.0.cmp(b.0));

        for (name, child_node) in dirs {
            let full_path = if path.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", path, name)
            };

            if child_node.is_directory() {
                // Show directory with expand/collapse
                let is_expanded = *self.file_tree_expanded.get(&full_path).unwrap_or(&true);
                let expand_symbol = if is_expanded { "📂" } else { "📁" };

                ui.horizontal(|ui| {
                    if ui.button(expand_symbol).clicked() {
                        self.file_tree_expanded
                            .insert(full_path.clone(), !is_expanded);
                    }
                    ui.label(name);
                });

                if is_expanded {
                    ui.indent(format!("dir_{}", full_path), |ui| {
                        self.show_file_tree_node(ui, child_node, &full_path, diffs);
                    });
                }
            } else if let Some(file_idx) = child_node.file_index {
                // Show file with icon and stats
                let diff = &diffs[file_idx];
                let is_selected = file_idx == self.current_file_index;

                ui.horizontal(|ui| {
                    // File type icon
                    let icon = self.get_file_icon(name);
                    ui.label(icon);

                    // File name (clickable)
                    let response = ui.selectable_label(is_selected, name);
                    if response.clicked() {
                        self.current_file_index = file_idx;
                    }

                    // Show change stats inline
                    if diff.stats.insertions > 0 {
                        ui.colored_label(
                            egui::Color32::GREEN,
                            format!("+{}", diff.stats.insertions),
                        );
                    }
                    if diff.stats.deletions > 0 {
                        ui.colored_label(egui::Color32::RED, format!("-{}", diff.stats.deletions));
                    }
                });
            }
        }
    }

    /// Get appropriate icon for file type
    fn get_file_icon(&self, filename: &str) -> &'static str {
        if let Some(ext) = filename.split('.').last() {
            match ext.to_lowercase().as_str() {
                "rs" => "🦀",
                "py" => "🐍",
                "js" | "ts" => "📜",
                "html" | "htm" => "🌐",
                "css" => "🎨",
                "json" => "📋",
                "md" => "📝",
                "toml" | "yaml" | "yml" => "⚙️",
                "png" | "jpg" | "jpeg" | "gif" | "svg" => "🖼️",
                "txt" => "📄",
                _ => "📄",
            }
        } else {
            "📄"
        }
    }

    /// Show enhanced file navigation bar
    fn show_file_navigation_bar(&mut self, ui: &mut egui::Ui, diffs: &[GitDiff]) {
        ui.horizontal(|ui| {
            ui.label("📁 File:");

            // Previous/Next buttons
            if ui.button("⏪").on_hover_text("Previous file").clicked()
                && self.current_file_index > 0
            {
                self.current_file_index -= 1;
            }

            // File selector dropdown
            egui::ComboBox::from_id_salt("file_selector")
                .selected_text(format!("{}/{}", self.current_file_index + 1, diffs.len()))
                .show_ui(ui, |ui| {
                    for (idx, diff) in diffs.iter().enumerate() {
                        let file_name = self.get_file_display_name(diff);
                        let icon = self.get_file_icon(&file_name);

                        let is_selected = idx == self.current_file_index;
                        if ui
                            .selectable_label(is_selected, format!("{} {}", icon, file_name))
                            .clicked()
                        {
                            self.current_file_index = idx;
                        }
                    }
                });

            if ui.button("⏩").on_hover_text("Next file").clicked()
                && self.current_file_index < diffs.len() - 1
            {
                self.current_file_index += 1;
            }

            ui.separator();

            // Current file info
            if let Some(diff) = diffs.get(self.current_file_index) {
                let file_name = self.get_file_display_name(diff);
                let icon = self.get_file_icon(&file_name);
                ui.label(format!("{} {}", icon, file_name));

                // File stats
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if diff.stats.deletions > 0 {
                        ui.colored_label(egui::Color32::RED, format!("-{}", diff.stats.deletions));
                    }
                    if diff.stats.insertions > 0 {
                        ui.colored_label(
                            egui::Color32::GREEN,
                            format!("+{}", diff.stats.insertions),
                        );
                    }

                    // File status indicator
                    match diff.status {
                        crate::models::DiffStatus::Added => {
                            ui.colored_label(egui::Color32::GREEN, "NEW");
                        }
                        crate::models::DiffStatus::Deleted => {
                            ui.colored_label(egui::Color32::RED, "DEL");
                        }
                        crate::models::DiffStatus::Renamed => {
                            ui.colored_label(egui::Color32::BLUE, "REN");
                        }
                        crate::models::DiffStatus::Modified => {
                            ui.colored_label(egui::Color32::YELLOW, "MOD");
                        }
                        _ => {}
                    }
                });
            }
        });
    }
}

impl SyntaxHighlighter {
    fn new() -> Self {
        let mut highlighter = Self {
            language_patterns: HashMap::new(),
            cache: HashMap::new(),
        };

        highlighter.initialize_languages();
        highlighter
    }

    fn initialize_languages(&mut self) {
        // Initialize Rust highlighting
        self.add_rust_highlighter();

        // Initialize Python highlighting
        self.add_python_highlighter();

        // Initialize JavaScript/TypeScript highlighting
        self.add_javascript_highlighter();

        // Initialize other languages
        self.add_common_language_highlighters();
    }

    fn add_rust_highlighter(&mut self) {
        let keywords = vec![
            "fn", "let", "mut", "const", "static", "if", "else", "match", "for", "while", "loop",
            "break", "continue", "return", "pub", "mod", "use", "crate", "super", "self", "Self",
            "struct", "enum", "trait", "impl", "type", "where", "async", "await", "move", "ref",
            "unsafe", "extern", "as", "in", "dyn", "box", "true", "false",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let operators = vec![
            "=", "==", "!=", "<", ">", "<=", ">=", "+", "-", "*", "/", "%", "&&", "||", "!", "&",
            "|", "^", "<<", ">>", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=",
            "->", "=>", "::", ".", "?", ":", ";", ",", "(", ")", "[", "]", "{", "}",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let patterns = vec![
            // String literals
            SyntaxPattern::new(r#""([^"\\]|\\.)*""#, TokenType::String, 3),
            SyntaxPattern::new(r"'([^'\\]|\\.)*'", TokenType::String, 3),
            SyntaxPattern::new(r"r#.*?#", TokenType::String, 3),
            // Comments
            SyntaxPattern::new(r"//.*$", TokenType::Comment, 2),
            SyntaxPattern::new(r"/\*[\s\S]*?\*/", TokenType::Comment, 2),
            // Numbers
            SyntaxPattern::new(r"\b\d+\.?\d*([eE][+-]?\d+)?\b", TokenType::Number, 1),
            SyntaxPattern::new(r"\b0x[0-9a-fA-F]+\b", TokenType::Number, 1),
            SyntaxPattern::new(r"\b0b[01]+\b", TokenType::Number, 1),
            SyntaxPattern::new(r"\b0o[0-7]+\b", TokenType::Number, 1),
            // Functions
            SyntaxPattern::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\s*\(", TokenType::Function, 1),
            // Types
            SyntaxPattern::new(r"\b[A-Z][a-zA-Z0-9_]*\b", TokenType::Type, 1),
            // Macros
            SyntaxPattern::new(r"\b[a-zA-Z_][a-zA-Z0-9_]*!", TokenType::Preprocessor, 1),
            // Attributes
            SyntaxPattern::new(r"#\[[^\]]*\]", TokenType::Preprocessor, 2),
        ];

        let highlighter = LanguageHighlighter {
            language: ProgrammingLanguage::Rust,
            keywords,
            operators,
            patterns,
        };

        self.language_patterns
            .insert("rs".to_string(), highlighter.clone());
        self.language_patterns
            .insert("rust".to_string(), highlighter);
    }

    fn add_python_highlighter(&mut self) {
        let keywords = vec![
            "and", "as", "assert", "break", "class", "continue", "def", "del", "elif", "else",
            "except", "exec", "finally", "for", "from", "global", "if", "import", "in", "is",
            "lambda", "not", "or", "pass", "print", "raise", "return", "try", "while", "with",
            "yield", "async", "await", "True", "False", "None",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let operators = vec![
            "=", "==", "!=", "<", ">", "<=", ">=", "+", "-", "*", "/", "//", "%", "**", "+=", "-=",
            "*=", "/=", "//=", "%=", "**=", "&", "|", "^", "~", "<<", ">>", "&=", "|=", "^=",
            "<<=", ">>=", "and", "or", "not", "in", "is",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let patterns = vec![
            // String literals
            SyntaxPattern::new(r#""([^"\\]|\\.)*""#, TokenType::String, 3),
            SyntaxPattern::new(r"'([^'\\]|\\.)*'", TokenType::String, 3),
            SyntaxPattern::new(r#"r"[^"]*""#, TokenType::String, 3),
            SyntaxPattern::new(r#"f"([^"\\]|\\.)*""#, TokenType::String, 3),
            SyntaxPattern::new(r#"b"([^"\\]|\\.)*""#, TokenType::String, 3),
            SyntaxPattern::new(r#"u"([^"\\]|\\.)*""#, TokenType::String, 3),
            // Comments
            SyntaxPattern::new(r"#.*$", TokenType::Comment, 2),
            // Numbers
            SyntaxPattern::new(r"\b\d+\.?\d*([eE][+-]?\d+)?\b", TokenType::Number, 1),
            SyntaxPattern::new(r"\b0x[0-9a-fA-F]+\b", TokenType::Number, 1),
            SyntaxPattern::new(r"\b0b[01]+\b", TokenType::Number, 1),
            SyntaxPattern::new(r"\b0o[0-7]+\b", TokenType::Number, 1),
            // Functions
            SyntaxPattern::new(r"\bdef\s+([a-zA-Z_][a-zA-Z0-9_]*)", TokenType::Function, 2),
            SyntaxPattern::new(r"\bclass\s+([a-zA-Z_][a-zA-Z0-9_]*)", TokenType::Type, 2),
            // Decorators
            SyntaxPattern::new(r"@[a-zA-Z_][a-zA-Z0-9_]*", TokenType::Preprocessor, 1),
        ];

        let highlighter = LanguageHighlighter {
            language: ProgrammingLanguage::Python,
            keywords,
            operators,
            patterns,
        };

        self.language_patterns
            .insert("py".to_string(), highlighter.clone());
        self.language_patterns
            .insert("python".to_string(), highlighter);
    }

    fn add_javascript_highlighter(&mut self) {
        let keywords: Vec<String> = vec![
            "abstract",
            "arguments",
            "await",
            "boolean",
            "break",
            "byte",
            "case",
            "catch",
            "char",
            "class",
            "const",
            "continue",
            "debugger",
            "default",
            "delete",
            "do",
            "double",
            "else",
            "enum",
            "eval",
            "export",
            "extends",
            "false",
            "final",
            "finally",
            "float",
            "for",
            "function",
            "goto",
            "if",
            "implements",
            "import",
            "in",
            "instanceof",
            "int",
            "interface",
            "let",
            "long",
            "native",
            "new",
            "null",
            "package",
            "private",
            "protected",
            "public",
            "return",
            "short",
            "static",
            "super",
            "switch",
            "synchronized",
            "this",
            "throw",
            "throws",
            "transient",
            "true",
            "try",
            "typeof",
            "var",
            "void",
            "volatile",
            "while",
            "with",
            "yield",
            "async",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let patterns = vec![
            // String literals
            SyntaxPattern::new(r#""([^"\\]|\\.)*""#, TokenType::String, 3),
            SyntaxPattern::new(r"'([^'\\]|\\.)*'", TokenType::String, 3),
            SyntaxPattern::new(r"`[^`]*`", TokenType::String, 3),
            // Comments
            SyntaxPattern::new(r"//.*$", TokenType::Comment, 2),
            SyntaxPattern::new(r"/\*[\s\S]*?\*/", TokenType::Comment, 2),
            // Numbers
            SyntaxPattern::new(r"\b\d+\.?\d*([eE][+-]?\d+)?\b", TokenType::Number, 1),
            SyntaxPattern::new(r"\b0x[0-9a-fA-F]+\b", TokenType::Number, 1),
            // Functions
            SyntaxPattern::new(
                r"\bfunction\s+([a-zA-Z_$][a-zA-Z0-9_$]*)",
                TokenType::Function,
                2,
            ),
            SyntaxPattern::new(r"\b([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(", TokenType::Function, 1),
            // RegEx
            SyntaxPattern::new(r"/[^/\n]+/[gimuy]*", TokenType::String, 2),
        ];

        let highlighter = LanguageHighlighter {
            language: ProgrammingLanguage::JavaScript,
            keywords: keywords.clone(),
            operators: vec![],
            patterns: patterns.clone(),
        };

        self.language_patterns
            .insert("js".to_string(), highlighter.clone());
        self.language_patterns
            .insert("javascript".to_string(), highlighter.clone());

        // TypeScript (extends JavaScript)
        let mut ts_keywords = keywords;
        ts_keywords.extend(
            vec![
                "type",
                "interface",
                "namespace",
                "module",
                "declare",
                "public",
                "private",
                "protected",
                "readonly",
                "abstract",
                "static",
                "implements",
                "extends",
                "keyof",
                "infer",
                "is",
            ]
            .into_iter()
            .map(String::from),
        );

        let ts_highlighter = LanguageHighlighter {
            language: ProgrammingLanguage::TypeScript,
            keywords: ts_keywords,
            operators: vec![],
            patterns,
        };

        self.language_patterns
            .insert("ts".to_string(), ts_highlighter.clone());
        self.language_patterns
            .insert("typescript".to_string(), ts_highlighter);
    }

    fn add_common_language_highlighters(&mut self) {
        // JSON
        let json_patterns = vec![
            SyntaxPattern::new(r#""([^"\\]|\\.)*""#, TokenType::String, 3),
            SyntaxPattern::new(r"\b\d+\.?\d*([eE][+-]?\d+)?\b", TokenType::Number, 1),
            SyntaxPattern::new(r"\b(true|false|null)\b", TokenType::Keyword, 2),
        ];

        let json_highlighter = LanguageHighlighter {
            language: ProgrammingLanguage::Json,
            keywords: vec!["true".to_string(), "false".to_string(), "null".to_string()],
            operators: vec![],
            patterns: json_patterns,
        };

        self.language_patterns
            .insert("json".to_string(), json_highlighter);

        // Markdown
        let md_patterns = vec![
            SyntaxPattern::new(r"^#+\s.*$", TokenType::Keyword, 3),
            SyntaxPattern::new(r"\*\*[^*]+\*\*", TokenType::Type, 2),
            SyntaxPattern::new(r"\*[^*]+\*", TokenType::String, 2),
            SyntaxPattern::new(r"`[^`]+`", TokenType::Constant, 2),
            SyntaxPattern::new(r"```[\s\S]*?```", TokenType::Comment, 3),
            SyntaxPattern::new(r"\[([^\]]+)\]\(([^)]+)\)", TokenType::Function, 2),
        ];

        let md_highlighter = LanguageHighlighter {
            language: ProgrammingLanguage::Markdown,
            keywords: vec![],
            operators: vec![],
            patterns: md_patterns,
        };

        self.language_patterns
            .insert("md".to_string(), md_highlighter.clone());
        self.language_patterns
            .insert("markdown".to_string(), md_highlighter);
    }

    fn detect_language(&self, filename: &str) -> ProgrammingLanguage {
        if let Some(extension) = filename.split('.').last() {
            match extension.to_lowercase().as_str() {
                "rs" => ProgrammingLanguage::Rust,
                "py" => ProgrammingLanguage::Python,
                "js" => ProgrammingLanguage::JavaScript,
                "ts" => ProgrammingLanguage::TypeScript,
                "go" => ProgrammingLanguage::Go,
                "java" => ProgrammingLanguage::Java,
                "c" => ProgrammingLanguage::C,
                "cpp" | "cc" | "cxx" => ProgrammingLanguage::Cpp,
                "cs" => ProgrammingLanguage::CSharp,
                "html" | "htm" => ProgrammingLanguage::Html,
                "css" => ProgrammingLanguage::Css,
                "json" => ProgrammingLanguage::Json,
                "yaml" | "yml" => ProgrammingLanguage::Yaml,
                "md" | "markdown" => ProgrammingLanguage::Markdown,
                "sh" | "bash" | "zsh" => ProgrammingLanguage::Shell,
                "sql" => ProgrammingLanguage::Sql,
                _ => ProgrammingLanguage::Unknown,
            }
        } else {
            ProgrammingLanguage::Unknown
        }
    }

    fn highlight_text(&mut self, text: &str, filename: &str) -> Vec<SyntaxToken> {
        let cache_key = format!("{}:{}", filename, text);
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        let _language = self.detect_language(filename);
        let extension = filename.split('.').last().unwrap_or("").to_lowercase();

        let tokens = if let Some(highlighter) = self.language_patterns.get(&extension) {
            self.tokenize_with_highlighter(text, highlighter)
        } else {
            self.tokenize_basic(text)
        };

        // Cache the result
        self.cache.insert(cache_key, tokens.clone());
        tokens
    }

    fn tokenize_with_highlighter(
        &self,
        text: &str,
        highlighter: &LanguageHighlighter,
    ) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let mut processed_ranges = Vec::new();

        // Process patterns by priority (higher priority first)
        let mut sorted_patterns = highlighter.patterns.clone();
        sorted_patterns.sort_by(|a, b| b.priority.cmp(&a.priority));

        for pattern in sorted_patterns {
            for cap in pattern.regex.find_iter(text) {
                let start = cap.start();
                let end = cap.end();

                // Check if this range overlaps with already processed ranges
                if !processed_ranges.iter().any(|(s, e)| start < *e && end > *s) {
                    tokens.push(SyntaxToken {
                        start,
                        end,
                        token_type: pattern.token_type.clone(),
                        text: cap.as_str().to_string(),
                    });
                    processed_ranges.push((start, end));
                }
            }
        }

        // Add keywords
        for keyword in &highlighter.keywords {
            let keyword_regex = Regex::new(&format!(r"\b{}\b", regex::escape(keyword))).unwrap();
            for cap in keyword_regex.find_iter(text) {
                let start = cap.start();
                let end = cap.end();

                if !processed_ranges.iter().any(|(s, e)| start < *e && end > *s) {
                    tokens.push(SyntaxToken {
                        start,
                        end,
                        token_type: TokenType::Keyword,
                        text: cap.as_str().to_string(),
                    });
                    processed_ranges.push((start, end));
                }
            }
        }

        // Sort tokens by position
        tokens.sort_by(|a, b| a.start.cmp(&b.start));
        tokens
    }

    fn tokenize_basic(&self, text: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();

        // Basic tokenization for unknown file types
        let string_regex = Regex::new(r#""([^"\\]|\\.)*"|'([^'\\]|\\.)*'"#).unwrap();
        for cap in string_regex.find_iter(text) {
            tokens.push(SyntaxToken {
                start: cap.start(),
                end: cap.end(),
                token_type: TokenType::String,
                text: cap.as_str().to_string(),
            });
        }

        let number_regex = Regex::new(r"\b\d+\.?\d*\b").unwrap();
        for cap in number_regex.find_iter(text) {
            tokens.push(SyntaxToken {
                start: cap.start(),
                end: cap.end(),
                token_type: TokenType::Number,
                text: cap.as_str().to_string(),
            });
        }

        tokens.sort_by(|a, b| a.start.cmp(&b.start));
        tokens
    }

    fn get_token_color(&self, token_type: &TokenType) -> egui::Color32 {
        match token_type {
            TokenType::Keyword => egui::Color32::from_rgb(86, 156, 214), // Blue
            TokenType::String => egui::Color32::from_rgb(206, 145, 120), // Orange
            TokenType::Comment => egui::Color32::from_rgb(106, 153, 85), // Green
            TokenType::Number => egui::Color32::from_rgb(181, 206, 168), // Light Green
            TokenType::Operator => egui::Color32::from_rgb(212, 212, 212), // Light Gray
            TokenType::Function => egui::Color32::from_rgb(220, 220, 170), // Yellow
            TokenType::Type => egui::Color32::from_rgb(78, 201, 176),    // Teal
            TokenType::Variable => egui::Color32::from_rgb(156, 220, 254), // Light Blue
            TokenType::Constant => egui::Color32::from_rgb(100, 102, 149), // Purple
            TokenType::Preprocessor => egui::Color32::from_rgb(155, 155, 155), // Gray
            TokenType::Error => egui::Color32::from_rgb(244, 71, 71),    // Red
            TokenType::Normal => egui::Color32::from_rgb(212, 212, 212), // Default
        }
    }
}

impl SyntaxPattern {
    fn new(pattern: &str, token_type: TokenType, priority: u8) -> Self {
        Self {
            regex: Regex::new(pattern).unwrap(),
            token_type,
            priority,
        }
    }
}

impl WordDiffEngine {
    fn new() -> Self {
        Self {
            word_boundary_regex: Regex::new(r"\b").unwrap(),
            cache: HashMap::new(),
        }
    }

    fn compute_word_diff(&mut self, old_line: &str, new_line: &str) -> WordDiffResult {
        let cache_key = format!("{}|{}", old_line, new_line);
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        let old_words = self.split_into_words(old_line);
        let new_words = self.split_into_words(new_line);

        let operations = self.compute_diff_operations(&old_words, &new_words);

        let result = WordDiffResult {
            old_words: old_words
                .into_iter()
                .map(|(text, start, end)| DiffWord {
                    text,
                    start_pos: start,
                    end_pos: end,
                    change_type: WordChangeType::Removed, // Will be updated based on operations
                })
                .collect(),
            new_words: new_words
                .into_iter()
                .map(|(text, start, end)| DiffWord {
                    text,
                    start_pos: start,
                    end_pos: end,
                    change_type: WordChangeType::Added, // Will be updated based on operations
                })
                .collect(),
            operations,
        };

        self.cache.insert(cache_key, result.clone());
        result
    }

    fn split_into_words(&self, text: &str) -> Vec<(String, usize, usize)> {
        let mut words = Vec::new();
        let _current_pos = 0;

        // Split on word boundaries, whitespace, and punctuation
        let word_regex = Regex::new(r"\w+|\s+|[^\w\s]").unwrap();

        for mat in word_regex.find_iter(text) {
            words.push((mat.as_str().to_string(), mat.start(), mat.end()));
        }

        words
    }

    fn compute_diff_operations(
        &self,
        old_words: &[(String, usize, usize)],
        new_words: &[(String, usize, usize)],
    ) -> Vec<DiffOperation> {
        let mut operations = Vec::new();

        // Simple LCS-based diff algorithm
        let old_texts: Vec<&str> = old_words.iter().map(|(text, _, _)| text.as_str()).collect();
        let new_texts: Vec<&str> = new_words.iter().map(|(text, _, _)| text.as_str()).collect();

        let lcs = self.longest_common_subsequence(&old_texts, &new_texts);

        let mut old_idx = 0;
        let mut new_idx = 0;
        let mut lcs_idx = 0;

        while old_idx < old_texts.len() || new_idx < new_texts.len() {
            if lcs_idx < lcs.len()
                && old_idx < old_texts.len()
                && new_idx < new_texts.len()
                && old_texts[old_idx] == lcs[lcs_idx]
                && new_texts[new_idx] == lcs[lcs_idx]
            {
                // Common text
                operations.push(DiffOperation::Equal(old_texts[old_idx].to_string()));
                old_idx += 1;
                new_idx += 1;
                lcs_idx += 1;
            } else if old_idx < old_texts.len()
                && (lcs_idx >= lcs.len() || old_texts[old_idx] != lcs[lcs_idx])
            {
                // Deletion
                if new_idx < new_texts.len()
                    && (lcs_idx >= lcs.len() || new_texts[new_idx] != lcs[lcs_idx])
                {
                    // Replacement
                    operations.push(DiffOperation::Replace(
                        old_texts[old_idx].to_string(),
                        new_texts[new_idx].to_string(),
                    ));
                    old_idx += 1;
                    new_idx += 1;
                } else {
                    operations.push(DiffOperation::Delete(old_texts[old_idx].to_string()));
                    old_idx += 1;
                }
            } else if new_idx < new_texts.len() {
                // Insertion
                operations.push(DiffOperation::Insert(new_texts[new_idx].to_string()));
                new_idx += 1;
            }
        }

        operations
    }

    fn longest_common_subsequence(&self, old: &[&str], new: &[&str]) -> Vec<String> {
        let m = old.len();
        let n = new.len();
        let mut dp = vec![vec![0; n + 1]; m + 1];

        // Build LCS table
        for i in 1..=m {
            for j in 1..=n {
                if old[i - 1] == new[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        // Backtrack to find LCS
        let mut lcs = Vec::new();
        let mut i = m;
        let mut j = n;

        while i > 0 && j > 0 {
            if old[i - 1] == new[j - 1] {
                lcs.push(old[i - 1].to_string());
                i -= 1;
                j -= 1;
            } else if dp[i - 1][j] > dp[i][j - 1] {
                i -= 1;
            } else {
                j -= 1;
            }
        }

        lcs.reverse();
        lcs
    }
}
