use crate::models::GitCommit;
use crate::state::{AppConfig, AppState};
use crate::ui::graph::CommitGraphRenderer;
use eframe::egui;

pub struct CommitGraph {
    selected_index: Option<usize>,
    graph_renderer: CommitGraphRenderer,
    view_mode: GraphViewMode,
    show_advanced_graph: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum GraphViewMode {
    Simple,   // Original simple list view
    Advanced, // New advanced graph view
    Hybrid,   // Combination of both
}

impl CommitGraph {
    pub fn new() -> Self {
        Self {
            selected_index: None,
            graph_renderer: CommitGraphRenderer::new(),
            view_mode: GraphViewMode::Advanced,
            show_advanced_graph: true,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, state: &mut AppState, config: &AppConfig) {
        // Header with view mode controls
        ui.horizontal(|ui| {
            ui.heading("Commits");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // View mode selector
                egui::ComboBox::from_label("View")
                    .selected_text(format!("{:?}", self.view_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.view_mode,
                            GraphViewMode::Simple,
                            "Simple List",
                        );
                        ui.selectable_value(
                            &mut self.view_mode,
                            GraphViewMode::Advanced,
                            "Advanced Graph",
                        );
                        ui.selectable_value(
                            &mut self.view_mode,
                            GraphViewMode::Hybrid,
                            "Hybrid View",
                        );
                    });

                // Graph controls
                if self.view_mode != GraphViewMode::Simple {
                    ui.separator();

                    if ui.button("Reset View").clicked() {
                        self.graph_renderer.reset_view();
                    }

                    ui.label("Zoom:");
                    let mut zoom = self.graph_renderer.zoom_level;
                    if ui
                        .add(egui::DragValue::new(&mut zoom).range(0.5..=3.0).speed(0.1))
                        .changed()
                    {
                        self.graph_renderer.set_zoom(zoom);
                    }

                    ui.separator();

                    // Branch filtering controls
                    ui.horizontal(|ui| {
                        ui.label("🌿 Branches:");
                        if ui.button("All").clicked() {
                            self.graph_renderer.clear_branch_filters();
                        }
                        if ui.button("Filter").clicked() {
                            // Could open branch selection dialog
                            if let Some(branches) = state.get_branches().first() {
                                self.graph_renderer.add_branch_filter(branches.clone());
                            }
                        }
                    });
                }
            });
        });

        ui.separator();

        // Use filtered commits from the view manager if available
        let filtered_commits = state.get_filtered_commits().to_vec();

        if filtered_commits.is_empty() {
            ui.label("No commits to display");
            return;
        }

        // Show commits based on view mode
        match self.view_mode {
            GraphViewMode::Simple => {
                self.show_simple_view(ui, &filtered_commits, state, config);
            }
            GraphViewMode::Advanced => {
                self.show_advanced_view(ui, &filtered_commits, state);
            }
            GraphViewMode::Hybrid => {
                self.show_hybrid_view(ui, &filtered_commits, state, config);
            }
        }
    }

    /// Show the original simple list view
    fn show_simple_view(
        &mut self,
        ui: &mut egui::Ui,
        commits: &[GitCommit],
        state: &mut AppState,
        config: &AppConfig,
    ) {
        let mut clicked_commit = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, commit) in commits.iter().enumerate() {
                let is_selected = self.selected_index == Some(index);

                ui.push_id(index, |ui| {
                    let response = self.show_commit_row(ui, commit, is_selected, config, state);

                    if response.clicked() {
                        clicked_commit = Some((index, commit.id.clone()));
                    }
                });
            }
        });

        // Handle commit selection
        if let Some((index, commit_id)) = clicked_commit {
            self.selected_index = Some(index);
            state.select_commit(commit_id);
        }
    }

    /// Show the new advanced graph view
    fn show_advanced_view(
        &mut self,
        ui: &mut egui::Ui,
        commits: &[GitCommit],
        state: &mut AppState,
    ) {
        // Create a scrollable area for the graph
        egui::ScrollArea::both().show(ui, |ui| {
            // Render the advanced graph
            let interaction_result = self.graph_renderer.render(ui, commits, state);

            // Handle selection changes from graph interactions
            if interaction_result.selection_changed {
                if let Some(ref selected_commit) = interaction_result.selected_commit {
                    if let Some(index) = commits.iter().position(|c| &c.id == selected_commit) {
                        self.selected_index = Some(index);
                        state.select_commit(selected_commit.clone());
                    }
                }
            }

            // Handle context menu requests
            if interaction_result.context_menu_requested {
                if let Some(ref context_commit) = interaction_result.context_commit {
                    self.show_context_menu(ui, context_commit, state);
                }
            }

            // Update UI state based on graph interactions
            if interaction_result.hover_changed {
                // Could trigger status bar updates or other UI feedback
            }

            if interaction_result.path_traced {
                // Could show path information in status bar
            }
        });
    }

    /// Show hybrid view with both graph and commit details
    fn show_hybrid_view(
        &mut self,
        ui: &mut egui::Ui,
        commits: &[GitCommit],
        state: &mut AppState,
        config: &AppConfig,
    ) {
        // Split view: graph on left, commit list on right
        egui::SidePanel::left("graph_panel")
            .default_width(400.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.heading("Commit Graph");
                ui.separator();

                egui::ScrollArea::both().show(ui, |ui| {
                    let interaction_result = self.graph_renderer.render(ui, commits, state);

                    // Handle graph interactions in hybrid view
                    if interaction_result.selection_changed {
                        if let Some(ref selected_commit) = interaction_result.selected_commit {
                            if let Some(index) =
                                commits.iter().position(|c| &c.id == selected_commit)
                            {
                                self.selected_index = Some(index);
                                state.select_commit(selected_commit.clone());
                            }
                        }
                    }

                    if interaction_result.context_menu_requested {
                        if let Some(ref context_commit) = interaction_result.context_commit {
                            self.show_context_menu(ui, context_commit, state);
                        }
                    }
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Commit Details");
            ui.separator();

            self.show_simple_view(ui, commits, state, config);
        });
    }

    fn show_commit_row(
        &self,
        ui: &mut egui::Ui,
        commit: &GitCommit,
        is_selected: bool,
        _config: &AppConfig,
        state: &AppState,
    ) -> egui::Response {
        let row_height = 60.0;
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_height),
            egui::Sense::click(),
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Background
            let bg_color = if is_selected {
                ui.visuals().selection.bg_fill
            } else if response.hovered() {
                ui.visuals().widgets.hovered.bg_fill
            } else {
                ui.visuals().panel_fill
            };

            painter.rect_filled(rect, 2.0, bg_color);

            // Graph visualization (simplified)
            let graph_width = 20.0;
            let graph_rect =
                egui::Rect::from_min_size(rect.min, egui::vec2(graph_width, rect.height()));
            // Draw a simple dot for each commit
            let center = graph_rect.center();
            painter.circle_filled(center, 6.0, egui::Color32::from_rgb(100, 150, 255));

            // Text area
            let text_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(graph_width + 10.0, 5.0),
                egui::vec2(rect.width() - graph_width - 15.0, rect.height() - 10.0),
            );

            // Commit info
            let text_color = if is_selected {
                ui.visuals().selection.stroke.color
            } else {
                ui.visuals().text_color()
            };

            // Short ID and message
            let id_text = format!("{}", commit.short_id);
            let message = if commit.summary.len() > 60 {
                format!("{}...", &commit.summary[..57])
            } else {
                commit.summary.clone()
            };

            painter.text(
                text_rect.min,
                egui::Align2::LEFT_TOP,
                &id_text,
                egui::FontId::monospace(12.0),
                text_color,
            );

            painter.text(
                text_rect.min + egui::vec2(80.0, 0.0),
                egui::Align2::LEFT_TOP,
                &message,
                egui::FontId::proportional(12.0),
                text_color,
            );

            // Author and date
            let author_text = format!("{}", commit.author.name);
            let date_text = commit.author.when.format("%Y-%m-%d %H:%M").to_string();

            painter.text(
                text_rect.min + egui::vec2(0.0, 20.0),
                egui::Align2::LEFT_TOP,
                &author_text,
                egui::FontId::proportional(11.0),
                ui.visuals().weak_text_color(),
            );

            painter.text(
                text_rect.min + egui::vec2(0.0, 35.0),
                egui::Align2::LEFT_TOP,
                &date_text,
                egui::FontId::proportional(11.0),
                ui.visuals().weak_text_color(),
            );

            // Parent count indicator
            if commit.parent_ids.len() > 1 {
                let merge_text = format!("Merge ({})", commit.parent_ids.len());
                painter.text(
                    text_rect.max - egui::vec2(100.0, 35.0),
                    egui::Align2::RIGHT_TOP,
                    &merge_text,
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgb(255, 150, 100),
                );
            }

            // Show references (branches, tags) for this commit
            let refs = state.get_refs_for_commit(&commit.id);
            if !refs.is_empty() {
                let mut ref_x = text_rect.max.x - 200.0;
                for (i, ref_name) in refs.iter().enumerate() {
                    if i >= 3 {
                        // Limit to 3 references to avoid clutter
                        let more_text = format!("...+{}", refs.len() - 3);
                        painter.text(
                            egui::pos2(ref_x, text_rect.min.y + 15.0),
                            egui::Align2::RIGHT_TOP,
                            &more_text,
                            egui::FontId::proportional(9.0),
                            egui::Color32::GRAY,
                        );
                        break;
                    }

                    let ref_color =
                        if ref_name.starts_with("refs/heads/") || !ref_name.contains('/') {
                            egui::Color32::from_rgb(100, 255, 100) // Green for branches
                        } else if ref_name.starts_with("refs/tags/") {
                            egui::Color32::from_rgb(255, 255, 100) // Yellow for tags
                        } else {
                            egui::Color32::from_rgb(100, 150, 255) // Blue for other refs
                        };

                    let display_name = ref_name.split('/').last().unwrap_or(ref_name);
                    painter.text(
                        egui::pos2(ref_x, text_rect.min.y + 15.0),
                        egui::Align2::RIGHT_TOP,
                        display_name,
                        egui::FontId::proportional(9.0),
                        ref_color,
                    );

                    ref_x -= 60.0; // Move left for next reference
                }
            }
        }

        response
    }

    /// Show context menu for commit operations
    fn show_context_menu(&self, ui: &mut egui::Ui, commit_id: &str, state: &mut AppState) {
        ui.menu_button("⋮", |ui| {
            ui.set_min_width(150.0);

            if ui.button("📋 Copy commit ID").clicked() {
                ui.ctx().copy_text(commit_id.to_string());
                ui.close();
            }

            if ui.button("📋 Copy short ID").clicked() {
                let short_id = if commit_id.len() >= 7 {
                    &commit_id[..7]
                } else {
                    commit_id
                };
                ui.ctx().copy_text(short_id.to_string());
                ui.close();
            }

            ui.separator();

            if ui.button("🔍 Show in diff view").clicked() {
                state.select_commit(commit_id.to_string());
                ui.close();
            }

            if ui.button("📊 Show commit details").clicked() {
                // Could open detailed commit view
                state.select_commit(commit_id.to_string());
                ui.close();
            }

            ui.separator();

            if ui.button("🌿 Create branch here").clicked() {
                // Could open branch creation dialog
                ui.close();
            }

            if ui.button("🏷️ Create tag here").clicked() {
                // Could open tag creation dialog
                ui.close();
            }

            ui.separator();

            if ui.button("🔄 Reset to this commit").clicked() {
                // Could show reset confirmation dialog
                ui.close();
            }

            if ui.button("🍒 Cherry-pick").clicked() {
                // Could initiate cherry-pick operation
                ui.close();
            }
        });
    }
}
