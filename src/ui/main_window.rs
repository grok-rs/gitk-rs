use crate::state::{AppConfig, AppState};
use crate::ui::{CommitGraph, DiffViewer, ReferencesPanel, SearchPanel, ViewsPanel};
use eframe::egui;

pub struct MainWindow {
    commit_graph: CommitGraph,
    diff_viewer: DiffViewer,
    search_panel: SearchPanel,
    references_panel: ReferencesPanel,
    views_panel: ViewsPanel,
    left_panel_width: f32,
    right_panel_width: f32,
    show_references: bool,
    show_views: bool,
    // Layout management
    layout_mode: LayoutMode,
    split_ratios: SplitRatios,
    panel_visibility: PanelVisibility,
    // Menu and toolbar state
    show_menubar: bool,
    show_toolbar: bool,
    show_statusbar: bool,
    // Context menus
    show_file_context_menu: bool,
    context_menu_pos: egui::Pos2,
    context_file_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutMode {
    ThreePaneClassic,  // Commit list | Diff | File tree
    TwoPaneHorizontal, // Commit list | Diff
    TwoPaneVertical,   // Commit list / Diff (stacked vertically)
    SinglePane,        // Full width diff view
}

#[derive(Debug, Clone)]
pub struct SplitRatios {
    pub left_panel_ratio: f32,     // 0.0 to 1.0
    pub right_panel_ratio: f32,    // 0.0 to 1.0
    pub vertical_split_ratio: f32, // For vertical layouts
}

#[derive(Debug, Clone)]
pub struct PanelVisibility {
    pub commit_graph: bool,
    pub diff_viewer: bool,
    pub file_tree: bool,
    pub references: bool,
    pub views: bool,
    pub search: bool,
    pub auto_hide_empty: bool,
}

impl MainWindow {
    pub fn new() -> Self {
        Self {
            commit_graph: CommitGraph::new(),
            diff_viewer: DiffViewer::new(),
            search_panel: SearchPanel::new(),
            references_panel: ReferencesPanel::new(),
            views_panel: ViewsPanel::new(),
            left_panel_width: 500.0,
            right_panel_width: 350.0,
            show_references: true,
            show_views: true,
            // Layout management
            layout_mode: LayoutMode::ThreePaneClassic,
            split_ratios: SplitRatios {
                left_panel_ratio: 0.35,
                right_panel_ratio: 0.25,
                vertical_split_ratio: 0.6,
            },
            panel_visibility: PanelVisibility {
                commit_graph: true,
                diff_viewer: true,
                file_tree: true,
                references: true,
                views: true,
                search: true,
                auto_hide_empty: false,
            },
            // Menu and toolbar state
            show_menubar: true,
            show_toolbar: true,
            show_statusbar: true,
            // Context menus
            show_file_context_menu: false,
            context_menu_pos: egui::Pos2::ZERO,
            context_file_path: String::new(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, state: &mut AppState, config: &AppConfig) {
        // Handle keyboard shortcuts first
        self.handle_keyboard_shortcuts(ui, state);

        // Responsive layout adjustment based on window size
        self.adjust_layout_for_screen_size(ui);

        // Menu bar
        if self.show_menubar {
            self.show_menu_bar(ui, state, config);
        }

        // Toolbar
        if self.show_toolbar {
            self.show_toolbar(ui, state, config);
        }

        // Main content area with layout-specific rendering
        match self.layout_mode {
            LayoutMode::ThreePaneClassic => {
                self.show_three_pane_layout(ui, state, config);
            }
            LayoutMode::TwoPaneHorizontal => {
                self.show_two_pane_horizontal_layout(ui, state, config);
            }
            LayoutMode::TwoPaneVertical => {
                self.show_two_pane_vertical_layout(ui, state, config);
            }
            LayoutMode::SinglePane => {
                self.show_single_pane_layout(ui, state, config);
            }
        }

        // Status bar
        if self.show_statusbar {
            self.show_status_bar(ui, state);
        }

        // Context menus
        self.handle_context_menus(ui, state);
    }

    pub fn show_dialogs(&mut self, ctx: &egui::Context, state: &mut AppState) {
        // Show views dialogs (create/edit view dialogs)
        self.views_panel.show_dialogs(ctx, state);
    }

    /// Handle keyboard shortcuts for the main window
    fn handle_keyboard_shortcuts(&mut self, ui: &mut egui::Ui, state: &mut AppState) {
        let ctx = ui.ctx();

        // File operations
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
            // Open repository dialog (would be implemented)
        }

        // Layout shortcuts
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Num1)) {
            self.layout_mode = LayoutMode::SinglePane;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Num2)) {
            self.layout_mode = LayoutMode::TwoPaneHorizontal;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Num3)) {
            self.layout_mode = LayoutMode::ThreePaneClassic;
        }

        // Panel visibility shortcuts
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::T)) {
            self.panel_visibility.file_tree = !self.panel_visibility.file_tree;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::R)) {
            self.panel_visibility.references = !self.panel_visibility.references;
        }

        // Search shortcut
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            self.panel_visibility.search = !self.panel_visibility.search;
        }

        // Refresh
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F5)) {
            state.refresh_commits();
        }
    }

    /// Show comprehensive menu bar
    fn show_menu_bar(&mut self, ui: &mut egui::Ui, state: &mut AppState, _config: &AppConfig) {
        egui::menu::bar(ui, |ui| {
            // File menu
            ui.menu_button("File", |ui| {
                if ui.button("📁 Open Repository...").clicked() {
                    // Would open file dialog
                    ui.close_menu();
                }
                if ui.button("📄 Open Recent").clicked() {
                    // Would show recent repositories
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("🔄 Refresh").clicked() {
                    state.refresh_commits();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("❌ Exit").clicked() {
                    std::process::exit(0);
                }
            });

            // Edit menu
            ui.menu_button("Edit", |ui| {
                if ui.button("📋 Copy Commit ID").clicked() {
                    if let Some(commit) = state.get_selected_commit() {
                        ui.output_mut(|o| o.copied_text = commit.id.clone());
                    }
                    ui.close_menu();
                }
                if ui.button("🔍 Find...").clicked() {
                    self.panel_visibility.search = true;
                    ui.close_menu();
                }
            });

            // View menu
            ui.menu_button("View", |ui| {
                ui.heading("Layout");
                ui.separator();

                ui.radio_value(&mut self.layout_mode, LayoutMode::SinglePane, "Single Pane");
                ui.radio_value(
                    &mut self.layout_mode,
                    LayoutMode::TwoPaneHorizontal,
                    "Two Pane Horizontal",
                );
                ui.radio_value(
                    &mut self.layout_mode,
                    LayoutMode::TwoPaneVertical,
                    "Two Pane Vertical",
                );
                ui.radio_value(
                    &mut self.layout_mode,
                    LayoutMode::ThreePaneClassic,
                    "Three Pane Classic",
                );

                ui.separator();
                ui.heading("Panels");
                ui.separator();

                ui.checkbox(&mut self.panel_visibility.commit_graph, "📊 Commit Graph");
                ui.checkbox(&mut self.panel_visibility.diff_viewer, "📝 Diff Viewer");
                ui.checkbox(&mut self.panel_visibility.file_tree, "🌳 File Tree");
                ui.checkbox(&mut self.panel_visibility.references, "🏷️ References");
                ui.checkbox(&mut self.panel_visibility.views, "👁️ Views");
                ui.checkbox(&mut self.panel_visibility.search, "🔍 Search");

                ui.separator();
                ui.heading("Interface");
                ui.separator();

                ui.checkbox(&mut self.show_menubar, "Menu Bar");
                ui.checkbox(&mut self.show_toolbar, "Toolbar");
                ui.checkbox(&mut self.show_statusbar, "Status Bar");
            });

            // Git menu
            ui.menu_button("Git", |ui| {
                if ui.button("🌿 Branches").clicked() {
                    self.panel_visibility.references = true;
                    ui.close_menu();
                }
                if ui.button("🏷️ Tags").clicked() {
                    self.panel_visibility.references = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("📊 Show Graph").clicked() {
                    self.panel_visibility.commit_graph = true;
                    ui.close_menu();
                }
            });

            // Help menu
            ui.menu_button("Help", |ui| {
                if ui.button("ℹ️ About").clicked() {
                    // Would show about dialog
                    ui.close_menu();
                }
                if ui.button("📚 User Guide").clicked() {
                    // Would open help
                    ui.close_menu();
                }
                if ui.button("⌨️ Keyboard Shortcuts").clicked() {
                    // Would show shortcuts dialog
                    ui.close_menu();
                }
            });
        });
    }

    /// Show toolbar with common actions
    fn show_toolbar(&mut self, ui: &mut egui::Ui, state: &mut AppState, config: &AppConfig) {
        ui.horizontal(|ui| {
            // File operations
            if ui.button("📁").on_hover_text("Open Repository").clicked() {
                // Would open file dialog
            }

            if ui.button("🔄").on_hover_text("Refresh").clicked() {
                state.refresh_commits();
            }

            ui.separator();

            // Layout buttons
            if ui
                .button("1")
                .on_hover_text("Single Pane (Ctrl+1)")
                .clicked()
            {
                self.layout_mode = LayoutMode::SinglePane;
            }
            if ui.button("2").on_hover_text("Two Pane (Ctrl+2)").clicked() {
                self.layout_mode = LayoutMode::TwoPaneHorizontal;
            }
            if ui
                .button("3")
                .on_hover_text("Three Pane (Ctrl+3)")
                .clicked()
            {
                self.layout_mode = LayoutMode::ThreePaneClassic;
            }

            ui.separator();

            // Panel toggles
            if ui
                .button("📊")
                .on_hover_text("Toggle Commit Graph")
                .clicked()
            {
                self.panel_visibility.commit_graph = !self.panel_visibility.commit_graph;
            }
            if ui
                .button("🌳")
                .on_hover_text("Toggle File Tree (Ctrl+T)")
                .clicked()
            {
                self.panel_visibility.file_tree = !self.panel_visibility.file_tree;
            }
            if ui
                .button("🏷️")
                .on_hover_text("Toggle References (Ctrl+R)")
                .clicked()
            {
                self.panel_visibility.references = !self.panel_visibility.references;
            }

            ui.separator();

            // Search
            if self.panel_visibility.search {
                ui.label("🔍");
                self.search_panel.show(ui, state);
            } else {
                if ui
                    .button("🔍")
                    .on_hover_text("Show Search (Ctrl+F)")
                    .clicked()
                {
                    self.panel_visibility.search = true;
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Repository info
                if let Some(repo_info) = state.repository_info() {
                    ui.label(format!("📂 {}", repo_info.name));
                    if let Some(ref branch) = state.get_current_branch() {
                        ui.label(format!("🌿 {}", branch));
                    }
                }
            });
        });

        ui.separator();
    }

    /// Show three-pane classic layout (commit list | diff | file tree)
    fn show_three_pane_layout(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut AppState,
        config: &AppConfig,
    ) {
        // Optional side panels first
        if self.panel_visibility.references {
            egui::SidePanel::left("references")
                .resizable(true)
                .default_width(250.0)
                .width_range(200.0..=400.0)
                .show_inside(ui, |ui| {
                    self.references_panel.show(ui, state);
                });
        }

        if self.panel_visibility.views {
            egui::SidePanel::left("views")
                .resizable(true)
                .default_width(250.0)
                .width_range(200.0..=400.0)
                .show_inside(ui, |ui| {
                    self.views_panel.show(ui, state);
                });
        }

        // Main three-pane layout
        if self.panel_visibility.commit_graph {
            egui::SidePanel::left("commit_list")
                .resizable(true)
                .default_width(self.left_panel_width)
                .width_range(200.0..=800.0)
                .show_inside(ui, |ui| {
                    self.commit_graph.show(ui, state, config);
                });
        }

        if self.panel_visibility.file_tree {
            egui::SidePanel::right("file_tree")
                .resizable(true)
                .default_width(self.right_panel_width)
                .width_range(200.0..=600.0)
                .show_inside(ui, |ui| {
                    self.show_file_tree(ui, state);
                });
        }

        if self.panel_visibility.diff_viewer {
            egui::CentralPanel::default().show_inside(ui, |ui| {
                self.diff_viewer.show(ui, state, config);
            });
        }
    }

    /// Show two-pane horizontal layout (commit list | diff)
    fn show_two_pane_horizontal_layout(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut AppState,
        config: &AppConfig,
    ) {
        // Optional side panels
        if self.panel_visibility.references {
            egui::SidePanel::left("references")
                .resizable(true)
                .default_width(250.0)
                .width_range(200.0..=400.0)
                .show_inside(ui, |ui| {
                    self.references_panel.show(ui, state);
                });
        }

        if self.panel_visibility.views {
            egui::SidePanel::left("views")
                .resizable(true)
                .default_width(250.0)
                .width_range(200.0..=400.0)
                .show_inside(ui, |ui| {
                    self.views_panel.show(ui, state);
                });
        }

        // Two-pane main layout
        if self.panel_visibility.commit_graph {
            egui::SidePanel::left("commit_list")
                .resizable(true)
                .default_width(self.left_panel_width)
                .width_range(200.0..=800.0)
                .show_inside(ui, |ui| {
                    self.commit_graph.show(ui, state, config);
                });
        }

        if self.panel_visibility.diff_viewer {
            egui::CentralPanel::default().show_inside(ui, |ui| {
                // Combined diff and file tree view
                egui::TopBottomPanel::bottom("file_tree_horizontal")
                    .resizable(true)
                    .default_height(150.0)
                    .height_range(100.0..=300.0)
                    .show_inside(ui, |ui| {
                        if self.panel_visibility.file_tree {
                            self.show_file_tree(ui, state);
                        }
                    });

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    self.diff_viewer.show(ui, state, config);
                });
            });
        }
    }

    /// Show two-pane vertical layout (commit list / diff stacked)
    fn show_two_pane_vertical_layout(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut AppState,
        config: &AppConfig,
    ) {
        // Optional side panels
        if self.panel_visibility.references {
            egui::SidePanel::left("references")
                .resizable(true)
                .default_width(250.0)
                .width_range(200.0..=400.0)
                .show_inside(ui, |ui| {
                    self.references_panel.show(ui, state);
                });
        }

        if self.panel_visibility.views {
            egui::SidePanel::left("views")
                .resizable(true)
                .default_width(250.0)
                .width_range(200.0..=400.0)
                .show_inside(ui, |ui| {
                    self.views_panel.show(ui, state);
                });
        }

        if self.panel_visibility.file_tree {
            egui::SidePanel::right("file_tree")
                .resizable(true)
                .default_width(self.right_panel_width)
                .width_range(200.0..=600.0)
                .show_inside(ui, |ui| {
                    self.show_file_tree(ui, state);
                });
        }

        // Vertical split main content
        if self.panel_visibility.commit_graph {
            egui::TopBottomPanel::top("commit_list_vertical")
                .resizable(true)
                .default_height(300.0)
                .height_range(200.0..=600.0)
                .show_inside(ui, |ui| {
                    self.commit_graph.show(ui, state, config);
                });
        }

        if self.panel_visibility.diff_viewer {
            egui::CentralPanel::default().show_inside(ui, |ui| {
                self.diff_viewer.show(ui, state, config);
            });
        }
    }

    /// Show single-pane layout (full-width diff view)
    fn show_single_pane_layout(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut AppState,
        config: &AppConfig,
    ) {
        // Optional side panels only
        if self.panel_visibility.references {
            egui::SidePanel::left("references")
                .resizable(true)
                .default_width(250.0)
                .width_range(200.0..=400.0)
                .show_inside(ui, |ui| {
                    self.references_panel.show(ui, state);
                });
        }

        if self.panel_visibility.views {
            egui::SidePanel::left("views")
                .resizable(true)
                .default_width(250.0)
                .width_range(200.0..=400.0)
                .show_inside(ui, |ui| {
                    self.views_panel.show(ui, state);
                });
        }

        if self.panel_visibility.file_tree {
            egui::SidePanel::right("file_tree")
                .resizable(true)
                .default_width(self.right_panel_width)
                .width_range(200.0..=600.0)
                .show_inside(ui, |ui| {
                    self.show_file_tree(ui, state);
                });
        }

        // Full-width content
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if self.panel_visibility.commit_graph && self.panel_visibility.diff_viewer {
                // Show both in tabbed interface
                egui::TopBottomPanel::top("tab_bar")
                    .exact_height(30.0)
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.selectable_label(true, "📊 Commits").clicked() {
                                // Switch to commits tab
                            }
                            if ui.selectable_label(false, "📝 Diff").clicked() {
                                // Switch to diff tab
                            }
                        });
                    });

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    // Show current tab content
                    self.commit_graph.show(ui, state, config);
                });
            } else if self.panel_visibility.commit_graph {
                self.commit_graph.show(ui, state, config);
            } else if self.panel_visibility.diff_viewer {
                self.diff_viewer.show(ui, state, config);
            }
        });
    }

    /// Show status bar with repository and selection info
    fn show_status_bar(&mut self, ui: &mut egui::Ui, state: &AppState) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(25.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    // Repository status
                    if let Some(repo_info) = state.repository_info() {
                        ui.label(format!("📂 {}", repo_info.name));

                        if let Some(ref branch) = state.get_current_branch() {
                            ui.separator();
                            ui.label(format!("🌿 {}", branch));
                        }

                        if state.is_detached_head() {
                            ui.separator();
                            ui.colored_label(egui::Color32::YELLOW, "⚠️ DETACHED HEAD");
                        }
                    } else {
                        ui.label("📂 No repository loaded");
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Commit count and selection info
                        let commit_count = state.commits.len();
                        if commit_count > 0 {
                            ui.label(format!("📊 {} commits", commit_count));

                            if let Some(selected_commit) = state.get_selected_commit() {
                                ui.separator();
                                ui.label(format!("Selected: {}", selected_commit.short_id));
                            }
                        }

                        // Loading indicator
                        if state.loading {
                            ui.separator();
                            ui.add(egui::Spinner::new());
                            ui.label("Loading...");
                        }

                        // Error indicator
                        if let Some(ref error) = state.error_message {
                            ui.separator();
                            ui.colored_label(egui::Color32::RED, format!("❌ {}", error));
                        }
                    });
                });
            });
    }

    /// Handle context menus
    fn handle_context_menus(&mut self, ui: &mut egui::Ui, _state: &mut AppState) {
        if self.show_file_context_menu {
            // Use a simpler context menu approach
            let popup_id = egui::Id::new("file_context_menu");
            egui::Area::new(popup_id)
                .fixed_pos(self.context_menu_pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(150.0);

                        if ui.button("📋 Copy Path").clicked() {
                            ui.output_mut(|o| o.copied_text = self.context_file_path.clone());
                            self.show_file_context_menu = false;
                        }
                        if ui.button("👁️ View File").clicked() {
                            // Open file view
                            self.show_file_context_menu = false;
                        }
                        if ui.button("📝 View Diff").clicked() {
                            // Show file diff
                            self.show_file_context_menu = false;
                        }

                        // Close on click outside
                        if ui.input(|i| i.pointer.any_click()) {
                            self.show_file_context_menu = false;
                        }
                    });
                });
        }
    }

    fn show_file_tree(&mut self, ui: &mut egui::Ui, state: &mut AppState) {
        ui.heading("Files");
        ui.separator();

        if let Some(selected_commit) = state.get_selected_commit().cloned() {
            ui.label(format!("Commit: {}", &selected_commit.short_id));
            ui.separator();

            if let Some(ref repo) = state.repository {
                match repo.get_commit_tree_entries(&selected_commit.id) {
                    Ok(entries) => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for entry in entries {
                                let icon = if entry.is_tree { "📁" } else { "📄" };
                                let response = ui.selectable_label(
                                    state.selected_files.contains(&entry.path),
                                    format!("{} {}", icon, entry.name),
                                );

                                if response.clicked() {
                                    // Handle file selection
                                    if !entry.is_tree {
                                        state.selected_files = vec![entry.path.clone()];
                                        if let Ok(diff) =
                                            repo.get_file_diff(&selected_commit.id, &entry.path)
                                        {
                                            state.current_diff = Some(diff);
                                        }
                                    }
                                }

                                // Context menu
                                if response.secondary_clicked() {
                                    self.context_file_path = entry.path.clone();
                                    self.context_menu_pos = response.rect.left_bottom();
                                    self.show_file_context_menu = true;
                                }
                            }
                        });
                    }
                    Err(e) => {
                        ui.label(format!("Error loading files: {}", e));
                    }
                }
            }
        } else {
            ui.label("Select a commit to view files");
        }
    }

    // Methods for keyboard shortcuts
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        match mode {
            ViewMode::Graph => self.layout_mode = LayoutMode::ThreePaneClassic,
            ViewMode::List => self.layout_mode = LayoutMode::TwoPaneHorizontal,
            ViewMode::Tree => self.layout_mode = LayoutMode::TwoPaneVertical,
        }
    }

    pub fn toggle_diff_view(&mut self) {
        self.panel_visibility.diff_viewer = !self.panel_visibility.diff_viewer;
    }

    pub fn toggle_file_tree(&mut self) {
        self.panel_visibility.file_tree = !self.panel_visibility.file_tree;
    }

    // Responsive layout adjustments
    fn adjust_layout_for_screen_size(&mut self, ui: &egui::Ui) {
        let available_width = ui.available_width();
        let available_height = ui.available_height();

        // Auto-adjust layout based on screen size
        if available_width < 800.0 {
            // Small screen: force single pane or vertical layout
            if self.layout_mode == LayoutMode::ThreePaneClassic {
                self.layout_mode = LayoutMode::TwoPaneVertical;
            }

            // Auto-hide panels on small screens
            self.panel_visibility.references = false;
            if available_width < 600.0 {
                self.panel_visibility.file_tree = false;
            }
        } else if available_width < 1200.0 {
            // Medium screen: prefer two-pane layout
            if self.layout_mode == LayoutMode::ThreePaneClassic && available_width < 1000.0 {
                self.layout_mode = LayoutMode::TwoPaneHorizontal;
            }
        }

        // Adjust panel ratios for optimal viewing
        self.adjust_panel_ratios(available_width, available_height);

        // Auto-hide empty or unnecessary panels
        if self.panel_visibility.auto_hide_empty {
            self.auto_hide_empty_panels();
        }
    }

    fn adjust_panel_ratios(&mut self, width: f32, height: f32) {
        // Dynamically adjust split ratios based on content and screen size
        match self.layout_mode {
            LayoutMode::ThreePaneClassic => {
                if width > 1600.0 {
                    // Large screen: more space for diff viewer
                    self.split_ratios.left_panel_ratio = 0.35;
                    self.split_ratios.right_panel_ratio = 0.25;
                } else if width < 1200.0 {
                    // Smaller screen: balanced layout
                    self.split_ratios.left_panel_ratio = 0.45;
                    self.split_ratios.right_panel_ratio = 0.35;
                }
            }
            LayoutMode::TwoPaneHorizontal => {
                // Optimal split for two-pane layout
                self.split_ratios.left_panel_ratio = if width > 1400.0 { 0.4 } else { 0.5 };
            }
            LayoutMode::TwoPaneVertical => {
                // Vertical split adjustment based on height
                self.split_ratios.vertical_split_ratio = if height > 800.0 { 0.6 } else { 0.5 };
            }
            LayoutMode::SinglePane => {
                // Single pane doesn't need ratio adjustments
            }
        }
    }

    fn auto_hide_empty_panels(&mut self) {
        // Hide panels that don't have useful content
        // This could be enhanced to check actual content availability

        // Example: Hide references panel if no refs are available
        // This would require state information to make intelligent decisions
    }

    // Window management utilities
    pub fn save_window_state(&self) -> WindowState {
        WindowState {
            layout_mode: self.layout_mode.clone(),
            panel_visibility: self.panel_visibility.clone(),
            split_ratios: self.split_ratios.clone(),
            left_panel_width: self.left_panel_width,
            right_panel_width: self.right_panel_width,
        }
    }

    pub fn restore_window_state(&mut self, state: &WindowState) {
        self.layout_mode = state.layout_mode.clone();
        self.panel_visibility = state.panel_visibility.clone();
        self.split_ratios = state.split_ratios.clone();
        self.left_panel_width = state.left_panel_width;
        self.right_panel_width = state.right_panel_width;
    }

    // Enhanced layout switching with animation support
    pub fn switch_layout_mode(&mut self, new_mode: LayoutMode) {
        if self.layout_mode != new_mode {
            // Save current state for potential restoration
            let _previous_mode = self.layout_mode.clone();

            // Apply new layout mode
            self.layout_mode = new_mode;

            // Adjust visibility based on new layout
            match self.layout_mode {
                LayoutMode::SinglePane => {
                    // In single pane, focus on diff viewer only
                    self.panel_visibility.commit_graph = false;
                    self.panel_visibility.file_tree = false;
                }
                LayoutMode::TwoPaneHorizontal | LayoutMode::TwoPaneVertical => {
                    // Two pane: enable commit graph and diff viewer
                    self.panel_visibility.commit_graph = true;
                    self.panel_visibility.diff_viewer = true;
                    self.panel_visibility.file_tree = false;
                }
                LayoutMode::ThreePaneClassic => {
                    // Three pane: enable all main panels
                    self.panel_visibility.commit_graph = true;
                    self.panel_visibility.diff_viewer = true;
                    self.panel_visibility.file_tree = true;
                }
            }
        }
    }
}

// Window state persistence
#[derive(Debug, Clone)]
pub struct WindowState {
    pub layout_mode: LayoutMode,
    pub panel_visibility: PanelVisibility,
    pub split_ratios: SplitRatios,
    pub left_panel_width: f32,
    pub right_panel_width: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Graph,
    List,
    Tree,
}
