use gtk::{
    prelude::{GridExt, WidgetExt},
    Grid,
};
use log::trace;
use std::{cell::RefCell, rc::Rc, time::Duration};

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventObserver, Unsubscriber},
    model::{GameActionEvent, GameStateEvent, GlobalEvent, LayoutConfiguration, Solution},
};

use super::{puzzle_cell_ui::PuzzleCellUI, ResourceSet};

pub struct PuzzleGridUI {
    pub grid: gtk::Grid,
    pub cells: Vec<Vec<Rc<RefCell<PuzzleCellUI>>>>,
    game_action_emitter: EventEmitter<GameActionEvent>,
    resources: Rc<ResourceSet>,
    game_state_subscription_id: Option<Unsubscriber<GameStateEvent>>,
    settings_subscription_id: Option<Unsubscriber<GlobalEvent>>,
    game_state_observer: EventObserver<GameStateEvent>,
    global_event_observer: EventObserver<GlobalEvent>,
    current_layout: LayoutConfiguration,
    n_rows: usize,
    n_variants: usize,
}

impl Destroyable for PuzzleGridUI {
    fn destroy(&mut self) {
        // Unparent all widgets
        self.grid.unparent();
        if let Some(subscription_id) = self.game_state_subscription_id.take() {
            subscription_id.unsubscribe();
        }
        if let Some(subscription_id) = self.settings_subscription_id.take() {
            subscription_id.unsubscribe();
        }
    }
}

impl PuzzleGridUI {
    pub fn new(
        game_action_emitter: EventEmitter<GameActionEvent>,
        game_state_observer: EventObserver<GameStateEvent>,
        global_event_observer: EventObserver<GlobalEvent>,
        resources: Rc<ResourceSet>,
        layout: LayoutConfiguration,
    ) -> Rc<RefCell<Self>> {
        let grid = Grid::new();
        grid.set_css_classes(&["puzzle-grid"]);

        let puzzle_grid_ui = Rc::new(RefCell::new(Self {
            grid,
            cells: vec![],
            game_action_emitter,
            resources,
            game_state_subscription_id: None,
            settings_subscription_id: None,
            game_state_observer: game_state_observer.clone(),
            global_event_observer: global_event_observer.clone(),
            current_layout: layout.clone(),
            n_rows: 0,
            n_variants: 0,
        }));

        // Subscribe to layout changes
        Self::connect_global_observer(puzzle_grid_ui.clone(), global_event_observer);
        Self::connect_game_state_observer(puzzle_grid_ui.clone(), game_state_observer);

        puzzle_grid_ui
            .borrow_mut()
            .set_grid_size(layout.grid.n_rows as usize, layout.grid.n_variants as usize);

        puzzle_grid_ui
    }

    fn update_layout(&mut self, layout: &LayoutConfiguration) {
        self.current_layout = layout.clone();

        // Update grid spacing
        self.grid.set_row_spacing(layout.grid.row_spacing as u32);
        self.grid
            .set_column_spacing(layout.grid.column_spacing as u32);

        self.grid.set_margin_start(layout.grid.outer_margin);
        self.grid.set_margin_end(layout.grid.outer_margin);
        self.grid.set_margin_top(layout.grid.outer_margin);
        self.grid.set_margin_bottom(layout.grid.outer_margin);

        // Propagate to all cells
        for row in &mut self.cells {
            for cell in row {
                cell.borrow_mut().update_layout(&layout.grid);
            }
        }
    }

    fn connect_global_observer(
        puzzle_grid_ui: Rc<RefCell<Self>>,
        global_event_observer: EventObserver<GlobalEvent>,
    ) {
        let puzzle_grid_ui_moved = puzzle_grid_ui.clone();
        let layout_subscription_id = global_event_observer.subscribe(move |event| {
            puzzle_grid_ui_moved.borrow_mut().handle_global_event(event);
        });

        puzzle_grid_ui.borrow_mut().settings_subscription_id = Some(layout_subscription_id);
    }

    fn connect_game_state_observer(
        puzzle_grid_ui: Rc<RefCell<Self>>,
        game_state_observer: EventObserver<GameStateEvent>,
    ) {
        let puzzle_grid_ui_moved = puzzle_grid_ui.clone();
        let subscription_id = game_state_observer.subscribe(move |event| {
            puzzle_grid_ui_moved
                .borrow_mut()
                .handle_game_state_event(event);
        });
        puzzle_grid_ui.borrow_mut().game_state_subscription_id = Some(subscription_id);
    }

    fn handle_global_event(&mut self, event: &GlobalEvent) {
        match event {
            GlobalEvent::LayoutChanged(new_layout) => self.update_layout(new_layout),
            _ => (),
        }
    }

    fn handle_game_state_event(&mut self, event: &GameStateEvent) {
        match event {
            GameStateEvent::GridUpdate(board) => {
                self.set_grid_size(board.solution.n_rows, board.solution.n_variants);
                for row in 0..board.solution.n_rows {
                    for col in 0..board.solution.n_variants {
                        if let Some(cell) = self.cells.get(row).and_then(|row| row.get(col)) {
                            let cell = cell.borrow_mut();
                            // If there's a solution, show it
                            if let Some(tile) = board.selected[row][col] {
                                cell.set_solution(Some(&tile));
                            } else {
                                // Otherwise show candidates
                                cell.set_solution(None);
                                // let correct_tile = board.solution.get(row, col);
                                for variant in board.get_variants().iter() {
                                    if let Some(candidate) = board.get_candidate(row, col, *variant)
                                    {
                                        cell.set_candidate(*variant, Some(&candidate));
                                        cell.highlight_candidate(*variant, None);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            GameStateEvent::CellHintHighlight { cell, variant } => {
                self.highlight_candidate(cell.0, cell.1, *variant);
            }
            _ => {}
        }
    }

    fn set_grid_size(&mut self, n_rows: usize, n_variants: usize) {
        if n_rows == self.n_rows && n_variants == self.n_variants {
            return;
        }
        self.n_rows = n_rows;
        self.n_variants = n_variants;

        trace!(
            target: "puzzle_grid_ui",
            "maybe_resize_grid; n_rows: {}; n_variants: {}",
            n_rows,
            n_variants
        );

        self.cells.iter().for_each(|row| {
            row.iter().for_each(|cell| {
                let mut cell = cell.borrow_mut();
                self.grid.remove(&cell.frame);
                cell.destroy();
            });
        });

        self.cells.clear();
        let variants_range = Solution::variants_range(n_variants);

        for row in 0..n_rows {
            let mut row_cells = vec![];
            for col in 0..n_variants {
                let cell_ui = PuzzleCellUI::new(
                    self.resources.clone(),
                    row,
                    col,
                    self.game_action_emitter.clone(),
                    variants_range.clone(),
                    self.current_layout.grid.clone(),
                );
                self.grid
                    .attach(&cell_ui.borrow().frame, col as i32, row as i32, 1, 1);
                row_cells.push(cell_ui);
            }
            self.cells.push(row_cells);
        }

        // let padding_size_from_css = 3;
        // let total_width = total_cell_width + total_col_spacing + padding_size_from_css;
        // let total_height = total_cell_height + total_row_spacing + padding_size_from_css;
        // self.grid.set_size_request(total_width, total_height);
        // self.grid.set_hexpand(false);
        // self.grid.set_vexpand(false);
    }

    pub(crate) fn highlight_candidate(&self, row: usize, column: usize, variant: char) {
        self.cells[row][column]
            .borrow()
            .hint_highlight_candidate_for(Duration::from_secs(4), variant);
    }
}
