use gtk::{
    prelude::{GestureSingleExt, GridExt, WidgetExt},
    ApplicationWindow, Grid,
};
use std::{rc::Rc, time::Duration};

use crate::game::game_event::GameEvent;
use crate::game::ClueSet;
use crate::model::ClueOrientation;
use crate::ui::clue_ui::ClueUI;
use crate::ui::layout::calc_clue_set_size;
use crate::ui::ResourceSet;
use crate::{
    game::clue_generator::{MAX_HORIZ_CLUES, MAX_VERT_CLUES},
    model::ClueWithGrouping,
};

// Create horizontal clue cells (3 tiles wide for each clue, in 2 columns)
const CLUES_PER_COLUMN: usize = (MAX_HORIZ_CLUES + 1) / 3; // Round up to handle odd numbers

pub struct ClueSetUI {
    pub horizontal_grid: Grid,
    pub vertical_grid: Grid,
    horizontal_clue_uis: Vec<ClueUI>,
    vertical_clue_uis: Vec<ClueUI>,
    window: Rc<ApplicationWindow>,
    resources: Rc<ResourceSet>,
}

// Parent widget for both horizontal clues and vertical clues
impl ClueSetUI {
    pub fn new(window: &Rc<ApplicationWindow>, resources: &Rc<ResourceSet>) -> Self {
        let horizontal_clues_grid = Grid::new();
        horizontal_clues_grid.set_row_spacing(0);
        horizontal_clues_grid.set_column_spacing(10);
        horizontal_clues_grid.set_margin_start(10);
        horizontal_clues_grid.set_margin_end(10);
        horizontal_clues_grid.set_hexpand(true);
        horizontal_clues_grid.set_vexpand(true);
        horizontal_clues_grid.set_css_classes(&["horizontal-clues"]);

        // Create vertical clues area (bottom)
        let vertical_clues_grid = Grid::new();
        vertical_clues_grid.set_row_spacing(0);
        vertical_clues_grid.set_column_spacing(0);
        vertical_clues_grid.set_margin_top(10);
        vertical_clues_grid.set_margin_bottom(10);
        vertical_clues_grid.set_hexpand(true);
        vertical_clues_grid.set_vexpand(true);
        vertical_clues_grid.set_css_classes(&["vertical-clues"]);

        let mut clue_set = Self {
            horizontal_grid: horizontal_clues_grid,
            vertical_grid: vertical_clues_grid,
            horizontal_clue_uis: Vec::with_capacity(MAX_HORIZ_CLUES),
            vertical_clue_uis: Vec::with_capacity(MAX_VERT_CLUES),
            window: Rc::clone(window),
            resources: Rc::clone(resources),
        };

        clue_set.setup_clue_sets();
        clue_set
    }

    fn setup_clue_sets(&mut self) {
        for row in 0..MAX_HORIZ_CLUES {
            let grid_col = row / CLUES_PER_COLUMN;
            let grid_row = row % CLUES_PER_COLUMN;

            let clue_set = ClueUI::new(Rc::clone(&self.resources), ClueOrientation::Horizontal);
            self.horizontal_grid
                .attach(&clue_set.frame, grid_col as i32, grid_row as i32, 1, 1);
            self.horizontal_clue_uis.push(clue_set);
        }

        // Create vertical clue cells (3 tiles high for each clue)
        for col in 0..MAX_VERT_CLUES {
            let clue_set = ClueUI::new(Rc::clone(&self.resources), ClueOrientation::Vertical);
            self.vertical_grid
                .attach(&clue_set.frame, col as i32, 0, 1, 1);
            self.vertical_clue_uis.push(clue_set);
        }

        self.wire_clue_handlers();
    }

    fn wire_clue_handlers(&self) {
        // Wire up horizontal clue handlers
        for (clue_idx, clue_set) in self.horizontal_clue_uis.iter().enumerate() {
            let window_ref = Rc::clone(&self.window);
            let gesture_right = gtk::GestureClick::new();
            gesture_right.set_button(3);
            gesture_right.connect_pressed(move |_gesture, _, _, _| {
                GameEvent::dispatch_event(&window_ref, GameEvent::HorizontalClueClick(clue_idx));
            });
            clue_set.frame.add_controller(gesture_right);
        }

        // Wire up vertical clue handlers
        for (clue_idx, clue_set) in self.vertical_clue_uis.iter().enumerate() {
            let window_ref = Rc::clone(&self.window);
            let gesture_right = gtk::GestureClick::new();
            gesture_right.set_button(3);
            gesture_right.connect_pressed(move |_gesture, _, _, _| {
                GameEvent::dispatch_event(&window_ref, GameEvent::VerticalClueClick(clue_idx));
            });
            clue_set.frame.add_controller(gesture_right);
        }
    }

    pub fn highlight_clue(
        &self,
        orientation: ClueOrientation,
        clue_idx: usize,
        duration: Duration,
    ) {
        match orientation {
            ClueOrientation::Horizontal => {
                self.horizontal_clue_uis[clue_idx].highlight_for(duration);
            }
            ClueOrientation::Vertical => {
                self.vertical_clue_uis[clue_idx].highlight_for(duration);
            }
        }
    }

    pub(crate) fn hide(&self) {
        self.horizontal_grid.set_visible(false);
        self.vertical_grid.set_visible(false);
    }

    pub(crate) fn show(&self) {
        self.horizontal_grid.set_visible(true);
        self.vertical_grid.set_visible(true);
    }

    pub(crate) fn set_clues(&self, clue_set: &ClueSet) {
        let mut previous_clue: Option<&ClueWithGrouping> = None;
        for (idx, clue_ui) in self.horizontal_clue_uis.iter().enumerate() {
            let clue = clue_set.horizontal_clues().get(idx);
            let is_new_group = match (clue, previous_clue) {
                (Some(clue), Some(previous_clue)) => clue.group != previous_clue.group,
                _ => false,
            };
            clue_ui.set_clue(clue.map(|c| &c.clue), is_new_group);

            previous_clue = clue;
        }
        for (idx, clue_ui) in self.vertical_clue_uis.iter().enumerate() {
            let clue = clue_set.vertical_clues().get(idx);
            let is_new_group = match (clue, previous_clue) {
                (Some(clue), Some(previous_clue)) => clue.group != previous_clue.group,
                _ => false,
            };
            clue_ui.set_clue(clue.map(|c| &c.clue), is_new_group);
            previous_clue = clue;
        }
        let n_horiz_clues = clue_set.horizontal_clues().len();
        let n_horiz_cols = (n_horiz_clues + 1) / CLUES_PER_COLUMN;
        let min_width = calc_clue_set_size(3) * (n_horiz_cols as i32) + 5; // Two columns of 3-cell clues plus spacing
        self.horizontal_grid.set_size_request(min_width, -1);
    }

    pub(crate) fn set_horiz_completion(&self, idx: usize, is_completed: bool) {
        self.horizontal_clue_uis[idx].set_completed(is_completed);
    }

    pub(crate) fn set_vert_completion(&self, idx: usize, is_completed: bool) {
        self.vertical_clue_uis[idx].set_completed(is_completed);
    }
}

impl Drop for ClueSetUI {
    fn drop(&mut self) {
        self.horizontal_grid.unparent();
        self.vertical_grid.unparent();
    }
}
