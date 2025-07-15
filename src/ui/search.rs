use eframe::egui;
use crate::state::AppState;

pub struct SearchPanel {
    search_text: String,
    search_focused: bool,
}

impl SearchPanel {
    pub fn new() -> Self {
        Self {
            search_text: String::new(),
            search_focused: false,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, state: &mut AppState) {
        ui.horizontal(|ui| {
            let search_response = ui.text_edit_singleline(&mut self.search_text);
            
            if search_response.changed() {
                state.search_commits(&self.search_text);
            }

            if ui.button("🔍").clicked() {
                state.search_commits(&self.search_text);
            }

            if ui.button("Clear").clicked() {
                self.search_text.clear();
                state.search_commits("");
            }

            ui.separator();

            ui.label("Author:");
            let author_response = ui.text_edit_singleline(&mut state.filter_author);
            
            if author_response.changed() {
                // Apply author filter
                self.apply_filters(state);
            }

            ui.separator();

            if ui.button("Refresh").clicked() {
                state.start_streaming_commits();
            }
        });
    }

    fn apply_filters(&self, state: &mut AppState) {
        // This would implement filtering logic
        // For now, we'll just refresh commits
        state.start_streaming_commits();
    }
}