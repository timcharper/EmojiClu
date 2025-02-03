use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::destroyable::Destroyable;
use crate::events::EventEmitter;
use crate::model::{Candidate, CandidateState, Clue, GameActionEvent, GridSizing, Tile};
use glib::timeout_add_local_once;
use gtk4::{prelude::*, GestureClick, Widget};
use gtk4::{Frame, Grid, Image, Overlay};
use log::{trace, warn};

use super::ResourceSet;

pub struct PuzzleCellUI {
    pub frame: Frame,
    pub candidates_grid: Grid,                 // 2x3 grid for candidates
    pub solution_image: Image,                 // Large image for selected solution
    pub solution_overlay: Rc<Overlay>,         // Overlay for solution image
    pub solution_highlight_frame: Rc<Frame>,   // Frame for solution highlighting
    pub candidate_images: Vec<Image>,          // Small images for candidates
    pub _candidate_overlays: Vec<Rc<Overlay>>, // Overlays for highlighting; need to hold references for GTK
    pub candidate_highlight_frames: Vec<Rc<Frame>>, // Frames for showing highlights
    pub resources: Rc<ResourceSet>,
    pub row: usize,
    pub col: usize,
    pub game_action_emitter: EventEmitter<GameActionEvent>,
    pub _variants: RangeInclusive<char>,
    pub n_variants: usize,
    current_layout: GridSizing,
    gesture_click: Option<GestureClick>,
    gesture_right: Option<GestureClick>,
    available_tiles: HashSet<Tile>,
    selected_tile: Option<Tile>,
    clue_selection: Option<Clue>,
}

impl PuzzleCellUI {
    fn grid_dimensions(n_variants: usize, idx: usize) -> (usize, usize) {
        let n_cols = (n_variants + 1) / 2;
        let row = idx / n_cols;
        let col = idx % n_cols;
        (row, col)
    }

    pub fn new(
        resources: Rc<ResourceSet>,
        row: usize,
        col: usize,
        game_action_emitter: EventEmitter<GameActionEvent>,
        variants: RangeInclusive<char>,
        layout: GridSizing,
    ) -> Rc<RefCell<Self>> {
        let frame = Frame::new(None);
        frame.set_css_classes(&["puzzle-cell-frame"]);

        let candidates_grid = Grid::new();
        candidates_grid.set_halign(gtk4::Align::Center);
        candidates_grid.set_valign(gtk4::Align::Center);
        candidates_grid.set_hexpand(false);
        candidates_grid.set_vexpand(false);

        let solution_image = Image::new();
        solution_image.set_visible(false);

        let solution_highlight_frame = Rc::new(
            Frame::builder()
                .name("solution-highlight-frame")
                .css_classes(["highlight-frame"])
                .build(),
        );

        let solution_overlay = Rc::new(Overlay::new());
        solution_overlay.set_child(Some(&solution_image));
        solution_overlay.add_overlay(solution_highlight_frame.upcast_ref::<Widget>());
        solution_overlay.set_visible(false);

        let n_variants = variants.clone().count();
        let candidate_images: Vec<Image> = variants
            .clone()
            .map(|_| {
                let img = Image::new();
                img
            })
            .collect();

        let candidate_highlight_frames: Vec<Rc<Frame>> = variants
            .clone()
            .map(|variant| {
                let frame = Frame::builder()
                    .name(&format!("candidate-highlight-frame-{}", variant))
                    .css_classes(["highlight-frame"])
                    .build();
                Rc::new(frame)
            })
            .collect();

        let candidate_overlays: Vec<Rc<Overlay>> =
            variants.clone().map(|_| Rc::new(Overlay::new())).collect();

        // Set up grid of candidate overlays
        for (idx, overlay) in candidate_overlays.iter().enumerate() {
            let (grid_row, grid_col) = PuzzleCellUI::grid_dimensions(n_variants, idx);

            overlay.set_child(Some(&candidate_images[idx]));
            overlay.add_overlay(candidate_highlight_frames[idx].upcast_ref::<Widget>());

            candidates_grid.attach(overlay.as_ref(), grid_col as i32, grid_row as i32, 1, 1);
        }

        // Create root overlay that will contain either candidates_grid or solution_overlay
        let root_overlay = Overlay::new();
        root_overlay.set_child(Some(&candidates_grid));

        frame.set_child(Some(&root_overlay));

        let cell_ui = Self {
            frame,
            candidates_grid,
            solution_image,
            solution_overlay,
            solution_highlight_frame,
            candidate_images,
            _candidate_overlays: candidate_overlays,
            candidate_highlight_frames,
            resources,
            row,
            col,
            game_action_emitter,
            _variants: variants.clone(),
            n_variants,
            current_layout: layout,
            gesture_click: None,
            gesture_right: None,
            available_tiles: HashSet::new(),
            selected_tile: None,
            clue_selection: None,
        };
        cell_ui.apply_layout();

        let cell_ui = Rc::new(RefCell::new(cell_ui));

        PuzzleCellUI::register_click_handler(cell_ui.clone());
        cell_ui
    }

    /// Dimm the puzzle cell if candidates not in clue
    pub fn set_clue_xray(&mut self, clue_selection: &Option<Clue>) {
        self.clue_selection = clue_selection.clone();
        self.sync_clue_xray();
    }

    fn sync_clue_xray(&self) {
        // clear css on all candidate cells and solution frame
        self.frame.remove_css_class("clue-xray-positive");
        self.frame.remove_css_class("clue-xray-negative");
        for highlight_frame in &self.candidate_highlight_frames {
            highlight_frame.remove_css_class("clue-1");
            highlight_frame.remove_css_class("clue-2");
            highlight_frame.remove_css_class("clue-3");
        }
        self.solution_highlight_frame.remove_css_class("clue-1");
        self.solution_highlight_frame.remove_css_class("clue-2");
        self.solution_highlight_frame.remove_css_class("clue-3");

        match &self.clue_selection {
            Some(clue) => {
                let mut match_count = 0;

                // Handle solution tile if present
                if let Some(selected_tile) = &self.selected_tile {
                    for (idx, assertion) in clue.assertions.iter().enumerate() {
                        if assertion.tile == *selected_tile {
                            match_count += 1;
                            self.solution_highlight_frame
                                .add_css_class(&format!("clue-{}", idx + 1));
                        }
                    }
                } else {
                    // Handle candidate tiles
                    for (idx, assertion) in clue.assertions.iter().enumerate() {
                        let tile_is_avail = if assertion.tile.row != self.row {
                            false
                        } else {
                            self.available_tiles.contains(&assertion.tile)
                        };
                        let variant_idx = assertion.tile.variant as usize - 'a' as usize;
                        if tile_is_avail {
                            match_count += 1;
                            self.candidate_highlight_frames[variant_idx]
                                .add_css_class(&format!("clue-{}", idx + 1));
                        }
                    }
                }

                if match_count > 0 {
                    self.frame.add_css_class("clue-xray-positive");
                } else {
                    self.frame.add_css_class("clue-xray-negative");
                }
            }
            None => {}
        }
    }

    pub fn apply_layout(&self) {
        // Update frame size
        self.frame.set_size_request(
            self.current_layout.cell.dimensions.width,
            self.current_layout.cell.dimensions.height,
        );

        // Update solution image size
        self.solution_image
            .set_pixel_size(self.current_layout.cell.solution_image.width);

        // Update candidate image sizes
        for img in &self.candidate_images {
            img.set_pixel_size(self.current_layout.cell.candidate_image.width);
        }

        // Update grid spacing
        self.candidates_grid
            .set_row_spacing(self.current_layout.cell.candidate_spacing as u32);
        self.candidates_grid
            .set_column_spacing(self.current_layout.cell.candidate_spacing as u32);
    }

    pub fn update_layout(&mut self, layout: &GridSizing) {
        self.current_layout = layout.clone();
        self.apply_layout();
    }

    fn register_click_handler(cell_ui: Rc<RefCell<Self>>) {
        let cell_ui_borrowed = cell_ui.borrow();
        let row = cell_ui_borrowed.row;
        let col = cell_ui_borrowed.col;
        let game_action_emitter = cell_ui_borrowed.game_action_emitter.clone();
        drop(cell_ui_borrowed);

        // Left click handler

        let gesture_click = gtk4::GestureClick::new();
        {
            let cell_ui = Rc::downgrade(&cell_ui);
            let game_action_emitter = game_action_emitter.clone();
            gesture_click.set_button(1);
            gesture_click.connect_pressed(move |_gesture, _, x, y| {
                if let Some(cell_ui) = cell_ui.upgrade() {
                    let variant = cell_ui.borrow_mut().get_variant_at_position(x, y);
                    game_action_emitter.emit(GameActionEvent::CellClick(row, col, variant));
                } else {
                    warn!(target: "puzzle_cell_ui", "Stale handler called!");
                }
            });
        }

        // Right click handler
        let gesture_right = gtk4::GestureClick::new();
        gesture_right.set_button(3);
        {
            let game_action_emitter = game_action_emitter.clone();
            let cell_ui = Rc::downgrade(&cell_ui);
            gesture_right.connect_pressed(move |_gesture, _, x, y| {
                if let Some(cell_ui) = cell_ui.upgrade() {
                    let variant = cell_ui.borrow_mut().get_variant_at_position(x, y);
                    game_action_emitter.emit(GameActionEvent::CellRightClick(row, col, variant));
                } else {
                    warn!(target: "puzzle_cell_ui", "Stale handler called!");
                }
            });
        }
        let mut cell_ui_borrowed = cell_ui.borrow_mut();
        let frame: &Frame = &cell_ui_borrowed.frame;
        frame.add_controller(gesture_click.clone());
        frame.add_controller(gesture_right.clone());
        cell_ui_borrowed.gesture_click = Some(gesture_click);
        cell_ui_borrowed.gesture_right = Some(gesture_right);
    }

    pub fn highlight_candidate(&self, index: char, highlight_class: Option<&str>) {
        let index = index as usize - 'a' as usize;
        if let Some(class) = highlight_class {
            self.candidate_highlight_frames[index].set_css_classes(&[class]);
        } else {
        }
    }

    pub fn set_candidate(&mut self, variant: char, candidate: Option<&Candidate>) {
        let variant_idx = variant as usize - 'a' as usize;
        if let Some(candidate) = candidate {
            if candidate.state == CandidateState::Available {
                self.available_tiles.insert(candidate.tile);
            } else {
                self.available_tiles.remove(&candidate.tile);
            }
            if let Some(pixbuf) = self
                .resources
                .get_icon(candidate.tile.row as i32, variant_idx as i32)
            {
                self.candidate_images[variant_idx].set_from_pixbuf(Some(&pixbuf));
                self.candidate_images[variant_idx].set_opacity(match candidate.state {
                    CandidateState::Available => 1.0,
                    CandidateState::Eliminated => 0.1,
                });
            }
        }
        self.sync_clue_xray();
    }

    pub fn set_solution(&mut self, tile: Option<&Tile>) {
        // First, remove current child to ensure clean state
        self.frame.set_child(Option::<&Widget>::None);

        if let Some(tile) = tile {
            if let Some(pixbuf) = self
                .resources
                .get_icon(tile.row as i32, tile.variant as i32 - 'a' as i32)
            {
                // Set up solution image
                self.solution_image.set_from_pixbuf(Some(&pixbuf));
                self.solution_image.set_visible(true);
                self.solution_overlay.set_visible(true);
                self.candidates_grid.set_visible(false);
                // Add solution overlay as child
                self.frame.set_child(Some(self.solution_overlay.as_ref()));
                self.selected_tile = Some(tile.clone());
            }
        } else {
            self.selected_tile = None;
            // Reset to candidates view
            self.solution_image.set_visible(false);
            self.solution_overlay.set_visible(false);
            self.candidates_grid.set_visible(true);
            // Add candidates grid as child
            self.frame.set_child(Some(&self.candidates_grid));
        }
        self.sync_clue_xray();
    }

    pub fn get_variant_at_position(&self, x: f64, y: f64) -> Option<char> {
        let ncols = (self.n_variants + 1) / 2;
        let nrows = 2;

        let grid_width = self.current_layout.total_dimensions.width as f64;
        let grid_height = self.current_layout.total_dimensions.height as f64;
        let candidate_image = &self.current_layout.cell.candidate_image;

        // unclear what these were
        let grid_x_offset = 2.0;
        let grid_y_offset = 1.0;

        trace!(target: "puzzle_cell_ui", "Click at ({}, {})", x, y);
        trace!(target: "puzzle_cell_ui", "Grid dimensions: {}x{}", grid_width, grid_height);
        trace!(target: "puzzle_cell_ui", "Grid offset: ({}, {})", grid_x_offset, grid_y_offset);

        // Adjust click position relative to grid
        let grid_x = x - grid_x_offset;
        let grid_y = y - grid_y_offset;

        trace!(target: "puzzle_cell_ui", "Adjusted click position: ({}, {})", grid_x, grid_y);

        // Check if click is outside the cell grid
        if grid_x < 0.0
            || grid_y < 0.0
            || grid_x >= self.current_layout.cell.dimensions.width as f64
            || grid_y >= self.current_layout.cell.dimensions.height as f64
        {
            trace!(target: "puzzle_cell_ui", "Click outside grid bounds");
            return None;
        }

        let col = (grid_x
            / (candidate_image.width as f64 + self.current_layout.cell.candidate_spacing as f64))
            .floor() as usize; // Add 2px for gap
        let row = (grid_y
            / (candidate_image.height as f64 + self.current_layout.cell.candidate_spacing as f64))
            .floor() as usize; // Add 2px for gap

        trace!(target: "puzzle_cell_ui", "Calculated grid position: row={}, col={}", row, col);

        if row >= nrows || col >= ncols {
            trace!(target: "puzzle_cell_ui", "Position outside valid range");
            return None;
        }

        // Convert grid position to variant (a-f)
        let variant_index = row * ncols + col;
        if variant_index >= self.n_variants {
            trace!(target: "puzzle_cell_ui", "Variant index {} out of range", variant_index);
            return None;
        }

        let variant = (b'a' + variant_index as u8) as char;
        trace!(target: "puzzle_cell_ui", "Selected variant: {}", variant);
        Some(variant)
    }

    pub fn hint_highlight_candidate_for(&self, from_secs: std::time::Duration, variant: char) {
        trace!(
            target: "cell_ui",
            "Highlighting candidate: {} in cell ({}, {})",
            variant,
            self.row,
            self.col
        );
        let index = variant as usize - 'a' as usize;
        self.candidate_highlight_frames[index].set_css_classes(&["clue-highlight"]);
        self.candidate_highlight_frames[index].set_visible(true);
        let highlight_frame = Rc::clone(&self.candidate_highlight_frames[index]);
        timeout_add_local_once(from_secs, move || {
            highlight_frame.remove_css_class("clue-highlight");
            highlight_frame.add_css_class("clue-nohighlight");
        });
    }
}

impl Drop for PuzzleCellUI {
    fn drop(&mut self) {
        // Unparent all widgets to ensure proper cleanup
        trace!(target: "puzzle_cell_ui", "Dropping cell UI {}, {}", self.col, self.row);
        self.frame.unparent();
    }
}

impl Destroyable for PuzzleCellUI {
    fn destroy(&mut self) {
        trace!(target: "puzzle_cell_ui", "Destroying cell UI {}, {}", self.col, self.row);
        if let Some(gesture_click) = self.gesture_click.take() {
            self.frame.remove_controller(&gesture_click);
        }
        if let Some(gesture_right) = self.gesture_right.take() {
            self.frame.remove_controller(&gesture_right);
        }
    }
}
