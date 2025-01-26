use gtk::{
    prelude::{GridExt, WidgetExt},
    ApplicationWindow, Grid, Label,
};
use std::rc::Rc;

use crate::{
    events::EventEmitter,
    model::{GameEvent, Solution},
};

use super::{layout::SPACING_LARGE, puzzle_cell_ui::PuzzleCellUI, ResourceSet};

pub struct PuzzleGridUI {
    n_rows: usize,
    n_cols: usize,
    pub grid: gtk::Grid,
    pub pause_label: gtk::Label,
    pub cells: Vec<Vec<PuzzleCellUI>>,
    game_event_emitter: EventEmitter<GameEvent>,
    resources: Rc<ResourceSet>,
}

impl PuzzleGridUI {
    pub fn new(
        game_event_emitter: EventEmitter<GameEvent>,
        resources: &Rc<ResourceSet>,
        n_rows: usize,
        n_cols: usize,
    ) -> Self {
        let grid = Grid::new();
        let pause_label = Label::new(Some("Game is Paused"));
        pause_label.set_css_classes(&["pause-label"]);
        pause_label.set_visible(false);
        pause_label.set_halign(gtk::Align::Center);
        pause_label.set_valign(gtk::Align::Center);
        pause_label.set_vexpand(true);
        pause_label.set_hexpand(true);

        let mut puzzle_grid = Self {
            grid,
            pause_label,
            cells: vec![vec![]],
            n_rows: 0,
            n_cols: 0,
            game_event_emitter: game_event_emitter,
            resources: Rc::clone(resources),
        };
        puzzle_grid.grid.set_row_spacing(SPACING_LARGE as u32);
        puzzle_grid.grid.set_column_spacing(SPACING_LARGE as u32);
        puzzle_grid.grid.set_hexpand(false);
        puzzle_grid.grid.set_vexpand(false);
        puzzle_grid.grid.set_css_classes(&["puzzle-grid"]);
        puzzle_grid.resize(n_rows, n_cols);
        puzzle_grid
    }

    pub fn resize(&mut self, n_rows: usize, n_cols: usize) {
        if n_rows == self.n_rows && n_cols == self.n_cols {
            return;
        }

        self.cells.iter().for_each(|row| {
            row.iter().for_each(|cell| {
                self.grid.remove(&cell.frame);
            });
        });

        self.cells.clear();
        let variants_range = Solution::variants_range(n_cols);

        for row in 0..n_rows {
            let mut row_cells = vec![];
            for col in 0..n_cols {
                let cell_ui = PuzzleCellUI::new(
                    &self.resources,
                    self.game_event_emitter.clone(),
                    variants_range.clone(),
                    row,
                    col,
                );
                self.grid
                    .attach(&cell_ui.frame, col as i32, row as i32, 1, 1);
                row_cells.push(cell_ui);
            }
            self.cells.push(row_cells);
        }
        let cell_width = PuzzleCellUI::calc_cell_width(n_cols);
        let cell_height = PuzzleCellUI::calc_cell_height();
        let total_cell_width = cell_width * n_cols as i32;
        let total_cell_height = cell_height * n_rows as i32;
        let total_col_spacing = (SPACING_LARGE * (n_cols - 1) as i32).min(0);
        let total_row_spacing = (SPACING_LARGE * (n_rows - 1) as i32).min(0);

        let padding_size_from_css = 3;
        let total_width = total_cell_width + total_col_spacing + padding_size_from_css;
        let total_height = total_cell_height + total_row_spacing + padding_size_from_css;
        self.grid.set_size_request(total_width, total_height);
        self.grid.set_hexpand(false);
        self.grid.set_vexpand(false);
        self.n_rows = n_rows;
        self.n_cols = n_cols;
    }

    pub(crate) fn highlight_candidate(
        &self,
        row: usize,
        column: usize,
        variant: char,
        duration: std::time::Duration,
    ) {
        self.cells[row][column].highlight_candidate_for(duration, variant);
    }

    pub fn show(&self) {
        self.grid.set_visible(true);
        self.pause_label.set_visible(false);
    }

    pub fn hide(&self) {
        self.grid.set_visible(false);
        self.pause_label.set_visible(true);
    }
}

impl Drop for PuzzleGridUI {
    fn drop(&mut self) {
        // Unparent the grid and label
        self.grid.unparent();
        self.pause_label.unparent();
    }
}
