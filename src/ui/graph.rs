use eframe::egui;
use std::collections::HashMap;
use crate::models::GitCommit;
use crate::state::AppState;

/// Advanced commit graph rendering system
/// Based on the original gitk's sophisticated branch layout algorithm
pub struct CommitGraphRenderer {
    /// Graph layout cache for performance
    layout_cache: HashMap<String, GraphLayout>,
    /// Color palette for branches
    branch_colors: Vec<egui::Color32>,
    /// Maximum number of parallel branches to display
    max_branches: usize,
    /// Row height for commits
    row_height: f32,
    /// Column width for branches
    column_width: f32,
    /// Current zoom level
    pub zoom_level: f32,
    /// Pan offset
    pan_offset: egui::Vec2,
    /// Selected commit path highlighting
    highlighted_path: Option<Vec<String>>,
    /// Graph interaction state
    interaction_state: GraphInteractionState,
    /// Filtered branch names for highlighting
    filtered_branches: Vec<String>,
    /// Whether to show all branches or only filtered ones
    show_filtered_only: bool,
}

/// Complete layout information for the commit graph
#[derive(Debug, Clone)]
struct GraphLayout {
    /// Commits with their display positions
    commit_positions: HashMap<String, CommitPosition>,
    /// Branch lanes allocation
    branch_lanes: HashMap<String, BranchLane>,
    /// Merge connection lines
    merge_lines: Vec<MergeLine>,
    /// Total graph width
    total_width: f32,
    /// Total graph height
    total_height: f32,
}

/// Position and visual information for a commit
#[derive(Debug, Clone)]
struct CommitPosition {
    /// Screen coordinates
    pos: egui::Pos2,
    /// Branch lane index
    lane: usize,
    /// Visual radius
    radius: f32,
    /// Color for this commit
    color: egui::Color32,
    /// Parent connections
    parent_lines: Vec<ConnectionLine>,
    /// Child connections
    child_lines: Vec<ConnectionLine>,
    /// References at this commit (tags, branches)
    refs: Vec<RefLabel>,
}

/// Branch lane with consistent coloring and metadata
#[derive(Debug, Clone)]
struct BranchLane {
    /// Lane index (0 = leftmost)
    index: usize,
    /// Branch color
    color: egui::Color32,
    /// Branch name (if known)
    name: Option<String>,
    /// Active range (start_row, end_row)
    active_range: (usize, usize),
    /// Branch type classification
    branch_type: BranchType,
    /// Merge conflict indicators
    conflict_markers: Vec<ConflictMarker>,
    /// Branch priority (for lane assignment)
    priority: BranchPriority,
}

/// Classification of branch types for visual distinction
#[derive(Debug, Clone, PartialEq)]
enum BranchType {
    Main,         // Main/master branch
    Feature,      // Feature branches
    Release,      // Release branches
    Hotfix,       // Hotfix branches
    Unknown,      // Unclassified branches
}

/// Conflict markers for visual indication
#[derive(Debug, Clone)]
struct ConflictMarker {
    /// Position where conflict occurs
    position: egui::Pos2,
    /// Type of conflict
    conflict_type: ConflictType,
    /// Severity level
    severity: ConflictSeverity,
}

/// Types of conflicts that can be detected
#[derive(Debug, Clone, PartialEq)]
enum ConflictType {
    MergeConflict,    // Traditional merge conflicts
    LaneCollision,    // Visual lane collisions
    FastForward,      // Fast-forward possibilities
    Divergence,       // Branch divergence points
}

/// Conflict severity for visual priority
#[derive(Debug, Clone, PartialEq)]
enum ConflictSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Branch priority for intelligent lane assignment
#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum BranchPriority {
    VeryHigh = 4,  // Main branches
    High = 3,      // Active development branches
    Medium = 2,    // Feature branches
    Low = 1,       // Inactive or old branches
}

/// Line connecting commits (parent-child relationship)
#[derive(Debug, Clone)]
struct ConnectionLine {
    /// Start position
    start: egui::Pos2,
    /// End position
    end: egui::Pos2,
    /// Control points for curves
    control_points: Vec<egui::Pos2>,
    /// Line color
    color: egui::Color32,
    /// Line thickness
    thickness: f32,
    /// Line style (solid, dashed, etc.)
    style: LineStyle,
}

/// Complex merge visualization
#[derive(Debug, Clone)]
struct MergeLine {
    /// Merge commit position
    merge_pos: egui::Pos2,
    /// Parent positions
    parent_positions: Vec<egui::Pos2>,
    /// Visual style for merge indicator
    style: MergeStyle,
    /// Color
    color: egui::Color32,
}

/// Reference label (branch, tag) display
#[derive(Debug, Clone)]
struct RefLabel {
    /// Label text
    text: String,
    /// Position relative to commit
    offset: egui::Vec2,
    /// Label background color
    background: egui::Color32,
    /// Text color
    text_color: egui::Color32,
    /// Reference type
    ref_type: RefType,
}

/// Different types of references
#[derive(Debug, Clone, PartialEq)]
enum RefType {
    LocalBranch,
    RemoteBranch,
    Tag,
    Head,
}

/// Line drawing styles
#[derive(Debug, Clone, PartialEq)]
enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

/// Merge visualization styles
#[derive(Debug, Clone, PartialEq)]
enum MergeStyle {
    Simple,      // Simple merge indicator
    Octopus,     // Multi-parent merge
    Highlighted, // Selected merge
}

/// Graph interaction state
#[derive(Debug, Default)]
struct GraphInteractionState {
    /// Mouse hover position
    hover_pos: Option<egui::Pos2>,
    /// Currently hovered commit
    hovered_commit: Option<String>,
    /// Mouse drag state
    is_dragging: bool,
    /// Drag start position
    drag_start: Option<egui::Pos2>,
    /// Selection state
    selected_commits: Vec<String>,
}

/// Result of graph interaction processing
#[derive(Debug, Default)]
pub struct GraphInteractionResult {
    /// Whether hover state changed
    pub hover_changed: bool,
    /// Currently hovered commit
    pub hovered_commit: Option<String>,
    /// Whether selection changed
    pub selection_changed: bool,
    /// Selected commit (for single selections)
    pub selected_commit: Option<String>,
    /// Whether view (zoom/pan) changed
    pub view_changed: bool,
    /// Whether a path was traced
    pub path_traced: bool,
    /// Whether context menu was requested
    pub context_menu_requested: bool,
    /// Commit for context menu
    pub context_commit: Option<String>,
}

impl CommitGraphRenderer {
    pub fn new() -> Self {
        Self {
            layout_cache: HashMap::new(),
            branch_colors: Self::create_color_palette(),
            max_branches: 32,
            row_height: 24.0,
            column_width: 16.0,
            zoom_level: 1.0,
            pan_offset: egui::Vec2::ZERO,
            highlighted_path: None,
            interaction_state: GraphInteractionState::default(),
            filtered_branches: Vec::new(),
            show_filtered_only: false,
        }
    }
    
    /// Create a sophisticated color palette for branches with better contrast and distinction
    fn create_color_palette() -> Vec<egui::Color32> {
        vec![
            // Primary colors with high contrast
            egui::Color32::from_rgb(220, 38, 127),  // Vibrant Pink
            egui::Color32::from_rgb(52, 168, 83),   // Green
            egui::Color32::from_rgb(66, 133, 244),  // Blue
            egui::Color32::from_rgb(251, 188, 5),   // Yellow
            egui::Color32::from_rgb(156, 39, 176),  // Purple
            egui::Color32::from_rgb(255, 87, 34),   // Deep Orange
            egui::Color32::from_rgb(0, 172, 193),   // Cyan
            egui::Color32::from_rgb(139, 195, 74),  // Light Green
            
            // Secondary colors for more branches
            egui::Color32::from_rgb(255, 112, 67),  // Coral
            egui::Color32::from_rgb(92, 107, 192),  // Indigo
            egui::Color32::from_rgb(174, 213, 129), // Pale Green
            egui::Color32::from_rgb(255, 183, 77),  // Amber
            egui::Color32::from_rgb(240, 98, 146),  // Pink
            egui::Color32::from_rgb(129, 199, 132), // Light Green
            egui::Color32::from_rgb(100, 181, 246), // Light Blue
            egui::Color32::from_rgb(171, 71, 188),  // Deep Purple
            
            // Tertiary colors for even more branches
            egui::Color32::from_rgb(255, 138, 101), // Peach
            egui::Color32::from_rgb(149, 117, 205), // Medium Purple
            egui::Color32::from_rgb(102, 187, 106), // Green
            egui::Color32::from_rgb(255, 202, 40),  // Gold
            egui::Color32::from_rgb(179, 136, 255), // Lavender
            egui::Color32::from_rgb(255, 171, 145), // Light Coral
            egui::Color32::from_rgb(130, 177, 255), // Periwinkle
            egui::Color32::from_rgb(165, 214, 167), // Mint Green
        ]
    }
    
    /// Main rendering function - displays the commit graph
    pub fn render(&mut self, ui: &mut egui::Ui, commits: &[GitCommit], state: &AppState) -> GraphInteractionResult {
        let available_rect = ui.available_rect_before_wrap();
        
        // Create or update graph layout
        let layout = self.compute_graph_layout(commits, available_rect.size());
        
        // Handle user interactions
        let interaction_result = self.handle_interactions(ui, &layout);
        
        // Render the graph
        self.render_graph(ui, &layout, commits, available_rect);
        
        // Render commit details on hover
        self.render_hover_tooltip(ui, commits);
        
        // Render interaction hints
        self.render_interaction_hints(ui, available_rect);
        
        interaction_result
    }
    
    /// Compute the complete graph layout using the original gitk algorithm
    fn compute_graph_layout(&mut self, commits: &[GitCommit], available_size: egui::Vec2) -> GraphLayout {
        // Use cached layout if available and valid
        let cache_key = self.generate_cache_key(commits);
        if let Some(cached_layout) = self.layout_cache.get(&cache_key) {
            return cached_layout.clone();
        }
        
        let mut layout = GraphLayout {
            commit_positions: HashMap::new(),
            branch_lanes: HashMap::new(),
            merge_lines: Vec::new(),
            total_width: 0.0,
            total_height: 0.0,
        };
        
        // Step 1: Analyze commit relationships and build parent-child map
        let (parent_map, child_map) = self.build_relationship_maps(commits);
        
        // Step 2: Assign lanes to commits using the gitk algorithm
        let lane_assignments = self.assign_commit_lanes(commits, &parent_map, &child_map);
        
        // Step 3: Calculate positions for each commit
        for (row, commit) in commits.iter().enumerate() {
            let lane = lane_assignments.get(&commit.id).unwrap_or(&0);
            let pos = egui::Pos2 {
                x: (*lane as f32) * self.column_width * self.zoom_level + self.pan_offset.x,
                y: (row as f32) * self.row_height * self.zoom_level + self.pan_offset.y,
            };
            
            let color = self.branch_colors[*lane % self.branch_colors.len()];
            
            // Create parent connection lines
            let parent_lines = self.create_parent_lines(
                commit, pos, &lane_assignments, &parent_map, row, commits
            );
            
            // Create child connection lines
            let child_lines = self.create_child_lines(
                commit, pos, &lane_assignments, &child_map, row, commits
            );
            
            // Create reference labels  
            let refs = Vec::new(); // TODO: Implement ref labels integration
            
            let commit_pos = CommitPosition {
                pos,
                lane: *lane,
                radius: 4.0 * self.zoom_level,
                color,
                parent_lines,
                child_lines,
                refs,
            };
            
            layout.commit_positions.insert(commit.id.clone(), commit_pos);
        }
        
        // Step 4: Create merge visualization
        layout.merge_lines = self.create_merge_lines(commits, &layout.commit_positions);
        
        // Step 5: Calculate total dimensions
        layout.total_width = (lane_assignments.values().max().unwrap_or(&0) + 1) as f32 * self.column_width * self.zoom_level;
        layout.total_height = commits.len() as f32 * self.row_height * self.zoom_level;
        
        // Cache the layout
        self.layout_cache.insert(cache_key, layout.clone());
        layout
    }
    
    /// Build parent-child relationship maps for efficient lookups
    fn build_relationship_maps(&self, commits: &[GitCommit]) -> (HashMap<String, Vec<String>>, HashMap<String, Vec<String>>) {
        let mut parent_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut child_map: HashMap<String, Vec<String>> = HashMap::new();
        
        for commit in commits {
            parent_map.insert(commit.id.clone(), commit.parent_ids.clone());
            
            for parent_id in &commit.parent_ids {
                child_map.entry(parent_id.clone())
                    .or_insert_with(Vec::new)
                    .push(commit.id.clone());
            }
        }
        
        (parent_map, child_map)
    }
    
    /// Assign lane positions using the sophisticated gitk algorithm
    fn assign_commit_lanes(
        &self, 
        commits: &[GitCommit], 
        parent_map: &HashMap<String, Vec<String>>,
        child_map: &HashMap<String, Vec<String>>
    ) -> HashMap<String, usize> {
        let mut lane_assignments: HashMap<String, usize> = HashMap::new();
        let mut active_lanes: Vec<Option<String>> = vec![None; self.max_branches];
        let mut next_free_lane = 0;
        
        for commit in commits {
            let commit_id = &commit.id;
            let parents = parent_map.get(commit_id).cloned().unwrap_or_default();
            
            // Determine the best lane for this commit
            let assigned_lane = if parents.is_empty() {
                // Root commit - assign to next free lane
                let lane = self.find_free_lane(&active_lanes);
                active_lanes[lane] = Some(commit_id.clone());
                lane
            } else if parents.len() == 1 {
                // Single parent - try to continue on parent's lane
                let parent_id = &parents[0];
                if let Some(parent_lane) = lane_assignments.get(parent_id) {
                    // Check if parent's lane is available
                    if active_lanes[*parent_lane].as_ref() == Some(parent_id) {
                        active_lanes[*parent_lane] = Some(commit_id.clone());
                        *parent_lane
                    } else {
                        // Parent's lane is occupied, find new lane
                        let lane = self.find_free_lane(&active_lanes);
                        active_lanes[lane] = Some(commit_id.clone());
                        lane
                    }
                } else {
                    // Parent not found, assign new lane
                    let lane = self.find_free_lane(&active_lanes);
                    active_lanes[lane] = Some(commit_id.clone());
                    lane
                }
            } else {
                // Merge commit - complex lane assignment
                self.assign_merge_commit_lane(commit_id, &parents, &mut active_lanes, &lane_assignments)
            };
            
            lane_assignments.insert(commit_id.clone(), assigned_lane);
            
            // Update active lanes based on children
            self.update_active_lanes_for_children(commit_id, &mut active_lanes, child_map);
        }
        
        lane_assignments
    }
    
    /// Find the next available lane
    fn find_free_lane(&self, active_lanes: &[Option<String>]) -> usize {
        for (i, lane) in active_lanes.iter().enumerate() {
            if lane.is_none() {
                return i;
            }
        }
        active_lanes.len() // Expand if needed
    }
    
    /// Handle merge commit lane assignment
    fn assign_merge_commit_lane(
        &self,
        commit_id: &str,
        parents: &[String],
        active_lanes: &mut Vec<Option<String>>,
        lane_assignments: &HashMap<String, usize>
    ) -> usize {
        // For merge commits, try to use the lane of the first parent
        if let Some(first_parent) = parents.first() {
            if let Some(parent_lane) = lane_assignments.get(first_parent) {
                if *parent_lane < active_lanes.len() && 
                   active_lanes[*parent_lane].as_ref() == Some(first_parent) {
                    active_lanes[*parent_lane] = Some(commit_id.to_string());
                    return *parent_lane;
                }
            }
        }
        
        // Fallback: assign to free lane
        let lane = self.find_free_lane(active_lanes);
        if lane >= active_lanes.len() {
            active_lanes.resize(lane + 1, None);
        }
        active_lanes[lane] = Some(commit_id.to_string());
        lane
    }
    
    /// Update active lanes based on commit children
    fn update_active_lanes_for_children(
        &self,
        commit_id: &str,
        active_lanes: &mut Vec<Option<String>>,
        child_map: &HashMap<String, Vec<String>>
    ) {
        // If this commit has no children, its lane becomes free
        if !child_map.contains_key(commit_id) {
            for lane in active_lanes.iter_mut() {
                if lane.as_ref() == Some(&commit_id.to_string()) {
                    *lane = None;
                    break;
                }
            }
        }
    }
    
    /// Create connection lines between parent and child commits
    fn create_parent_lines(
        &self,
        commit: &GitCommit,
        commit_pos: egui::Pos2,
        lane_assignments: &HashMap<String, usize>,
        parent_map: &HashMap<String, Vec<String>>,
        row: usize,
        commits: &[GitCommit]
    ) -> Vec<ConnectionLine> {
        let mut lines = Vec::new();
        
        if let Some(parents) = parent_map.get(&commit.id) {
            for parent_id in parents {
                if let Some(parent_lane) = lane_assignments.get(parent_id) {
                    // Find parent position
                    if let Some(parent_row) = commits.iter().position(|c| &c.id == parent_id) {
                        let parent_pos = egui::Pos2 {
                            x: (*parent_lane as f32) * self.column_width * self.zoom_level + self.pan_offset.x,
                            y: (parent_row as f32) * self.row_height * self.zoom_level + self.pan_offset.y,
                        };
                        
                        let color = self.branch_colors[*parent_lane % self.branch_colors.len()];
                        
                        // Create curved line for lane changes
                        let line = if lane_assignments.get(&commit.id).unwrap_or(&0) != parent_lane {
                            self.create_curved_line(commit_pos, parent_pos, color)
                        } else {
                            self.create_straight_line(commit_pos, parent_pos, color)
                        };
                        
                        lines.push(line);
                    }
                }
            }
        }
        
        lines
    }
    
    /// Create child connection lines (for highlighting paths)
    fn create_child_lines(
        &self,
        commit: &GitCommit,
        commit_pos: egui::Pos2,
        lane_assignments: &HashMap<String, usize>,
        child_map: &HashMap<String, Vec<String>>,
        row: usize,
        commits: &[GitCommit]
    ) -> Vec<ConnectionLine> {
        let mut lines = Vec::new();
        
        if let Some(children) = child_map.get(&commit.id) {
            for child_id in children {
                if let Some(child_lane) = lane_assignments.get(child_id) {
                    if let Some(child_row) = commits.iter().position(|c| &c.id == child_id) {
                        let child_pos = egui::Pos2 {
                            x: (*child_lane as f32) * self.column_width * self.zoom_level + self.pan_offset.x,
                            y: (child_row as f32) * self.row_height * self.zoom_level + self.pan_offset.y,
                        };
                        
                        let color = self.branch_colors[*child_lane % self.branch_colors.len()];
                        
                        let line = if lane_assignments.get(&commit.id).unwrap_or(&0) != child_lane {
                            self.create_curved_line(commit_pos, child_pos, color)
                        } else {
                            self.create_straight_line(commit_pos, child_pos, color)
                        };
                        
                        lines.push(line);
                    }
                }
            }
        }
        
        lines
    }
    
    /// Create a straight connection line
    fn create_straight_line(&self, start: egui::Pos2, end: egui::Pos2, color: egui::Color32) -> ConnectionLine {
        ConnectionLine {
            start,
            end,
            control_points: Vec::new(),
            color,
            thickness: 2.0 * self.zoom_level,
            style: LineStyle::Solid,
        }
    }
    
    /// Create a curved connection line for lane changes
    fn create_curved_line(&self, start: egui::Pos2, end: egui::Pos2, color: egui::Color32) -> ConnectionLine {
        // Calculate control points for Bezier curve
        let mid_y = (start.y + end.y) / 2.0;
        let control1 = egui::Pos2::new(start.x, mid_y);
        let control2 = egui::Pos2::new(end.x, mid_y);
        
        ConnectionLine {
            start,
            end,
            control_points: vec![control1, control2],
            color,
            thickness: 2.0 * self.zoom_level,
            style: LineStyle::Solid,
        }
    }
    
    /// Create merge visualization lines
    fn create_merge_lines(&self, commits: &[GitCommit], positions: &HashMap<String, CommitPosition>) -> Vec<MergeLine> {
        let mut merge_lines = Vec::new();
        
        for commit in commits {
            if commit.parent_ids.len() > 1 {
                // This is a merge commit
                if let Some(merge_pos_info) = positions.get(&commit.id) {
                    let mut parent_positions = Vec::new();
                    
                    for parent_id in &commit.parent_ids {
                        if let Some(parent_pos_info) = positions.get(parent_id) {
                            parent_positions.push(parent_pos_info.pos);
                        }
                    }
                    
                    let style = if commit.parent_ids.len() > 2 {
                        MergeStyle::Octopus
                    } else {
                        MergeStyle::Simple
                    };
                    
                    let merge_line = MergeLine {
                        merge_pos: merge_pos_info.pos,
                        parent_positions,
                        style,
                        color: merge_pos_info.color,
                    };
                    
                    merge_lines.push(merge_line);
                }
            }
        }
        
        merge_lines
    }
    
    /// Create reference labels for branches and tags
    fn create_ref_labels(&self, commit: &GitCommit, state: &AppState) -> Vec<RefLabel> {
        let mut labels = Vec::new();
        
        // Get references for this commit from the state
        let refs = state.get_refs_for_commit(&commit.id);
        
        for (i, ref_name) in refs.iter().enumerate() {
            let ref_type = if ref_name.starts_with("refs/heads/") {
                RefType::LocalBranch
            } else if ref_name.starts_with("refs/remotes/") {
                RefType::RemoteBranch
            } else if ref_name.starts_with("refs/tags/") {
                RefType::Tag
            } else if ref_name == "HEAD" {
                RefType::Head
            } else {
                RefType::LocalBranch
            };
            
            let (background, text_color) = match ref_type {
                RefType::LocalBranch => (egui::Color32::from_rgb(0, 128, 0), egui::Color32::WHITE),
                RefType::RemoteBranch => (egui::Color32::from_rgb(128, 0, 128), egui::Color32::WHITE),
                RefType::Tag => (egui::Color32::from_rgb(255, 165, 0), egui::Color32::BLACK),
                RefType::Head => (egui::Color32::from_rgb(255, 0, 0), egui::Color32::WHITE),
            };
            
            let label = RefLabel {
                text: ref_name.clone(),
                offset: egui::Vec2::new(20.0 + (i as f32 * 80.0), -10.0),
                background,
                text_color,
                ref_type,
            };
            
            labels.push(label);
        }
        
        labels
    }
    
    /// Handle user interactions with the graph
    fn handle_interactions(&mut self, ui: &mut egui::Ui, layout: &GraphLayout) -> GraphInteractionResult {
        let response = ui.interact(ui.available_rect_before_wrap(), ui.id(), egui::Sense::click_and_drag());
        let mut interaction_result = GraphInteractionResult::default();
        
        // Handle mouse hover
        if let Some(hover_pos) = response.hover_pos() {
            self.interaction_state.hover_pos = Some(hover_pos);
            let old_hovered = self.interaction_state.hovered_commit.clone();
            self.interaction_state.hovered_commit = self.find_commit_at_position(hover_pos, layout);
            
            // Check if hover changed
            if old_hovered != self.interaction_state.hovered_commit {
                interaction_result.hover_changed = true;
                interaction_result.hovered_commit = self.interaction_state.hovered_commit.clone();
            }
        } else {
            if self.interaction_state.hovered_commit.is_some() {
                interaction_result.hover_changed = true;
            }
            self.interaction_state.hover_pos = None;
            self.interaction_state.hovered_commit = None;
        }
        
        // Handle mouse clicks for selection and path highlighting
        if response.clicked() {
            if let Some(ref commit_id) = self.interaction_state.hovered_commit {
                // Toggle commit selection
                if let Some(pos) = self.interaction_state.selected_commits.iter().position(|c| c == commit_id) {
                    self.interaction_state.selected_commits.remove(pos);
                } else {
                    // Limit selection to reasonable number
                    if self.interaction_state.selected_commits.len() < 10 {
                        self.interaction_state.selected_commits.push(commit_id.clone());
                    }
                }
                
                interaction_result.selection_changed = true;
                interaction_result.selected_commit = Some(commit_id.clone());
                
                // Auto-highlight path for single selection
                if self.interaction_state.selected_commits.len() == 1 {
                    let commit_id_clone = commit_id.clone();
                    self.auto_highlight_commit_path(&commit_id_clone, layout);
                } else if self.interaction_state.selected_commits.is_empty() {
                    self.highlighted_path = None;
                }
            }
        }
        
        // Handle double-click for path tracing
        if response.double_clicked() {
            if let Some(ref commit_id) = self.interaction_state.hovered_commit {
                let commit_id_clone = commit_id.clone();
                self.trace_commit_ancestry(&commit_id_clone, layout);
                interaction_result.path_traced = true;
            }
        }
        
        // Handle right-click for context menu
        if response.secondary_clicked() {
            if let Some(ref commit_id) = self.interaction_state.hovered_commit {
                interaction_result.context_menu_requested = true;
                interaction_result.context_commit = Some(commit_id.clone());
            }
        }
        
        // Handle mouse drag for panning
        if response.dragged() {
            let drag_delta = response.drag_delta();
            if !self.interaction_state.is_dragging {
                self.interaction_state.is_dragging = true;
                self.interaction_state.drag_start = response.interact_pointer_pos();
            }
            
            // Apply panning with momentum
            self.pan_offset += drag_delta;
            interaction_result.view_changed = true;
            
            // Clear layout cache when panning significantly
            if drag_delta.length() > 5.0 {
                self.layout_cache.clear();
            }
        } else {
            self.interaction_state.is_dragging = false;
        }
        
        // Handle zoom with scroll wheel (with focus point)
        ui.input(|i| {
            let scroll_delta = i.raw_scroll_delta.y;
            if scroll_delta.abs() > 0.1 {
                let old_zoom = self.zoom_level;
                
                if scroll_delta > 0.0 {
                    self.zoom_level = (self.zoom_level * 1.1).clamp(0.1, 5.0);
                } else {
                    self.zoom_level = (self.zoom_level * 0.9).clamp(0.1, 5.0);
                }
                
                // Zoom towards mouse cursor if hovering
                if let Some(hover_pos) = self.interaction_state.hover_pos {
                    let zoom_factor = self.zoom_level / old_zoom;
                    let cursor_offset = hover_pos - ui.available_rect_before_wrap().center();
                    let new_offset = cursor_offset * (zoom_factor - 1.0);
                    self.pan_offset -= new_offset;
                }
                
                interaction_result.view_changed = true;
                self.layout_cache.clear(); // Invalidate cache on zoom
            }
        });
        
        // Handle keyboard shortcuts
        ui.input(|i| {
            // Reset view with 'R' key
            if i.key_pressed(egui::Key::R) {
                self.reset_view();
                interaction_result.view_changed = true;
            }
            
            // Clear selection with Escape
            if i.key_pressed(egui::Key::Escape) {
                if !self.interaction_state.selected_commits.is_empty() || self.highlighted_path.is_some() {
                    self.interaction_state.selected_commits.clear();
                    self.highlighted_path = None;
                    interaction_result.selection_changed = true;
                }
            }
            
            // Fit to window with 'F' key
            if i.key_pressed(egui::Key::F) {
                self.fit_to_window(layout, ui.available_rect_before_wrap());
                interaction_result.view_changed = true;
            }
            
            // Zoom in/out with +/- keys
            if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                self.zoom_level = (self.zoom_level * 1.2).clamp(0.1, 5.0);
                self.layout_cache.clear();
                interaction_result.view_changed = true;
            }
            if i.key_pressed(egui::Key::Minus) {
                self.zoom_level = (self.zoom_level * 0.8).clamp(0.1, 5.0);
                self.layout_cache.clear();
                interaction_result.view_changed = true;
            }
        });
        
        interaction_result
    }
    
    /// Find commit at a specific screen position
    fn find_commit_at_position(&self, pos: egui::Pos2, layout: &GraphLayout) -> Option<String> {
        for (commit_id, commit_pos) in &layout.commit_positions {
            let distance = (pos - commit_pos.pos).length();
            if distance <= commit_pos.radius + 5.0 {
                return Some(commit_id.clone());
            }
        }
        None
    }
    
    /// Render the complete graph
    fn render_graph(&self, ui: &mut egui::Ui, layout: &GraphLayout, commits: &[GitCommit], rect: egui::Rect) {
        let painter = ui.painter();
        
        // Draw within available area (clipping handled by egui)
        
        // Draw connection lines first (behind commits)
        for commit_pos in layout.commit_positions.values() {
            for line in &commit_pos.parent_lines {
                self.draw_connection_line(&painter, line);
            }
        }
        
        // Draw merge indicators
        for merge_line in &layout.merge_lines {
            self.draw_merge_indicator(&painter, merge_line);
        }
        
        // Draw commits
        for (commit_id, commit_pos) in &layout.commit_positions {
            self.draw_commit(&painter, commit_pos, commit_id == self.interaction_state.hovered_commit.as_ref().unwrap_or(&String::new()));
        }
        
        // Draw reference labels
        for commit_pos in layout.commit_positions.values() {
            for ref_label in &commit_pos.refs {
                self.draw_ref_label(&painter, commit_pos.pos, ref_label);
            }
        }
        
        // Draw conflict markers
        let conflicts = self.detect_conflicts(layout, commits);
        self.draw_conflict_markers(&painter, &conflicts);
        
        // Draw path highlighting if active
        if let Some(ref path) = self.highlighted_path {
            self.draw_highlighted_path(&painter, path, layout);
        }
    }
    
    /// Draw a connection line between commits
    fn draw_connection_line(&self, painter: &egui::Painter, line: &ConnectionLine) {
        let stroke = egui::Stroke::new(line.thickness, line.color);
        
        if line.control_points.is_empty() {
            // Straight line
            painter.line_segment([line.start, line.end], stroke);
        } else {
            // Curved line using Bezier curve
            let mut path = Vec::new();
            path.push(line.start);
            
            // Add curve points
            for i in 0..=10 {
                let t = i as f32 / 10.0;
                let point = self.cubic_bezier(line.start, line.control_points[0], line.control_points[1], line.end, t);
                path.push(point);
            }
            
            // Draw the curve as connected line segments
            for window in path.windows(2) {
                painter.line_segment([window[0], window[1]], stroke);
            }
        }
    }
    
    /// Calculate point on cubic Bezier curve
    fn cubic_bezier(&self, p0: egui::Pos2, p1: egui::Pos2, p2: egui::Pos2, p3: egui::Pos2, t: f32) -> egui::Pos2 {
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;
        
        let x = uuu * p0.x + 3.0 * uu * t * p1.x + 3.0 * u * tt * p2.x + ttt * p3.x;
        let y = uuu * p0.y + 3.0 * uu * t * p1.y + 3.0 * u * tt * p2.y + ttt * p3.y;
        
        egui::Pos2::new(x, y)
    }
    
    /// Draw a commit circle
    fn draw_commit(&self, painter: &egui::Painter, commit_pos: &CommitPosition, is_hovered: bool) {
        let radius = if is_hovered { commit_pos.radius * 1.3 } else { commit_pos.radius };
        let stroke_width = if is_hovered { 3.0 } else { 2.0 };
        
        // Draw commit circle
        painter.circle(
            commit_pos.pos,
            radius,
            commit_pos.color,
            egui::Stroke::new(stroke_width, egui::Color32::BLACK)
        );
        
        // Draw selection indicator if selected
        if self.interaction_state.selected_commits.contains(&commit_pos.pos.to_string()) {
            painter.circle_stroke(
                commit_pos.pos,
                radius + 3.0,
                egui::Stroke::new(2.0, egui::Color32::YELLOW)
            );
        }
    }
    
    /// Draw merge commit indicator with enhanced visual styles
    fn draw_merge_indicator(&self, painter: &egui::Painter, merge_line: &MergeLine) {
        let center = merge_line.merge_pos;
        
        match merge_line.style {
            MergeStyle::Simple => {
                // Draw sophisticated merge visualization for 2-parent merges
                if merge_line.parent_positions.len() == 2 {
                    let parent1 = merge_line.parent_positions[0];
                    let parent2 = merge_line.parent_positions[1];
                    
                    // Draw main branch line (stronger)
                    let main_stroke = egui::Stroke::new(3.0 * self.zoom_level, merge_line.color);
                    painter.line_segment([center, parent1], main_stroke);
                    
                    // Draw merge branch line with curve
                    let merge_color = if parent2.x != parent1.x {
                        // Different lanes - use contrasting color
                        self.get_contrasting_color(merge_line.color)
                    } else {
                        merge_line.color
                    };
                    
                    let merge_stroke = egui::Stroke::new(2.0 * self.zoom_level, merge_color);
                    self.draw_curved_merge_line(painter, center, parent2, merge_stroke);
                    
                    // Draw merge indicator diamond
                    self.draw_merge_diamond(painter, center, merge_line.color);
                } else {
                    // Fallback for other parent counts
                    for parent_pos in &merge_line.parent_positions {
                        let stroke = egui::Stroke::new(3.0 * self.zoom_level, merge_line.color);
                        painter.line_segment([center, *parent_pos], stroke);
                    }
                }
            }
            MergeStyle::Octopus => {
                // Draw enhanced octopus merge with color coding
                let arm_count = merge_line.parent_positions.len();
                let base_thickness = 2.0 * self.zoom_level;
                
                // Draw a central indicator
                painter.circle_filled(center, 6.0 * self.zoom_level, merge_line.color);
                painter.circle_stroke(center, 6.0 * self.zoom_level, 
                    egui::Stroke::new(2.0, egui::Color32::BLACK));
                
                // Draw each arm with distinct colors and styles
                for (i, parent_pos) in merge_line.parent_positions.iter().enumerate() {
                    let arm_color = self.branch_colors[i % self.branch_colors.len()];
                    let thickness = base_thickness * (1.0 + (i as f32 * 0.2));
                    let stroke = egui::Stroke::new(thickness, arm_color);
                    
                    // Draw curved line for octopus arms
                    self.draw_curved_merge_line(painter, center, *parent_pos, stroke);
                    
                    // Add small indicator at parent end
                    painter.circle_filled(*parent_pos, 3.0 * self.zoom_level, arm_color);
                }
                
                // Add octopus label
                let text_pos = center + egui::Vec2::new(10.0 * self.zoom_level, -15.0 * self.zoom_level);
                painter.text(
                    text_pos,
                    egui::Align2::LEFT_CENTER,
                    format!("{}-way", arm_count),
                    egui::FontId::proportional(8.0 * self.zoom_level),
                    egui::Color32::BLACK
                );
            }
            MergeStyle::Highlighted => {
                // Draw enhanced highlighted merge with glow effect
                for (i, parent_pos) in merge_line.parent_positions.iter().enumerate() {
                    // Draw glow effect (multiple overlapping lines)
                    for glow_radius in [6.0, 4.0, 2.0] {
                        let glow_alpha = (255.0 / glow_radius) as u8;
                        let glow_color = egui::Color32::from_rgba_unmultiplied(255, 255, 0, glow_alpha);
                        let glow_stroke = egui::Stroke::new(glow_radius * self.zoom_level, glow_color);
                        painter.line_segment([center, *parent_pos], glow_stroke);
                    }
                    
                    // Draw main highlighted line
                    let highlight_stroke = egui::Stroke::new(4.0 * self.zoom_level, egui::Color32::YELLOW);
                    painter.line_segment([center, *parent_pos], highlight_stroke);
                }
                
                // Draw highlighted merge indicator
                painter.circle_filled(center, 8.0 * self.zoom_level, egui::Color32::YELLOW);
                painter.circle_stroke(center, 8.0 * self.zoom_level, 
                    egui::Stroke::new(2.0, egui::Color32::BLACK));
            }
        }
    }
    
    /// Draw a curved line for merge connections
    fn draw_curved_merge_line(&self, painter: &egui::Painter, start: egui::Pos2, end: egui::Pos2, stroke: egui::Stroke) {
        let distance = (end - start).length();
        
        if distance < 20.0 * self.zoom_level {
            // Too close for curve, draw straight line
            painter.line_segment([start, end], stroke);
            return;
        }
        
        // Calculate curve based on distance and direction
        let mid_point = (start + end.to_vec2()) / 2.0;
        let perpendicular = egui::Vec2::new(-(end.y - start.y), end.x - start.x).normalized();
        let curve_offset = (distance * 0.2).min(30.0 * self.zoom_level);
        let control_point = mid_point + perpendicular * curve_offset;
        
        // Draw curve as series of line segments
        let segments = 8;
        for i in 0..segments {
            let t1 = i as f32 / segments as f32;
            let t2 = (i + 1) as f32 / segments as f32;
            
            let p1 = self.quadratic_bezier(start, control_point, end, t1);
            let p2 = self.quadratic_bezier(start, control_point, end, t2);
            
            painter.line_segment([p1, p2], stroke);
        }
    }
    
    /// Calculate point on quadratic Bezier curve
    fn quadratic_bezier(&self, p0: egui::Pos2, p1: egui::Pos2, p2: egui::Pos2, t: f32) -> egui::Pos2 {
        let u = 1.0 - t;
        let x = u * u * p0.x + 2.0 * u * t * p1.x + t * t * p2.x;
        let y = u * u * p0.y + 2.0 * u * t * p1.y + t * t * p2.y;
        egui::Pos2::new(x, y)
    }
    
    /// Draw a diamond shape for merge indicators
    fn draw_merge_diamond(&self, painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
        let size = 4.0 * self.zoom_level;
        let points = [
            center + egui::Vec2::new(0.0, -size),    // Top
            center + egui::Vec2::new(size, 0.0),     // Right
            center + egui::Vec2::new(0.0, size),     // Bottom
            center + egui::Vec2::new(-size, 0.0),    // Left
        ];
        
        // Fill diamond
        painter.add(egui::Shape::convex_polygon(
            points.to_vec(),
            color,
            egui::Stroke::new(1.0, egui::Color32::BLACK)
        ));
    }
    
    /// Get a contrasting color for better visibility
    fn get_contrasting_color(&self, base_color: egui::Color32) -> egui::Color32 {
        // Simple contrast algorithm - if color is bright, return darker variant
        let [r, g, b, a] = base_color.to_array();
        let brightness = (r as f32 * 0.299 + g as f32 * 0.587 + b as f32 * 0.114) / 255.0;
        
        if brightness > 0.5 {
            // Darken the color
            egui::Color32::from_rgba_unmultiplied(
                (r as f32 * 0.6) as u8,
                (g as f32 * 0.6) as u8,
                (b as f32 * 0.6) as u8,
                a
            )
        } else {
            // Brighten the color
            egui::Color32::from_rgba_unmultiplied(
                ((r as f32 * 1.4).min(255.0)) as u8,
                ((g as f32 * 1.4).min(255.0)) as u8,
                ((b as f32 * 1.4).min(255.0)) as u8,
                a
            )
        }
    }
    
    /// Draw reference label
    fn draw_ref_label(&self, painter: &egui::Painter, commit_pos: egui::Pos2, ref_label: &RefLabel) {
        let label_pos = commit_pos + ref_label.offset;
        let text_size = 10.0 * self.zoom_level;
        
        // Measure text to create background rectangle
        let font = egui::FontId::monospace(text_size);
        let text_galley = painter.layout_no_wrap(ref_label.text.clone(), font.clone(), ref_label.text_color);
        
        let background_rect = egui::Rect::from_min_size(
            label_pos,
            text_galley.size() + egui::Vec2::splat(4.0)
        );
        
        // Draw background
        painter.rect_filled(background_rect, 2.0, ref_label.background);
        painter.rect_stroke(background_rect, egui::CornerRadius::same(2), egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Outside);
        
        // Draw text
        painter.galley(label_pos + egui::Vec2::splat(2.0), text_galley, ref_label.text_color);
    }
    
    /// Draw highlighted path through commit history
    fn draw_highlighted_path(&self, painter: &egui::Painter, path: &[String], layout: &GraphLayout) {
        for window in path.windows(2) {
            if let (Some(from_pos), Some(to_pos)) = (
                layout.commit_positions.get(&window[0]),
                layout.commit_positions.get(&window[1])
            ) {
                let stroke = egui::Stroke::new(4.0, egui::Color32::YELLOW);
                painter.line_segment([from_pos.pos, to_pos.pos], stroke);
            }
        }
    }
    
    /// Render hover tooltip with commit information
    fn render_hover_tooltip(&self, ui: &mut egui::Ui, commits: &[GitCommit]) {
        if let Some(ref hovered_commit_id) = self.interaction_state.hovered_commit {
            if let Some(commit) = commits.iter().find(|c| &c.id == hovered_commit_id) {
                egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), ui.id().with("tooltip"), |ui: &mut egui::Ui| {
                    ui.vertical(|ui| {
                        ui.label(format!("Commit: {}", &commit.short_id));
                        ui.label(format!("Author: {}", commit.author.name));
                        ui.label(format!("Date: {}", commit.author.when.format("%Y-%m-%d %H:%M")));
                        ui.separator();
                        ui.label(&commit.message);
                    });
                });
            }
        }
    }
    
    /// Generate cache key for layout caching
    fn generate_cache_key(&self, commits: &[GitCommit]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        commits.len().hash(&mut hasher);
        self.zoom_level.to_bits().hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }
    
    /// Set highlighted path for visual emphasis
    pub fn set_highlighted_path(&mut self, path: Option<Vec<String>>) {
        self.highlighted_path = path;
    }
    
    /// Get currently hovered commit
    pub fn get_hovered_commit(&self) -> Option<&String> {
        self.interaction_state.hovered_commit.as_ref()
    }
    
    /// Set zoom level
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom_level = zoom.clamp(0.5, 3.0);
        self.layout_cache.clear(); // Invalidate cache
    }
    
    /// Reset pan and zoom
    pub fn reset_view(&mut self) {
        self.zoom_level = 1.0;
        self.pan_offset = egui::Vec2::ZERO;
        self.layout_cache.clear();
    }
    
    /// Set filtered branches for highlighting
    pub fn set_filtered_branches(&mut self, branches: Vec<String>) {
        self.filtered_branches = branches;
        self.layout_cache.clear(); // Invalidate cache when filter changes
    }
    
    /// Toggle between showing all branches or only filtered ones
    pub fn set_show_filtered_only(&mut self, show_filtered_only: bool) {
        self.show_filtered_only = show_filtered_only;
        self.layout_cache.clear();
    }
    
    /// Add a branch to the filter
    pub fn add_branch_filter(&mut self, branch_name: String) {
        if !self.filtered_branches.contains(&branch_name) {
            self.filtered_branches.push(branch_name);
            self.layout_cache.clear();
        }
    }
    
    /// Remove a branch from the filter
    pub fn remove_branch_filter(&mut self, branch_name: &str) {
        self.filtered_branches.retain(|b| b != branch_name);
        self.layout_cache.clear();
    }
    
    /// Clear all branch filters
    pub fn clear_branch_filters(&mut self) {
        self.filtered_branches.clear();
        self.layout_cache.clear();
    }
    
    /// Classify branch type based on name and commit patterns
    fn classify_branch_type(&self, branch_name: &str, commits: &[GitCommit]) -> BranchType {
        let name_lower = branch_name.to_lowercase();
        
        // Main branch detection
        if name_lower == "main" || name_lower == "master" || name_lower == "trunk" {
            return BranchType::Main;
        }
        
        // Release branch detection
        if name_lower.starts_with("release/") || name_lower.starts_with("rel/") || 
           name_lower.contains("release") {
            return BranchType::Release;
        }
        
        // Hotfix branch detection
        if name_lower.starts_with("hotfix/") || name_lower.starts_with("fix/") ||
           name_lower.contains("hotfix") {
            return BranchType::Hotfix;
        }
        
        // Feature branch detection
        if name_lower.starts_with("feature/") || name_lower.starts_with("feat/") ||
           name_lower.starts_with("dev/") || name_lower.contains("feature") {
            return BranchType::Feature;
        }
        
        BranchType::Unknown
    }
    
    /// Calculate branch priority based on type and activity
    fn calculate_branch_priority(&self, branch_type: &BranchType, commits: &[GitCommit], branch_name: &str) -> BranchPriority {
        match branch_type {
            BranchType::Main => BranchPriority::VeryHigh,
            BranchType::Release => BranchPriority::High,
            BranchType::Hotfix => BranchPriority::High,
            BranchType::Feature => {
                // Check activity level for feature branches
                let recent_commits = commits.iter()
                    .take(50) // Look at recent commits
                    .filter(|c| c.message.contains(branch_name) || 
                               c.author.name.contains(branch_name))
                    .count();
                
                if recent_commits > 5 {
                    BranchPriority::High
                } else if recent_commits > 2 {
                    BranchPriority::Medium
                } else {
                    BranchPriority::Low
                }
            }
            BranchType::Unknown => BranchPriority::Medium,
        }
    }
    
    /// Detect potential conflicts and create markers
    fn detect_conflicts(&self, layout: &GraphLayout, commits: &[GitCommit]) -> Vec<ConflictMarker> {
        let mut conflicts = Vec::new();
        
        // Detect merge conflicts (commits with multiple parents that could conflict)
        for commit in commits {
            if commit.parent_ids.len() > 1 {
                if let Some(commit_pos) = layout.commit_positions.get(&commit.id) {
                    // Check if this merge has potential conflicts
                    let conflict_severity = self.assess_merge_conflict_risk(commit, commits);
                    
                    if conflict_severity != ConflictSeverity::Low {
                        conflicts.push(ConflictMarker {
                            position: commit_pos.pos,
                            conflict_type: ConflictType::MergeConflict,
                            severity: conflict_severity,
                        });
                    }
                }
            }
        }
        
        // Detect lane collisions (visual conflicts)
        let mut lane_occupancy: std::collections::HashMap<usize, Vec<&CommitPosition>> = 
            std::collections::HashMap::new();
        
        for commit_pos in layout.commit_positions.values() {
            lane_occupancy.entry(commit_pos.lane)
                .or_insert_with(Vec::new)
                .push(commit_pos);
        }
        
        // Check for overcrowded lanes
        for (lane, positions) in lane_occupancy {
            if positions.len() > 20 { // Threshold for overcrowding
                for pos in positions {
                    conflicts.push(ConflictMarker {
                        position: pos.pos,
                        conflict_type: ConflictType::LaneCollision,
                        severity: ConflictSeverity::Medium,
                    });
                }
            }
        }
        
        conflicts
    }
    
    /// Assess the risk level of merge conflicts
    fn assess_merge_conflict_risk(&self, merge_commit: &GitCommit, commits: &[GitCommit]) -> ConflictSeverity {
        let parent_count = merge_commit.parent_ids.len();
        
        // Octopus merges are inherently riskier
        if parent_count > 2 {
            return ConflictSeverity::High;
        }
        
        // Check temporal distance between parents
        let mut parent_ages = Vec::new();
        for parent_id in &merge_commit.parent_ids {
            if let Some(parent_commit) = commits.iter().find(|c| &c.id == parent_id) {
                let age_diff = merge_commit.author.when.timestamp() - parent_commit.author.when.timestamp();
                parent_ages.push(age_diff);
            }
        }
        
        if let Some(&max_age_diff) = parent_ages.iter().max() {
            if max_age_diff > 86400 * 7 { // More than a week apart
                ConflictSeverity::Medium
            } else if max_age_diff > 86400 * 30 { // More than a month apart
                ConflictSeverity::High
            } else {
                ConflictSeverity::Low
            }
        } else {
            ConflictSeverity::Low
        }
    }
    
    /// Get color for branch type with visual distinction
    fn get_branch_type_color(&self, branch_type: &BranchType, base_color: egui::Color32) -> egui::Color32 {
        match branch_type {
            BranchType::Main => {
                // Main branches get stronger, more saturated colors
                let [r, g, b, a] = base_color.to_array();
                egui::Color32::from_rgba_unmultiplied(
                    ((r as f32 * 1.2).min(255.0)) as u8,
                    ((g as f32 * 1.2).min(255.0)) as u8,
                    ((b as f32 * 1.2).min(255.0)) as u8,
                    a
                )
            }
            BranchType::Release => {
                // Release branches get golden tint
                let [r, g, b, a] = base_color.to_array();
                egui::Color32::from_rgba_unmultiplied(
                    ((r as f32 * 1.1).min(255.0)) as u8,
                    ((g as f32 * 1.1).min(255.0)) as u8,
                    (b as f32 * 0.8) as u8,
                    a
                )
            }
            BranchType::Hotfix => {
                // Hotfix branches get reddish tint
                let [r, g, b, a] = base_color.to_array();
                egui::Color32::from_rgba_unmultiplied(
                    ((r as f32 * 1.3).min(255.0)) as u8,
                    (g as f32 * 0.8) as u8,
                    (b as f32 * 0.8) as u8,
                    a
                )
            }
            BranchType::Feature => {
                // Feature branches keep base color
                base_color
            }
            BranchType::Unknown => {
                // Unknown branches get muted colors
                let [r, g, b, a] = base_color.to_array();
                egui::Color32::from_rgba_unmultiplied(
                    (r as f32 * 0.8) as u8,
                    (g as f32 * 0.8) as u8,
                    (b as f32 * 0.8) as u8,
                    a
                )
            }
        }
    }
    
    /// Draw conflict markers on the graph
    fn draw_conflict_markers(&self, painter: &egui::Painter, conflicts: &[ConflictMarker]) {
        for conflict in conflicts {
            let color = match conflict.severity {
                ConflictSeverity::Low => egui::Color32::from_rgb(255, 255, 0),      // Yellow
                ConflictSeverity::Medium => egui::Color32::from_rgb(255, 165, 0),   // Orange
                ConflictSeverity::High => egui::Color32::from_rgb(255, 69, 0),      // Red-Orange
                ConflictSeverity::Critical => egui::Color32::from_rgb(255, 0, 0),   // Red
            };
            
            let size = match conflict.severity {
                ConflictSeverity::Low => 2.0 * self.zoom_level,
                ConflictSeverity::Medium => 3.0 * self.zoom_level,
                ConflictSeverity::High => 4.0 * self.zoom_level,
                ConflictSeverity::Critical => 5.0 * self.zoom_level,
            };
            
            match conflict.conflict_type {
                ConflictType::MergeConflict => {
                    // Draw warning triangle
                    let points = [
                        conflict.position + egui::Vec2::new(0.0, -size),
                        conflict.position + egui::Vec2::new(size * 0.866, size * 0.5),
                        conflict.position + egui::Vec2::new(-size * 0.866, size * 0.5),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        points.to_vec(),
                        color,
                        egui::Stroke::new(1.0, egui::Color32::BLACK)
                    ));
                }
                ConflictType::LaneCollision => {
                    // Draw warning circle
                    painter.circle_filled(conflict.position, size, color);
                    painter.circle_stroke(conflict.position, size, 
                        egui::Stroke::new(1.0, egui::Color32::BLACK));
                }
                ConflictType::FastForward => {
                    // Draw forward arrow
                    self.draw_forward_arrow(painter, conflict.position, color, size);
                }
                ConflictType::Divergence => {
                    // Draw divergence indicator
                    self.draw_divergence_indicator(painter, conflict.position, color, size);
                }
            }
        }
    }
    
    /// Draw forward arrow for fast-forward indicators
    fn draw_forward_arrow(&self, painter: &egui::Painter, center: egui::Pos2, color: egui::Color32, size: f32) {
        let points = [
            center + egui::Vec2::new(-size, -size * 0.5),
            center + egui::Vec2::new(size * 0.5, 0.0),
            center + egui::Vec2::new(-size, size * 0.5),
        ];
        painter.add(egui::Shape::convex_polygon(
            points.to_vec(),
            color,
            egui::Stroke::new(1.0, egui::Color32::BLACK)
        ));
    }
    
    /// Draw divergence indicator
    fn draw_divergence_indicator(&self, painter: &egui::Painter, center: egui::Pos2, color: egui::Color32, size: f32) {
        // Draw Y-shaped indicator
        let base = center + egui::Vec2::new(0.0, size);
        let left = center + egui::Vec2::new(-size * 0.7, -size * 0.5);
        let right = center + egui::Vec2::new(size * 0.7, -size * 0.5);
        
        let stroke = egui::Stroke::new(2.0, color);
        painter.line_segment([base, center], stroke);
        painter.line_segment([center, left], stroke);
        painter.line_segment([center, right], stroke);
    }
    
    /// Auto-highlight path from a commit to its ancestors
    fn auto_highlight_commit_path(&mut self, commit_id: &str, layout: &GraphLayout) {
        let mut path = Vec::new();
        let mut current_id = commit_id.to_string();
        
        // Trace back through parent commits (first parent for linear path)
        for _ in 0..20 { // Limit path length to prevent infinite loops
            path.push(current_id.clone());
            
            // Find commit position to get parent information
            if let Some(commit_pos) = layout.commit_positions.get(&current_id) {
                // For simplicity, follow the first parent line if available
                if let Some(first_parent_line) = commit_pos.parent_lines.first() {
                    // Find the commit at the parent line's end position
                    if let Some((parent_id, _)) = layout.commit_positions.iter()
                        .find(|(_, pos)| (pos.pos - first_parent_line.end).length() < 5.0) {
                        current_id = parent_id.clone();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        self.highlighted_path = if path.len() > 1 { Some(path) } else { None };
    }
    
    /// Trace commit ancestry for comprehensive path highlighting
    fn trace_commit_ancestry(&mut self, commit_id: &str, layout: &GraphLayout) {
        let mut ancestry_path = Vec::new();
        let mut to_visit = vec![commit_id.to_string()];
        let mut visited = std::collections::HashSet::new();
        
        // Breadth-first search through commit ancestry
        while let Some(current_id) = to_visit.pop() {
            if visited.contains(&current_id) || ancestry_path.len() >= 50 {
                continue;
            }
            
            visited.insert(current_id.clone());
            ancestry_path.push(current_id.clone());
            
            // Add all parents to the visit queue
            if let Some(commit_pos) = layout.commit_positions.get(&current_id) {
                for parent_line in &commit_pos.parent_lines {
                    // Find parent commit by position
                    if let Some((parent_id, _)) = layout.commit_positions.iter()
                        .find(|(_, pos)| (pos.pos - parent_line.end).length() < 5.0) {
                        if !visited.contains(parent_id) {
                            to_visit.push(parent_id.clone());
                        }
                    }
                }
            }
        }
        
        self.highlighted_path = if ancestry_path.len() > 1 { Some(ancestry_path) } else { None };
    }
    
    /// Fit graph to window bounds
    fn fit_to_window(&mut self, layout: &GraphLayout, window_rect: egui::Rect) {
        if layout.commit_positions.is_empty() {
            return;
        }
        
        // Find bounds of all commit positions
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        
        for commit_pos in layout.commit_positions.values() {
            min_x = min_x.min(commit_pos.pos.x);
            max_x = max_x.max(commit_pos.pos.x);
            min_y = min_y.min(commit_pos.pos.y);
            max_y = max_y.max(commit_pos.pos.y);
        }
        
        let content_width = max_x - min_x;
        let content_height = max_y - min_y;
        
        if content_width > 0.0 && content_height > 0.0 {
            // Calculate zoom to fit content with padding
            let padding = 50.0;
            let zoom_x = (window_rect.width() - padding * 2.0) / content_width;
            let zoom_y = (window_rect.height() - padding * 2.0) / content_height;
            self.zoom_level = zoom_x.min(zoom_y).clamp(0.1, 5.0);
            
            // Center the content
            let content_center_x = (min_x + max_x) / 2.0;
            let content_center_y = (min_y + max_y) / 2.0;
            let window_center = window_rect.center();
            
            self.pan_offset = window_center.to_vec2() - egui::Vec2::new(content_center_x, content_center_y) * self.zoom_level;
        }
        
        self.layout_cache.clear();
    }
    
    /// Render interaction hints and help text
    fn render_interaction_hints(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        // Only show hints if graph is not empty
        if self.interaction_state.selected_commits.is_empty() && self.highlighted_path.is_none() {
            let hint_text = "💡 Click: Select • Double-click: Trace ancestry • Right-click: Context menu\n\
                           📍 Drag: Pan • Scroll: Zoom • R: Reset view • F: Fit to window • Esc: Clear selection";
            
            let text_pos = rect.left_bottom() + egui::Vec2::new(10.0, -30.0);
            let background_rect = egui::Rect::from_min_size(
                text_pos - egui::Vec2::new(5.0, 20.0),
                egui::Vec2::new(400.0, 25.0)
            );
            
            // Semi-transparent background
            ui.painter().rect_filled(
                background_rect,
                3.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180)
            );
            
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_BOTTOM,
                hint_text,
                egui::FontId::proportional(10.0),
                egui::Color32::WHITE
            );
        }
        
        // Show zoom level in top-right corner
        let zoom_text = format!("🔍 {:.0}%", self.zoom_level * 100.0);
        let zoom_pos = rect.right_top() + egui::Vec2::new(-80.0, 10.0);
        
        ui.painter().text(
            zoom_pos,
            egui::Align2::RIGHT_TOP,
            zoom_text,
            egui::FontId::monospace(11.0),
            egui::Color32::from_rgb(200, 200, 200)
        );
        
        // Show selection count if any
        if !self.interaction_state.selected_commits.is_empty() {
            let selection_text = format!("📌 {} selected", self.interaction_state.selected_commits.len());
            let selection_pos = rect.right_top() + egui::Vec2::new(-80.0, 30.0);
            
            ui.painter().text(
                selection_pos,
                egui::Align2::RIGHT_TOP,
                selection_text,
                egui::FontId::proportional(10.0),
                egui::Color32::YELLOW
            );
        }
    }
    
    /// Get list of selected commits
    pub fn get_selected_commits(&self) -> &[String] {
        &self.interaction_state.selected_commits
    }
    
    /// Clear current selection
    pub fn clear_selection(&mut self) {
        self.interaction_state.selected_commits.clear();
        self.highlighted_path = None;
    }
    
    /// Set external selection (from other UI components)
    pub fn set_selection(&mut self, commit_ids: Vec<String>) {
        self.interaction_state.selected_commits = commit_ids;
        
        // Auto-highlight path for single selection
        if self.interaction_state.selected_commits.len() == 1 {
            // Note: This would need layout access, so we'll set a flag instead
            self.highlighted_path = None; // Will be set by next interaction
        } else {
            self.highlighted_path = None;
        }
    }
    
    /// Check if a commit is currently selected
    pub fn is_commit_selected(&self, commit_id: &str) -> bool {
        self.interaction_state.selected_commits.contains(&commit_id.to_string())
    }
}

impl Default for CommitGraphRenderer {
    fn default() -> Self {
        Self::new()
    }
}