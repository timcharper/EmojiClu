use gtk4::{
    prelude::{GridExt, WidgetExt},
    Grid,
};
use log::trace;
use std::{cell::RefCell, collections::HashSet, rc::Rc, time::Duration};

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventObserver, Unsubscriber},
    game::settings::Settings,
    model::{
        ClueAddress, ClueWithAddress, Difficulty, GameEngineEvent, InputEvent, LayoutConfiguration,
        LayoutManagerEvent, Solution,
    },
};

use super::{puzzle_cell_ui::PuzzleCellUI, ImageSet};

pub struct PuzzleGridUI {
    pub grid: Grid,
    cells: Vec<Vec<Rc<RefCell<PuzzleCellUI>>>>,
    input_event_emitter: EventEmitter<InputEvent>,
    resources: Rc<ImageSet>,
    game_engine_event_subscription_id: Option<Unsubscriber<GameEngineEvent>>,
    layout_subscription_id: Option<Unsubscriber<LayoutManagerEvent>>,
    current_layout: LayoutConfiguration,
    n_rows: usize,
    n_variants: usize,
    current_spotlight_enabled: bool,
    current_focused_clue: Option<ClueWithAddress>,
    completed_clues: HashSet<ClueAddress>,
    current_clue_hint: Option<ClueWithAddress>,
    current_difficulty: Difficulty,
    settings: Settings,
}

impl Destroyable for PuzzleGridUI {
    fn destroy(&mut self) {
        // Unparent all widgets
        self.grid.unparent();
        if let Some(subscription_id) = self.game_engine_event_subscription_id.take() {
            subscription_id.unsubscribe();
        }
        if let Some(subscription_id) = self.layout_subscription_id.take() {
            subscription_id.unsubscribe();
        }
    }
}

impl PuzzleGridUI {
    pub fn new(
        input_event_emitter: EventEmitter<InputEvent>,
        game_engine_event_observer: EventObserver<GameEngineEvent>,
        layout_manager_event_observer: EventObserver<LayoutManagerEvent>,
        resources: Rc<ImageSet>,
        layout: LayoutConfiguration,
        settings: &Settings,
    ) -> Rc<RefCell<Self>> {
        let grid = Grid::new();
        grid.set_css_classes(&["puzzle-grid"]);

        let puzzle_grid_ui = Rc::new(RefCell::new(Self {
            grid,
            cells: vec![],
            input_event_emitter,
            resources,
            game_engine_event_subscription_id: None,
            layout_subscription_id: None,
            current_layout: layout.clone(),
            n_rows: 0,
            n_variants: 0,
            current_spotlight_enabled: settings.clue_spotlight_enabled,
            current_focused_clue: None,
            completed_clues: HashSet::new(),
            current_clue_hint: None,
            current_difficulty: settings.difficulty,
            settings: settings.clone(),
        }));

        // Subscribe to layout changes
        Self::connect_layout_observer(puzzle_grid_ui.clone(), layout_manager_event_observer);
        Self::connect_game_engine_event_observer(
            puzzle_grid_ui.clone(),
            game_engine_event_observer,
        );

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

    fn connect_layout_observer(
        puzzle_grid_ui: Rc<RefCell<Self>>,
        layout_manager_event_observer: EventObserver<LayoutManagerEvent>,
    ) {
        let puzzle_grid_ui_moved = puzzle_grid_ui.clone();
        let layout_subscription_id = layout_manager_event_observer.subscribe(move |event| {
            puzzle_grid_ui_moved.borrow_mut().handle_layout_event(event);
        });

        puzzle_grid_ui.borrow_mut().layout_subscription_id = Some(layout_subscription_id);
    }

    fn connect_game_engine_event_observer(
        puzzle_grid_ui: Rc<RefCell<Self>>,
        game_engine_event_observer: EventObserver<GameEngineEvent>,
    ) {
        let puzzle_grid_ui_moved = puzzle_grid_ui.clone();
        let subscription_id = game_engine_event_observer.subscribe(move |event| {
            puzzle_grid_ui_moved
                .borrow_mut()
                .handle_game_engine_event(event);
        });
        puzzle_grid_ui
            .borrow_mut()
            .game_engine_event_subscription_id = Some(subscription_id);
    }

    fn handle_layout_event(&mut self, event: &LayoutManagerEvent) {
        match event {
            LayoutManagerEvent::LayoutChanged(new_layout) => self.update_layout(new_layout),
            LayoutManagerEvent::ImagesOptimized(new_image_set) => {
                self.resources = new_image_set.clone();
                // propagate image set to all cells
                for row in &mut self.cells {
                    for cell in row {
                        cell.borrow_mut().set_image_set(self.resources.clone());
                    }
                }
            }
            _ => (),
        }
    }

    fn handle_game_engine_event(&mut self, event: &GameEngineEvent) {
        match event {
            GameEngineEvent::GameBoardUpdated { board, .. } => {
                self.current_difficulty = board.solution.difficulty;
                self.set_grid_size(board.solution.n_rows, board.solution.n_variants);
                for row in 0..board.solution.n_rows {
                    for col in 0..board.solution.n_variants {
                        if let Some(cell) = self.cells.get(row).and_then(|row| row.get(col)) {
                            let mut cell = cell.borrow_mut();
                            // If there's a solution, show it
                            if let Some(tile) = board.get_selection(row, col) {
                                cell.set_solution(Some(&tile));
                            } else {
                                // Otherwise show candidates
                                cell.set_solution(None);
                                cell.set_candidates(
                                    board
                                        .solution
                                        .variants
                                        .iter()
                                        .map(|v| board.get_candidate(row, col, *v))
                                        .collect::<Vec<_>>(),
                                );
                            }
                        }
                    }
                }
                self.completed_clues = board.completed_clues().clone();
                self.sync_clue_spotlight_enabled();
            }
            GameEngineEvent::HintSuggested(deduction) => {
                self.highlight_candidate(
                    deduction.tile_assertion.tile.row,
                    deduction.column,
                    deduction.tile_assertion.tile.variant,
                );
            }
            GameEngineEvent::ClueSelected(clue_selection) => {
                if let Some(clue_selection) = clue_selection {
                    if clue_selection.is_focused {
                        self.set_current_clue(&Some(clue_selection.clue.clone()));
                    } else {
                        self.set_current_clue(&None);
                    }
                } else {
                    self.set_current_clue(&None);
                }
            }
            GameEngineEvent::ClueHintHighlighted(addressed_clue) => {
                self.current_clue_hint = addressed_clue.clone();
                self.sync_spotlight();
            }

            _ => {}
        }
    }

    fn set_current_clue(&mut self, clue: &Option<ClueWithAddress>) {
        self.current_focused_clue = clue.clone();
        if self.current_focused_clue != self.current_clue_hint {
            // clear the hint state we move on
            self.current_clue_hint = None;
        }
        self.sync_spotlight();
    }

    fn sync_spotlight(&self) {
        let current_focused_clue_completed = self
            .current_focused_clue
            .as_ref()
            .map(|clue| self.completed_clues.contains(&clue.address()))
            .unwrap_or(false);

        let selected_clue_is_hint = self.current_clue_hint == self.current_focused_clue;

        let spotlight_clue = if (selected_clue_is_hint || self.current_spotlight_enabled)
            && !current_focused_clue_completed
        {
            self.current_focused_clue.clone()
        } else {
            None
        };
        for row in &self.cells {
            for cell in row {
                cell.borrow_mut().set_clue_spotlight(&spotlight_clue);
            }
        }
    }

    fn sync_clue_spotlight_enabled(&mut self) {
        self.current_spotlight_enabled =
            self.current_difficulty == Difficulty::Tutorial || self.settings.clue_spotlight_enabled;
        self.sync_spotlight();
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
                    self.input_event_emitter.clone(),
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
