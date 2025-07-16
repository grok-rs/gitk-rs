use crate::git::GitRepository;
use crate::state::{AppConfig, AppState};
use crate::ui::MainWindow;
use eframe::egui;
use std::path::PathBuf;

pub struct GitkApp {
    state: AppState,
    config: AppConfig,
    main_window: MainWindow,
}

impl GitkApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = AppConfig::load();
        let state = AppState::new();
        let main_window = MainWindow::new();

        Self {
            state,
            config,
            main_window,
        }
    }

    fn open_repository(&mut self, path: PathBuf) {
        match GitRepository::discover(&path) {
            Ok(repo) => {
                self.config.add_recent_repository(path);
                self.state.set_repository(repo);
                let _ = self.config.save();
            }
            Err(e) => {
                self.state.error_message = Some(format!("Failed to open repository: {}", e));
            }
        }
    }

    fn show_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open Repository...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.open_repository(path);
                        }
                        ui.close();
                    }

                    if ui.button("Recent Repositories").clicked() {
                        // Will be implemented in main_window
                    }

                    ui.separator();

                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.state.show_all_branches, "Show All Branches");
                    ui.separator();
                    ui.checkbox(&mut self.config.show_line_numbers, "Show Line Numbers");
                    ui.checkbox(&mut self.config.word_wrap, "Word Wrap");
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("Search Commits").clicked() {
                        // Focus search box
                    }
                    if ui.button("Refresh").clicked() {
                        self.state.start_streaming_commits();
                    }
                    ui.separator();
                    if ui.button("Settings").clicked() {
                        self.state.show_settings_dialog = true;
                        ui.close();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("Keyboard Shortcuts").clicked() {
                        self.state.show_shortcuts_dialog = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("About").clicked() {
                        self.state.show_about_dialog = true;
                        ui.close();
                    }
                });
            });
        });
    }

    fn show_status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(repo_info) = self.state.repository_info() {
                    ui.label(format!("Repository: {}", repo_info.name));
                    ui.separator();
                    if let Some(branch) = &repo_info.head_branch {
                        ui.label(format!("Branch: {}", branch));
                    }
                    ui.separator();
                    ui.label(format!("Commits: {}", self.state.commits.len()));
                } else {
                    ui.label("No repository opened");
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.state.loading {
                        ui.spinner();
                        ui.label("Loading...");
                    }
                });
            });
        });
    }

    fn show_error_dialog(&mut self, ctx: &egui::Context) {
        if let Some(error) = self.state.error_message.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(&error);
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            self.state.clear_error();
                        }
                    });
                });
        }
    }
}

impl eframe::App for GitkApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(ctx);

        // Update window size in config for persistence
        self.update_window_size(frame);

        // Poll commit stream for new commits
        if self.state.poll_commit_stream() {
            ctx.request_repaint(); // Request repaint when new commits arrive
        }

        // Continue polling if we're still streaming
        if self.state.is_streaming() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16)); // ~60 FPS
        }

        self.show_menu_bar(ctx);
        self.show_status_bar(ctx);
        self.show_error_dialog(ctx);
        self.show_shortcuts_dialog(ctx);
        self.show_about_dialog(ctx);
        self.show_settings_dialog(ctx);

        // Show modal dialogs
        if self.state.has_repository() {
            self.main_window.show_dialogs(ctx, &mut self.state);
        }

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.has_repository() {
                self.main_window.show(ui, &mut self.state, &self.config);
            } else {
                self.show_welcome_screen(ui);
            }
        });
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        let _ = self.config.save();
    }
}

impl GitkApp {
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // Global shortcuts
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
            // Open repository
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                self.open_repository(path);
            }
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Q)) {
            // Quit application
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F5)) {
            // Refresh repository
            if self.state.has_repository() {
                self.state.start_streaming_commits();
            }
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            // Focus search
            self.state.focus_search = true;
        }

        // Navigation shortcuts (only when repository is open)
        if self.state.has_repository() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
                self.state.navigate_commits(-1);
            }

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
                self.state.navigate_commits(1);
            }

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::PageUp)) {
                self.state.navigate_commits(-10);
            }

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::PageDown)) {
                self.state.navigate_commits(10);
            }

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Home)) {
                self.state.navigate_to_first_commit();
            }

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::End)) {
                self.state.navigate_to_last_commit();
            }

            // View mode shortcuts
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Num1)) {
                self.main_window.set_view_mode(crate::ui::ViewMode::Graph);
            }

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Num2)) {
                self.main_window.set_view_mode(crate::ui::ViewMode::List);
            }

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Num3)) {
                self.main_window.set_view_mode(crate::ui::ViewMode::Tree);
            }

            // Diff view shortcuts
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::D)) {
                self.main_window.toggle_diff_view();
            }

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::T)) {
                self.main_window.toggle_file_tree();
            }
        }
    }

    fn update_window_size(&self, _frame: &eframe::Frame) {
        // Window size tracking would be implemented here
        // For now, we'll rely on the save method being called on app close
        // The responsive layout adjustments in MainWindow handle runtime responsiveness
    }

    fn show_shortcuts_dialog(&mut self, ctx: &egui::Context) {
        if self.state.show_shortcuts_dialog {
            egui::Window::new("Keyboard Shortcuts")
                .collapsible(false)
                .resizable(true)
                .default_width(600.0)
                .show(ctx, |ui| {
                    ui.heading("Global Shortcuts");
                    ui.separator();
                    ui.label("Ctrl+O: Open Repository");
                    ui.label("Ctrl+Q: Quit");
                    ui.label("F5: Refresh");
                    ui.label("Ctrl+F: Focus Search");

                    ui.add_space(10.0);
                    ui.heading("Navigation");
                    ui.separator();
                    ui.label("↑/↓: Navigate commits");
                    ui.label("Page Up/Down: Navigate 10 commits");
                    ui.label("Home/End: First/Last commit");

                    ui.add_space(10.0);
                    ui.heading("View Modes");
                    ui.separator();
                    ui.label("Ctrl+1: Graph View");
                    ui.label("Ctrl+2: List View");
                    ui.label("Ctrl+3: Tree View");

                    ui.add_space(10.0);
                    ui.heading("Diff Viewer");
                    ui.separator();
                    ui.label("Ctrl+D: Toggle Diff View");
                    ui.label("Ctrl+T: Toggle File Tree");

                    ui.add_space(20.0);
                    if ui.button("Close").clicked() {
                        self.state.show_shortcuts_dialog = false;
                    }
                });
        }
    }

    fn show_about_dialog(&mut self, ctx: &egui::Context) {
        if self.state.show_about_dialog {
            egui::Window::new("About Gitk-Rust")
                .collapsible(false)
                .resizable(false)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Gitk-Rust");
                        ui.add_space(10.0);
                        ui.label("A modern Git repository browser");
                        ui.label("Built with Rust and egui");
                        ui.add_space(20.0);
                        ui.label("Version: 0.1.0");
                        ui.add_space(20.0);
                        if ui.button("Close").clicked() {
                            self.state.show_about_dialog = false;
                        }
                    });
                });
        }
    }

    fn show_settings_dialog(&mut self, ctx: &egui::Context) {
        if self.state.show_settings_dialog {
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(true)
                .default_width(700.0)
                .default_height(500.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // General Settings
                        ui.heading("General");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Theme:");
                            egui::ComboBox::from_id_salt("theme")
                                .selected_text(format!("{:?}", self.config.theme))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.config.theme,
                                        crate::state::config::Theme::Light,
                                        "Light",
                                    );
                                    ui.selectable_value(
                                        &mut self.config.theme,
                                        crate::state::config::Theme::Dark,
                                        "Dark",
                                    );
                                    ui.selectable_value(
                                        &mut self.config.theme,
                                        crate::state::config::Theme::Auto,
                                        "Auto",
                                    );
                                });
                        });

                        ui.horizontal(|ui| {
                            ui.label("Font Size:");
                            ui.add(egui::Slider::new(&mut self.config.font_size, 8.0..=24.0));
                        });

                        ui.checkbox(
                            &mut self.config.confirm_destructive_actions,
                            "Confirm destructive actions",
                        );
                        ui.checkbox(&mut self.config.show_relative_dates, "Show relative dates");
                        ui.checkbox(&mut self.config.compact_view, "Compact view");

                        ui.add_space(20.0);

                        // Performance Settings
                        ui.heading("Performance");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Max commits to load:");
                            ui.add(egui::Slider::new(
                                &mut self.config.performance_settings.max_commits_to_load,
                                100..=10000,
                            ));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Commit batch size:");
                            ui.add(egui::Slider::new(
                                &mut self.config.performance_settings.commit_batch_size,
                                10..=500,
                            ));
                        });

                        ui.checkbox(
                            &mut self.config.performance_settings.enable_commit_streaming,
                            "Enable commit streaming",
                        );
                        ui.checkbox(
                            &mut self.config.performance_settings.cache_diffs,
                            "Cache diffs",
                        );

                        ui.add_space(20.0);

                        // Diff Settings
                        ui.heading("Diff Viewer");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Context lines:");
                            ui.add(egui::Slider::new(
                                &mut self.config.diff_settings.context_lines,
                                0..=10,
                            ));
                        });

                        ui.checkbox(
                            &mut self.config.diff_settings.ignore_whitespace,
                            "Ignore whitespace",
                        );
                        ui.checkbox(
                            &mut self.config.diff_settings.show_word_diff,
                            "Show word-level diff",
                        );
                        ui.checkbox(
                            &mut self.config.diff_settings.syntax_highlighting,
                            "Syntax highlighting",
                        );

                        ui.add_space(20.0);

                        // Layout Settings
                        ui.heading("Layout");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("Default layout:");
                            egui::ComboBox::from_id_salt("layout")
                                .selected_text(&self.config.layout_settings.default_layout_mode)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.config.layout_settings.default_layout_mode,
                                        "three_pane".to_string(),
                                        "Three Pane",
                                    );
                                    ui.selectable_value(
                                        &mut self.config.layout_settings.default_layout_mode,
                                        "two_pane_h".to_string(),
                                        "Two Pane Horizontal",
                                    );
                                    ui.selectable_value(
                                        &mut self.config.layout_settings.default_layout_mode,
                                        "two_pane_v".to_string(),
                                        "Two Pane Vertical",
                                    );
                                    ui.selectable_value(
                                        &mut self.config.layout_settings.default_layout_mode,
                                        "single".to_string(),
                                        "Single Pane",
                                    );
                                });
                        });

                        ui.checkbox(
                            &mut self.config.layout_settings.remember_panel_states,
                            "Remember panel states",
                        );
                        ui.checkbox(
                            &mut self.config.layout_settings.auto_hide_empty_panels,
                            "Auto-hide empty panels",
                        );

                        ui.add_space(30.0);

                        // Action buttons
                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() {
                                let _ = self.config.save();
                                self.state.show_settings_dialog = false;
                            }

                            if ui.button("Cancel").clicked() {
                                // Reload config to discard changes
                                self.config = crate::state::AppConfig::load();
                                self.state.show_settings_dialog = false;
                            }

                            if ui.button("Reset to Defaults").clicked() {
                                self.config = crate::state::AppConfig::default();
                            }
                        });
                    });
                });
        }
    }

    fn show_welcome_screen(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Welcome to Gitk-Rust");
            ui.add_space(20.0);
            ui.label("A Git repository browser written in Rust");
            ui.add_space(40.0);

            if ui.button("Open Repository").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.open_repository(path);
                }
            }

            ui.add_space(20.0);

            if !self.config.recent_repositories.is_empty() {
                ui.label("Recent Repositories:");
                ui.add_space(10.0);

                for (i, repo_path) in self.config.recent_repositories.clone().iter().enumerate() {
                    if i >= 5 {
                        break;
                    } // Show only first 5

                    let display_name = repo_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown");

                    if ui.button(display_name).clicked() {
                        self.open_repository(repo_path.clone());
                    }
                }
            }
        });
    }
}
