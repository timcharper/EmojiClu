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
        Clue, GameActionEvent, GameStateEvent, GlobalEvent, InputEvent, LayoutConfiguration,
        Solution,
    },
};

use super::{puzzle_cell_ui::PuzzleCellUI, ImageSet};

pub struct PuzzleGridUI {
    pub grid: Grid,
    cells: Vec<Vec<Rc<RefCell<PuzzleCellUI>>>>,
    input_event_emitter: EventEmitter<InputEvent>,
    resources: Rc<ImageSet>,
    game_state_subscription_id: Option<Unsubscriber<GameStateEvent>>,
    settings_subscription_id: Option<Unsubscriber<GlobalEvent>>,
    current_layout: LayoutConfiguration,
    n_rows: usize,
    n_variants: usize,
    current_xray_enabled: bool,
    current_focused_clue: Option<Clue>,
    completed_clues: HashSet<Clue>,
    current_clue_hint: Option<Clue>,
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
        input_event_emitter: EventEmitter<InputEvent>,
        game_state_observer: EventObserver<GameStateEvent>,
        global_event_observer: EventObserver<GlobalEvent>,
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
            game_state_subscription_id: None,
            settings_subscription_id: None,
            current_layout: layout.clone(),
            n_rows: 0,
            n_variants: 0,
            current_xray_enabled: settings.clue_xray_enabled,
            current_focused_clue: None,
            completed_clues: HashSet::new(),
            current_clue_hint: None,
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
            GlobalEvent::ImagesOptimized(new_image_set) => {
                self.resources = new_image_set.clone();
                // propagate image set to all cells
                for row in &mut self.cells {
                    for cell in row {
                        cell.borrow_mut().set_image_set(self.resources.clone());
                    }
                }
            }
            GlobalEvent::SettingsChanged(settings) => {
                if settings.clue_xray_enabled != self.current_xray_enabled {
                    self.set_clue_xray_enabled(settings.clue_xray_enabled);
                }
            }
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
                            let mut cell = cell.borrow_mut();
                            // If there's a solution, show it
                            if let Some(tile) = board.get_selection(row, col) {
                                cell.set_solution(Some(&tile));
                            } else {
                                // Otherwise show candidates
                                cell.set_solution(None);
                                cell.set_candidates(
                                    board
                                        .get_variants()
                                        .iter()
                                        .map(|v| board.get_candidate(row, col, *v))
                                        .collect::<Vec<_>>(),
                                );
                            }
                        }
                    }
                }
                self.completed_clues = board.completed_clues.clone();
            }
            GameStateEvent::CellHintHighlight { cell, variant } => {
                self.highlight_candidate(cell.0, cell.1, *variant);
            }
            GameStateEvent::ClueSelected(clue_selection) => {
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
            GameStateEvent::ClueHintHighlight { clue_with_grouping } => {
                self.current_clue_hint = Some(clue_with_grouping.clue.clone());
                self.sync_xray();
            }
            _ => {}
        }
    }

    fn set_current_clue(&mut self, clue: &Option<Clue>) {
        self.current_focused_clue = clue.clone();
        if self.current_focused_clue != self.current_clue_hint {
            // clear the hint state we move on
            self.current_clue_hint = None;
        }
        self.sync_xray();
    }

    fn sync_xray(&self) {
        let current_focused_clue_completed = self
            .current_focused_clue
            .as_ref()
            .map(|clue| self.completed_clues.contains(clue))
            .unwrap_or(false);

        let selected_clue_is_hint = self.current_clue_hint == self.current_focused_clue;

        let xray_clue = if (selected_clue_is_hint || self.current_xray_enabled)
            && !current_focused_clue_completed
        {
            self.current_focused_clue.clone()
        } else {
            None
        };
        for row in &self.cells {
            for cell in row {
                cell.borrow_mut().set_clue_xray(&xray_clue);
            }
        }
    }

    fn set_clue_xray_enabled(&mut self, enabled: bool) {
        self.current_xray_enabled = enabled;
        self.sync_xray();
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
