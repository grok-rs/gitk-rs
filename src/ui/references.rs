use crate::state::AppState;
use eframe::egui;

pub struct ReferencesPanel {
    show_local_branches: bool,
    show_remote_branches: bool,
    show_tags: bool,
    filter_text: String,
}

impl ReferencesPanel {
    pub fn new() -> Self {
        Self {
            show_local_branches: true,
            show_remote_branches: false,
            show_tags: true,
            filter_text: String::new(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, state: &mut AppState) {
        ui.heading("References");
        ui.separator();

        // Filter and controls
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter_text);

            if ui.button("🔄").on_hover_text("Refresh").clicked() {
                state.refresh_references();
            }
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_local_branches, "Local");
            ui.checkbox(&mut self.show_remote_branches, "Remote");
            ui.checkbox(&mut self.show_tags, "Tags");
        });

        // Update state based on UI settings
        state.show_remote_branches = self.show_remote_branches;

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Current branch indicator
            if let Some(current_branch) = state.get_current_branch() {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::GREEN, "📍");
                    ui.strong(format!("Current: {}", current_branch));
                    if state.is_detached_head() {
                        ui.colored_label(egui::Color32::YELLOW, "(detached HEAD)");
                    }
                });
                ui.separator();
            }

            // Local branches
            if self.show_local_branches {
                self.show_local_branches_section(ui, state);
            }

            // Remote branches
            if self.show_remote_branches {
                self.show_remote_branches_section(ui, state);
            }

            // Tags
            if self.show_tags {
                self.show_tags_section(ui, state);
            }
        });
    }

    fn show_local_branches_section(&self, ui: &mut egui::Ui, state: &mut AppState) {
        ui.collapsing("Local Branches", |ui| {
            let branches = state.get_branches();
            let current_branch = state.get_current_branch().cloned();
            let mut branch_to_switch = None;

            for branch in branches.iter() {
                if !self.matches_filter(branch) {
                    continue;
                }

                // Skip remote branches in this section
                if branch.contains('/') && !branch.starts_with("refs/heads/") {
                    continue;
                }

                ui.horizontal(|ui| {
                    let is_current = current_branch
                        .as_ref()
                        .map(|cb| cb == branch)
                        .unwrap_or(false);

                    if is_current {
                        ui.colored_label(egui::Color32::GREEN, "📍");
                    } else {
                        ui.label("  ");
                    }

                    let response = ui.selectable_label(is_current, branch);

                    if response.clicked() && !is_current {
                        branch_to_switch = Some(branch.clone());
                    }

                    response.context_menu(|ui| {
                        if ui.button("Switch to branch").clicked() {
                            branch_to_switch = Some(branch.clone());
                            ui.close_menu();
                        }

                        if ui.button("View commits").clicked() {
                            // This would switch to viewing this branch's commits
                            branch_to_switch = Some(branch.clone());
                            ui.close_menu();
                        }

                        ui.separator();

                        if ui.button("Delete branch").clicked() {
                            // TODO: Implement branch deletion with confirmation
                            ui.close_menu();
                        }
                    });
                });
            }

            // Handle branch switching outside the loop
            if let Some(branch) = branch_to_switch {
                state.switch_to_branch(&branch);
            }
        });
    }

    fn show_remote_branches_section(&self, ui: &mut egui::Ui, state: &AppState) {
        ui.collapsing("Remote Branches", |ui| {
            if let Some(ref ref_manager) = state.ref_manager {
                for branch in ref_manager.get_remote_branches() {
                    if !self.matches_filter(&branch.name) {
                        continue;
                    }

                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::LIGHT_BLUE, "🌐");
                        let response = ui.selectable_label(false, &branch.name);

                        response.context_menu(|ui| {
                            if ui.button("Create local branch").clicked() {
                                // TODO: Implement creating local branch from remote
                                ui.close_menu();
                            }

                            if ui.button("View commits").clicked() {
                                // TODO: Implement viewing remote branch commits
                                ui.close_menu();
                            }
                        });
                    });
                }
            }
        });
    }

    fn show_tags_section(&self, ui: &mut egui::Ui, state: &AppState) {
        ui.collapsing("Tags", |ui| {
            let tags = state.get_tags();

            for tag in tags.iter() {
                if !self.matches_filter(tag) {
                    continue;
                }

                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::YELLOW, "🏷️");
                    let response = ui.selectable_label(false, tag);

                    response.context_menu(|ui| {
                        if ui.button("View commit").clicked() {
                            // TODO: Implement jumping to tag commit
                            ui.close_menu();
                        }

                        if ui.button("Create branch from tag").clicked() {
                            // TODO: Implement creating branch from tag
                            ui.close_menu();
                        }

                        ui.separator();

                        if ui.button("Delete tag").clicked() {
                            // TODO: Implement tag deletion with confirmation
                            ui.close_menu();
                        }
                    });
                });
            }
        });
    }

    fn matches_filter(&self, name: &str) -> bool {
        if self.filter_text.is_empty() {
            return true;
        }

        name.to_lowercase()
            .contains(&self.filter_text.to_lowercase())
    }
}

/// Dialog for creating new branches
pub struct CreateBranchDialog {
    branch_name: String,
    from_commit: String,
    show: bool,
}

impl CreateBranchDialog {
    pub fn new() -> Self {
        Self {
            branch_name: String::new(),
            from_commit: String::new(),
            show: false,
        }
    }

    pub fn show_dialog(&mut self) {
        self.show = true;
        self.branch_name.clear();
        self.from_commit.clear();
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &AppState) -> Option<(String, String)> {
        if !self.show {
            return None;
        }

        let mut result = None;
        let mut keep_open = true;

        egui::Window::new("Create Branch")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Branch name:");
                        ui.text_edit_singleline(&mut self.branch_name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("From commit:");
                        ui.text_edit_singleline(&mut self.from_commit);
                    });

                    if self.from_commit.is_empty() {
                        if let Some(current_branch) = state.get_current_branch() {
                            ui.label(format!(
                                "Will branch from current HEAD of '{}'",
                                current_branch
                            ));
                        }
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        let can_create = !self.branch_name.is_empty()
                            && !self.branch_name.contains(' ')
                            && !self.branch_name.contains('/');

                        if ui
                            .add_enabled(can_create, egui::Button::new("Create"))
                            .clicked()
                        {
                            let from_commit = if self.from_commit.is_empty() {
                                "HEAD".to_string()
                            } else {
                                self.from_commit.clone()
                            };
                            result = Some((self.branch_name.clone(), from_commit));
                            keep_open = false;
                        }

                        if ui.button("Cancel").clicked() {
                            keep_open = false;
                        }
                    });
                });
            });

        if !keep_open {
            self.show = false;
        }

        result
    }
}

/// Dialog for creating new tags
pub struct CreateTagDialog {
    tag_name: String,
    tag_message: String,
    target_commit: String,
    is_annotated: bool,
    show: bool,
}

impl CreateTagDialog {
    pub fn new() -> Self {
        Self {
            tag_name: String::new(),
            tag_message: String::new(),
            target_commit: String::new(),
            is_annotated: false,
            show: false,
        }
    }

    pub fn show_dialog(&mut self, commit_sha: Option<&str>) {
        self.show = true;
        self.tag_name.clear();
        self.tag_message.clear();
        self.target_commit = commit_sha
            .map(|s| s.to_string())
            .unwrap_or_else(|| "HEAD".to_string());
        self.is_annotated = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<(String, String, Option<String>)> {
        if !self.show {
            return None;
        }

        let mut result = None;
        let mut keep_open = true;

        egui::Window::new("Create Tag")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Tag name:");
                        ui.text_edit_singleline(&mut self.tag_name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Target commit:");
                        ui.text_edit_singleline(&mut self.target_commit);
                    });

                    ui.checkbox(&mut self.is_annotated, "Annotated tag");

                    if self.is_annotated {
                        ui.horizontal(|ui| {
                            ui.label("Message:");
                            ui.text_edit_singleline(&mut self.tag_message);
                        });
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        let can_create = !self.tag_name.is_empty()
                            && !self.target_commit.is_empty()
                            && (!self.is_annotated || !self.tag_message.is_empty());

                        if ui
                            .add_enabled(can_create, egui::Button::new("Create"))
                            .clicked()
                        {
                            let message = if self.is_annotated && !self.tag_message.is_empty() {
                                Some(self.tag_message.clone())
                            } else {
                                None
                            };
                            result =
                                Some((self.tag_name.clone(), self.target_commit.clone(), message));
                            keep_open = false;
                        }

                        if ui.button("Cancel").clicked() {
                            keep_open = false;
                        }
                    });
                });
            });

        if !keep_open {
            self.show = false;
        }

        result
    }
}
