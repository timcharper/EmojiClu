use gtk::{
    prelude::{GridExt, WidgetExt},
    Grid, Label,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventObserver, SubscriptionId},
    model::{GameActionEvent, GameStateEvent, Solution},
};

use super::{layout::SPACING_LARGE, puzzle_cell_ui::PuzzleCellUI, ResourceSet};

pub struct PuzzleGridUI {
    n_rows: usize,
    n_cols: usize,
    pub grid: gtk::Grid,
    pub pause_label: gtk::Label,
    pub cells: Vec<Vec<PuzzleCellUI>>,
    game_action_emitter: EventEmitter<GameActionEvent>,
    resources: Rc<ResourceSet>,
    subscription_id: Option<SubscriptionId>,
    game_state_observer: EventObserver<GameStateEvent>,
}

impl Destroyable for PuzzleGridUI {
    fn destroy(&mut self) {
        // Unparent all widgets
        self.grid.unparent();
        self.pause_label.unparent();
        if let Some(subscription_id) = self.subscription_id.take() {
            self.game_state_observer.unsubscribe(subscription_id);
        }
    }
}

impl PuzzleGridUI {
    pub fn new(
        game_action_emitter: EventEmitter<GameActionEvent>,
        game_state_observer: EventObserver<GameStateEvent>,
        resources: &Rc<ResourceSet>,
        n_rows: usize,
        n_cols: usize,
    ) -> Rc<RefCell<Self>> {
        let grid = Grid::new();
        grid.set_row_spacing(SPACING_LARGE as u32);
        grid.set_column_spacing(SPACING_LARGE as u32);
        grid.set_hexpand(false);
        grid.set_vexpand(false);
        grid.set_css_classes(&["puzzle-grid"]);
        let pause_label = Label::new(Some("Game is Paused"));
        pause_label.set_css_classes(&["pause-label"]);
        pause_label.set_visible(false);
        pause_label.set_halign(gtk::Align::Center);
        pause_label.set_valign(gtk::Align::Center);
        pause_label.set_vexpand(true);
        pause_label.set_hexpand(true);

        let puzzle_grid_ui = Rc::new(RefCell::new(Self {
            grid,
            pause_label,
            cells: vec![vec![]],
            n_rows: 0,
            n_cols: 0,
            game_action_emitter: game_action_emitter,
            resources: Rc::clone(resources),
            subscription_id: None,
            game_state_observer: game_state_observer.clone(),
        }));

        // Initialize grid
        {
            let mut grid = puzzle_grid_ui.borrow_mut();
            grid.maybe_resize(n_rows, n_cols);
        }

        // Connect observer
        Self::connect_observer(puzzle_grid_ui.clone(), game_state_observer);

        puzzle_grid_ui
    }

    fn connect_observer(
        puzzle_grid_ui: Rc<RefCell<Self>>,
        game_state_observer: EventObserver<GameStateEvent>,
    ) {
        let puzzle_grid_ui_moved = puzzle_grid_ui.clone();
        let subscription_id = game_state_observer.subscribe(move |event| match event {
            GameStateEvent::GridUpdate(board) => {
                let mut puzzle_grid_ui = puzzle_grid_ui_moved.borrow_mut();
                puzzle_grid_ui.maybe_resize(board.solution.n_rows, board.solution.n_rows);
                for row in 0..board.solution.n_rows {
                    for col in 0..board.solution.n_variants {
                        if let Some(cell) =
                            puzzle_grid_ui.cells.get(row).and_then(|row| row.get(col))
                        {
                            // If there's a solution, show it
                            if let Some(tile) = board.selected[row][col] {
                                cell.set_solution(Some(&tile));
                            } else {
                                // Otherwise show candidates
                                cell.set_solution(None);
                                let correct_tile = board.solution.get(row, col);
                                for (i, variant) in board.get_variants().iter().enumerate() {
                                    if let Some(candidate) = board.get_candidate(row, col, *variant)
                                    {
                                        cell.set_candidate(i, Some(&candidate));
                                        cell.highlight_candidate(i, None);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            GameStateEvent::PuzzleVisibilityChanged(visible) => {
                if *visible {
                    puzzle_grid_ui_moved.borrow().show();
                } else {
                    puzzle_grid_ui_moved.borrow().hide();
                }
            }
            GameStateEvent::CellHintHighlight { cell, variant } => {
                puzzle_grid_ui_moved.borrow().highlight_candidate(
                    cell.0,
                    cell.1,
                    *variant,
                    Duration::from_secs(4),
                );
            }
            _ => {}
        });
        puzzle_grid_ui.borrow_mut().subscription_id = Some(subscription_id);
    }

    pub fn maybe_resize(&mut self, n_rows: usize, n_cols: usize) {
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
                    self.game_action_emitter.clone(),
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
