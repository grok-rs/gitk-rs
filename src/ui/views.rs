use crate::git::{ViewFilter, ViewManager, ViewPreset};
use crate::state::AppState;
use eframe::egui;

pub struct ViewsPanel {
    show_create_dialog: bool,
    create_dialog: CreateViewDialog,
    show_edit_dialog: bool,
    edit_dialog: EditViewDialog,
}

impl ViewsPanel {
    pub fn new() -> Self {
        Self {
            show_create_dialog: false,
            create_dialog: CreateViewDialog::new(),
            show_edit_dialog: false,
            edit_dialog: EditViewDialog::new(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, state: &mut AppState) {
        ui.heading("Views");
        ui.separator();

        // Controls
        ui.horizontal(|ui| {
            if ui.button("➕ New View").clicked() {
                self.show_create_dialog = true;
                self.create_dialog.reset();
            }

            if ui.button("🔄 Refresh").clicked() {
                if let Some(ref mut view_manager) = state.view_manager {
                    if let Some(ref repo) = state.repository {
                        let _ = view_manager.update_current_view(repo);
                    }
                }
            }

            if ui.button("⚙️ Presets").clicked() {
                self.show_presets_menu(ui, state);
            }
        });

        ui.separator();

        // Current view info
        if let Some(ref view_manager) = state.view_manager {
            ui.horizontal(|ui| {
                ui.strong("Current:");
                ui.label(view_manager.get_current_view_name());

                if let Some(current_view) = view_manager.get_current_view() {
                    ui.label(format!("({} commits)", current_view.commits.len()));

                    if current_view.is_loading {
                        ui.spinner();
                    }
                }
            });
            ui.separator();
        }

        // View list
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(ref mut view_manager) = state.view_manager {
                let view_names = view_manager.get_view_names();
                let current_view_name = view_manager.get_current_view_name().to_string();
                let mut view_to_switch = None;
                let mut view_to_edit = None;
                let mut view_to_delete = None;

                for view_name in view_names {
                    ui.horizontal(|ui| {
                        let is_current = view_name == current_view_name;

                        if is_current {
                            ui.colored_label(egui::Color32::GREEN, "📋");
                        } else {
                            ui.label("  ");
                        }

                        let response = ui.selectable_label(is_current, &view_name);

                        if response.clicked() && !is_current {
                            view_to_switch = Some(view_name.clone());
                        }

                        response.context_menu(|ui| {
                            if ui.button("Switch to view").clicked() {
                                view_to_switch = Some(view_name.clone());
                                ui.close_menu();
                            }

                            ui.separator();

                            if ui.button("Edit view").clicked() {
                                view_to_edit = Some(view_name.clone());
                                ui.close_menu();
                            }

                            if ui.button("Refresh view").clicked() {
                                if let Some(ref repo) = state.repository {
                                    let _ = view_manager.refresh_view(&view_name, repo);
                                }
                                ui.close_menu();
                            }

                            ui.separator();

                            if view_name != "Default" {
                                if ui.button("Delete view").clicked() {
                                    view_to_delete = Some(view_name.clone());
                                    ui.close_menu();
                                }
                            }
                        });

                        // Show view description
                        if let Some(view) = view_manager.get_view(&view_name) {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.weak(&view.filter.description);
                                },
                            );
                        }
                    });
                }

                // Handle view operations
                if let Some(view_name) = view_to_switch {
                    let _ = view_manager.switch_view(&view_name);
                    if let Some(ref repo) = state.repository {
                        let _ = view_manager.update_current_view(repo);
                    }
                }

                if let Some(view_name) = view_to_edit {
                    if let Some(view) = view_manager.get_view(&view_name) {
                        self.edit_dialog.set_filter(view.filter.clone());
                        self.show_edit_dialog = true;
                    }
                }

                if let Some(view_name) = view_to_delete {
                    let _ = view_manager.remove_view(&view_name);
                }
            }
        });
    }

    fn show_presets_menu(&self, ui: &mut egui::Ui, state: &mut AppState) {
        ui.menu_button("Presets", |ui| {
            let presets = ViewPreset::create_common_presets();

            for preset in presets {
                if ui.button(&preset.name).clicked() {
                    if let Some(ref mut view_manager) = state.view_manager {
                        view_manager.add_view(preset.name.clone(), preset.filter);
                        let _ = view_manager.switch_view(&preset.name);

                        if let Some(ref repo) = state.repository {
                            let _ = view_manager.update_current_view(repo);
                        }
                    }
                    ui.close_menu();
                }
            }
        });
    }

    pub fn show_dialogs(&mut self, ctx: &egui::Context, state: &mut AppState) {
        // Create view dialog
        if self.show_create_dialog {
            if let Some(filter) = self.create_dialog.show(ctx) {
                if let Some(ref mut view_manager) = state.view_manager {
                    view_manager.add_view(filter.name.clone(), filter);
                    self.show_create_dialog = false;
                }
            }

            if !self.create_dialog.is_open() {
                self.show_create_dialog = false;
            }
        }

        // Edit view dialog
        if self.show_edit_dialog {
            if let Some(filter) = self.edit_dialog.show(ctx) {
                if let Some(ref mut view_manager) = state.view_manager {
                    // Remove old view and add updated one
                    let _ = view_manager.remove_view(&filter.name);
                    view_manager.add_view(filter.name.clone(), filter);
                    self.show_edit_dialog = false;
                }
            }

            if !self.edit_dialog.is_open() {
                self.show_edit_dialog = false;
            }
        }
    }
}

pub struct CreateViewDialog {
    filter: ViewFilter,
    is_open: bool,
}

impl CreateViewDialog {
    pub fn new() -> Self {
        Self {
            filter: ViewFilter::default(),
            is_open: false,
        }
    }

    pub fn reset(&mut self) {
        self.filter = ViewFilter::default();
        self.filter.name.clear();
        self.is_open = true;
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<ViewFilter> {
        if !self.is_open {
            return None;
        }

        let mut result = None;

        egui::Window::new("Create New View")
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Basic info
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.filter.name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                        ui.text_edit_singleline(&mut self.filter.description);
                    });

                    ui.separator();

                    // Filters
                    ui.collapsing("Author/Committer", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Author:");
                            let mut author = self.filter.author_filter.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut author).changed() {
                                self.filter.author_filter = if author.is_empty() {
                                    None
                                } else {
                                    Some(author)
                                };
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Committer:");
                            let mut committer =
                                self.filter.committer_filter.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut committer).changed() {
                                self.filter.committer_filter = if committer.is_empty() {
                                    None
                                } else {
                                    Some(committer)
                                };
                            }
                        });
                    });

                    ui.collapsing("Message", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Contains:");
                            let mut message =
                                self.filter.message_filter.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut message).changed() {
                                self.filter.message_filter = if message.is_empty() {
                                    None
                                } else {
                                    Some(message)
                                };
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.filter.case_sensitive, "Case sensitive");
                            ui.checkbox(&mut self.filter.use_regex, "Use regex");
                        });
                    });

                    ui.collapsing("Date Range", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("From:");
                            let mut date_from = self.filter.date_from.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut date_from).changed() {
                                self.filter.date_from = if date_from.is_empty() {
                                    None
                                } else {
                                    Some(date_from)
                                };
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("To:");
                            let mut date_to = self.filter.date_to.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut date_to).changed() {
                                self.filter.date_to = if date_to.is_empty() {
                                    None
                                } else {
                                    Some(date_to)
                                };
                            }
                        });

                        ui.weak("Use formats like '2023-01-01', '1.week.ago', 'yesterday'");
                    });

                    ui.collapsing("Other Options", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Branch:");
                            let mut branch = self.filter.branch_filter.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut branch).changed() {
                                self.filter.branch_filter = if branch.is_empty() {
                                    None
                                } else {
                                    Some(branch)
                                };
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("File:");
                            let mut file = self.filter.file_filter.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut file).changed() {
                                self.filter.file_filter =
                                    if file.is_empty() { None } else { Some(file) };
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Max commits:");
                            let mut max_commits = self.filter.max_commits.unwrap_or(1000);
                            if ui
                                .add(egui::DragValue::new(&mut max_commits).range(1..=10000))
                                .changed()
                            {
                                self.filter.max_commits = Some(max_commits);
                            }
                        });

                        ui.checkbox(&mut self.filter.include_merges, "Include merge commits");
                    });

                    ui.separator();

                    // Buttons
                    ui.horizontal(|ui| {
                        let can_create = !self.filter.name.is_empty();

                        if ui
                            .add_enabled(can_create, egui::Button::new("Create"))
                            .clicked()
                        {
                            result = Some(self.filter.clone());
                            self.is_open = false;
                        }

                        if ui.button("Cancel").clicked() {
                            self.is_open = false;
                        }
                    });
                });
            });

        result
    }
}

pub struct EditViewDialog {
    filter: ViewFilter,
    is_open: bool,
}

impl EditViewDialog {
    pub fn new() -> Self {
        Self {
            filter: ViewFilter::default(),
            is_open: false,
        }
    }

    pub fn set_filter(&mut self, filter: ViewFilter) {
        self.filter = filter;
        self.is_open = true;
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<ViewFilter> {
        if !self.is_open {
            return None;
        }

        let mut result = None;

        egui::Window::new("Edit View")
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                // Same content as CreateViewDialog but for editing
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.filter.name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                        ui.text_edit_singleline(&mut self.filter.description);
                    });

                    ui.separator();

                    // Similar filter editing as in CreateViewDialog...
                    // (Implementation shortened for brevity, would be identical)

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            result = Some(self.filter.clone());
                            self.is_open = false;
                        }

                        if ui.button("Cancel").clicked() {
                            self.is_open = false;
                        }
                    });
                });
            });

        result
    }
}
