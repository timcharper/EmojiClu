use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::events::EventEmitter;
use crate::model::{Candidate, CandidateState, GameEvent, Tile};
use crate::ui::layout::{CANDIDATE_IMG_SIZE, CELL_SPACING, FRAME_MARGIN};
use glib::timeout_add_local_once;
use gtk::prelude::*;
use gtk::{Frame, Grid, Image, Overlay};
use log::trace;

use super::layout::CELL_SIZE;
use super::ResourceSet;

pub struct PuzzleCellUI {
    pub frame: Frame,
    pub candidates_grid: Grid,                 // 2x3 grid for candidates
    pub solution_image: Image,                 // Large image for selected solution
    pub candidate_images: Vec<Image>,          // Small images for candidates
    pub _candidate_overlays: Vec<Rc<Overlay>>, // Overlays for highlighting; need to hold references for GTK
    pub highlight_frames: Vec<Rc<Frame>>,      // Frames for showing highlights
    pub resources: Rc<ResourceSet>,
    pub row: usize,
    pub col: usize,
    pub game_event_emitter: EventEmitter<GameEvent>,
    pub _variants: RangeInclusive<char>,
    pub n_variants: usize,
}

impl PuzzleCellUI {
    pub fn calc_cell_width(n_variants: usize) -> i32 {
        CELL_SIZE * (PuzzleCellUI::calc_ncols(n_variants) as i32) + CELL_SPACING * 2
    }

    pub fn calc_cell_height() -> i32 {
        CELL_SIZE * 2 + CELL_SPACING
    }

    fn calc_ncols(n_variants: usize) -> usize {
        (n_variants + 1) / 2
    }

    pub fn new(
        resources: &Rc<ResourceSet>,
        game_event_emitter: EventEmitter<GameEvent>,
        variants: RangeInclusive<char>,
        row: usize,
        col: usize,
    ) -> Self {
        let frame = Frame::new(None);
        let n_variants = variants.clone().count();
        frame.set_margin_start(FRAME_MARGIN);
        frame.set_margin_end(FRAME_MARGIN);
        frame.set_margin_top(FRAME_MARGIN);
        frame.set_margin_bottom(FRAME_MARGIN);
        frame.set_css_classes(&["cell-frame"]);
        frame.set_hexpand(false);
        frame.set_vexpand(false);
        let cell_width = PuzzleCellUI::calc_cell_width(n_variants);
        let cell_height = PuzzleCellUI::calc_cell_height();
        frame.set_size_request(cell_width, cell_height);

        let candidates_grid = Grid::new();
        candidates_grid.set_row_spacing(CELL_SPACING as u32);
        candidates_grid.set_column_spacing(CELL_SPACING as u32);
        candidates_grid.set_halign(gtk::Align::Center);
        candidates_grid.set_valign(gtk::Align::Center);
        candidates_grid.set_hexpand(false);
        candidates_grid.set_vexpand(false);

        let solution_image = Image::new();
        solution_image.set_visible(false);

        let candidate_images: Vec<Image> = variants
            .clone()
            .map(|_| {
                let img = Image::new();
                img.set_pixel_size(CANDIDATE_IMG_SIZE);
                img
            })
            .collect();

        let candidate_highlight_frames: Vec<Rc<Frame>> = variants
            .clone()
            .map(|_| {
                let frame = Frame::new(None);
                frame.set_css_classes(&["highlight-frame"]);
                Rc::new(frame)
            })
            .collect();

        let candidate_overlays: Vec<Rc<Overlay>> =
            variants.clone().map(|_| Rc::new(Overlay::new())).collect();

        let ncols = PuzzleCellUI::calc_ncols(n_variants); // TODO - handle odd # of columns
        let nrows = 2;
        // Add candidates to grid
        for row in 0..nrows {
            for col in 0..ncols {
                let idx = row * ncols + col;
                if idx >= n_variants {
                    break;
                }
                // Set up overlay with image and highlight frame
                candidate_overlays[idx].set_child(Some(&candidate_images[idx]));
                candidate_overlays[idx].add_overlay(candidate_highlight_frames[idx].as_ref());
                candidates_grid.attach(
                    candidate_overlays[idx].as_ref(),
                    col as i32,
                    row as i32,
                    1,
                    1,
                );
            }
        }

        let overlay = Overlay::new();
        overlay.set_css_classes(&["grid-overlay"]);

        let highlight_frame = Frame::new(None);
        highlight_frame.set_css_classes(&["highlight-frame"]);
        overlay.add_overlay(&highlight_frame);

        // overlay.set_child(Some(&candidates_grid));
        // frame.set_child(Some(&overlay));

        frame.set_child(Some(&candidates_grid));

        let instance = Self {
            _variants: variants,
            frame,
            candidates_grid,
            solution_image,
            candidate_images,
            _candidate_overlays: candidate_overlays,
            highlight_frames: candidate_highlight_frames,
            resources: Rc::clone(resources),
            row,
            col,
            game_event_emitter: game_event_emitter,
            n_variants,
        };
        instance.register_click_handler();
        instance
    }

    fn register_click_handler(&self) {
        let row = self.row;
        let col = self.col;
        let n_variants = self.n_variants;

        // Left click handler
        let gesture_click = gtk::GestureClick::new();
        gesture_click.set_button(1);
        let game_event_emitter = self.game_event_emitter.clone();
        gesture_click.connect_pressed(move |_gesture, _, x, y| {
            let variant = PuzzleCellUI::get_variant_at_position(x, y, n_variants);
            game_event_emitter.emit(&GameEvent::CellClick(row, col, variant));
        });

        // Right click handler
        let gesture_right = gtk::GestureClick::new();
        gesture_right.set_button(3);
        let game_event_emitter = self.game_event_emitter.clone();
        gesture_right.connect_pressed(move |_gesture, _, x, y| {
            let variant = PuzzleCellUI::get_variant_at_position(x, y, n_variants);
            game_event_emitter.emit(&GameEvent::CellRightClick(row, col, variant));
        });

        self.frame.add_controller(gesture_click);
        self.frame.add_controller(gesture_right);
    }

    pub fn highlight_candidate(&self, index: usize, highlight_class: Option<&str>) {
        if let Some(class) = highlight_class {
            self.highlight_frames[index].set_css_classes(&[class]);
            self.highlight_frames[index].set_visible(true);
        } else {
            self.highlight_frames[index].set_visible(false);
        }
    }

    pub fn set_candidate(&self, index: usize, candidate: Option<&Candidate>) {
        if let Some(candidate) = candidate {
            if let Some(pixbuf) = self.resources.get_icon(
                candidate.tile.row as i32,
                candidate.tile.variant as i32 - 'a' as i32,
            ) {
                self.candidate_images[index].set_from_pixbuf(Some(&pixbuf));
                self.candidate_images[index].set_opacity(match candidate.state {
                    CandidateState::Available => 1.0,
                    CandidateState::Eliminated => 0.1,
                });
            }
        }
    }

    pub fn set_solution(&self, tile: Option<&Tile>) {
        // First, remove current child to ensure clean state
        self.frame.set_child(Option::<&gtk::Widget>::None);

        if let Some(tile) = tile {
            if let Some(pixbuf) = self
                .resources
                .get_icon(tile.row as i32, tile.variant as i32 - 'a' as i32)
            {
                // Set up solution image
                self.solution_image.set_from_pixbuf(Some(&pixbuf));
                self.solution_image.set_visible(true);
                self.candidates_grid.set_visible(false);
                // Add solution image as child
                self.frame.set_child(Some(&self.solution_image));
            }
        } else {
            // Reset to candidates view
            self.solution_image.set_visible(false);
            self.candidates_grid.set_visible(true);
            // Add candidates grid as child
            self.frame.set_child(Some(&self.candidates_grid));
        }
    }

    pub fn get_variant_at_position(x: f64, y: f64, n_variants: usize) -> Option<char> {
        // Each candidate is 32x32 pixels, and the grid is centered in a 96x96 cell
        // Calculate offset to center of candidates grid

        let ncols = PuzzleCellUI::calc_ncols(n_variants); // TODO - handle odd # of columns
        let nrows = 2;

        let grid_width = (ncols as f64) * (CANDIDATE_IMG_SIZE as f64) + 2.0 * 2.0; // 3 candidates * 32px + 2 gaps * 2px
        let grid_height = (nrows as f64) * (CANDIDATE_IMG_SIZE as f64) + 1.0 * 2.0; // 2 rows * 32px + 1 gap * 2px
        let grid_x_offset = 2.0;
        let grid_y_offset = 1.0;

        trace!(target: "puzzle_cell_ui", "Click at ({}, {})", x, y);
        trace!(target: "puzzle_cell_ui", "Grid dimensions: {}x{}", grid_width, grid_height);
        trace!(target: "puzzle_cell_ui", "Grid offset: ({}, {})", grid_x_offset, grid_y_offset);

        // Adjust click position relative to grid
        let grid_x = x - grid_x_offset;
        let grid_y = y - grid_y_offset;

        trace!(target: "puzzle_cell_ui", "Adjusted click position: ({}, {})", grid_x, grid_y);

        // Check if click is outside the grid
        if grid_x < 0.0 || grid_y < 0.0 || grid_x >= grid_width || grid_y >= grid_height {
            trace!(target: "puzzle_cell_ui", "Click outside grid bounds");
            return None;
        }

        let col = (grid_x / (CANDIDATE_IMG_SIZE as f64 + 2.0)).floor() as usize; // Add 2px for gap
        let row = (grid_y / (CANDIDATE_IMG_SIZE as f64 + 2.0)).floor() as usize; // Add 2px for gap

        trace!(target: "puzzle_cell_ui", "Calculated grid position: row={}, col={}", row, col);

        if row >= nrows || col >= ncols {
            trace!(target: "puzzle_cell_ui", "Position outside valid range");
            return None;
        }

        // Convert grid position to variant (a-f)
        let variant_index = row * ncols + col;
        if variant_index >= n_variants {
            trace!(target: "puzzle_cell_ui", "Variant index {} out of range", variant_index);
            return None;
        }

        let variant = (b'a' + variant_index as u8) as char;
        trace!(target: "puzzle_cell_ui", "Selected variant: {}", variant);
        Some(variant)
    }

    pub fn highlight_candidate_for(&self, from_secs: std::time::Duration, variant: char) {
        trace!(
            target: "cell_ui",
            "Highlighting candidate: {} in cell ({}, {})",
            variant,
            self.row,
            self.col
        );
        let index = variant as usize - 'a' as usize;
        self.highlight_frames[index].set_css_classes(&["clue-highlight"]);
        self.highlight_frames[index].set_visible(true);
        let highlight_frame = Rc::clone(&self.highlight_frames[index]);
        timeout_add_local_once(from_secs, move || {
            highlight_frame.remove_css_class("clue-highlight");
            highlight_frame.add_css_class("clue-nohighlight");
        });
    }
}

impl Drop for PuzzleCellUI {
    fn drop(&mut self) {
        // Unparent all widgets to ensure proper cleanup
        self.frame.unparent();
    }
}
