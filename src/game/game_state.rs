use gtk::prelude::WidgetExt;
use log::trace;
use std::cell::RefCell;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use super::clue_generator::ClueGeneratorResult;
use super::solver::{deduce_hidden_pairs, perform_evaluation_step, EvaluationStepResult};
use super::stats_manager::GameStats;
use super::{deduce_clue, generate_clues};
use crate::events::{EventEmitter, EventObserver};
use crate::model::{
    CandidateState, ClueSet, ClueWithGrouping, Deduction, Difficulty, GameBoard, GameEvent,
    Solution, TimerState,
};
use crate::ui::clue_set_ui::ClueSetUI;
use crate::ui::game_info_ui::GameInfoUI;
use crate::ui::puzzle_grid_ui::PuzzleGridUI;
use crate::ui::ResourceSet;
use std::rc::Rc;

const HINT_LEVEL_MAX: u8 = 1;

struct DeductionResult {
    deductions: Vec<Deduction>,
    clue: Option<ClueWithGrouping>,
}
impl std::fmt::Debug for DeductionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeductionResult: {{deductions: {:?}, clue: {:?}}}",
            self.deductions, self.clue
        )
    }
}

#[derive(Debug)]
struct HintStatus {
    history_index: usize,
    hint_level: u8,
}

pub struct GameState {
    pub clue_set: Rc<ClueSet>,
    pub current_board: Rc<GameBoard>,
    pub history: Vec<Rc<GameBoard>>,
    pub puzzle_grid_ui: PuzzleGridUI,
    pub clue_set_ui: ClueSetUI,
    pub solution: Rc<Solution>,
    pub debug_mode: bool,
    pub history_index: usize,
    pub submit_button: Rc<gtk::Button>,
    pub hints_used: u32,
    pub current_playthrough_id: Uuid,
    hint_status: HintStatus,
    pub undo_button: Rc<gtk::Button>,
    pub redo_button: Rc<gtk::Button>,
    pub game_info: GameInfoUI,
    pub is_paused: bool,
    pub timer_state: TimerState,
}

pub struct GameBoardSet {
    pub clue_set: Rc<ClueSet>,
    pub board: GameBoard,
    pub solution: Rc<Solution>,
    pub debug_mode: bool,
}
impl Default for GameBoardSet {
    fn default() -> Self {
        let solution = Rc::new(Solution::default());
        Self {
            clue_set: Rc::new(ClueSet::new(vec![])),
            board: GameBoard::new(solution.clone()),
            solution: solution,
            debug_mode: false,
        }
    }
}

impl GameState {
    pub fn is_debug_mode() -> bool {
        std::env::var("DEBUG").map(|v| v == "1").unwrap_or(false)
    }

    pub fn seed_from_env() -> Option<u64> {
        std::env::var("SEED")
            .map(|v| v.parse::<u64>().unwrap())
            .ok()
    }

    pub fn new_board_set(n_rows: usize) -> GameBoardSet {
        let difficulty = match n_rows {
            4 => Difficulty::Easy,
            5 => Difficulty::Moderate,
            6 => Difficulty::Hard,
            _ => Difficulty::Veteran,
        };
        let solution = Rc::new(Solution::new(difficulty, GameState::seed_from_env()));
        trace!(target: "game_state", "Generated solution: {:?}", solution);
        let blank_board = GameBoard::new(Rc::clone(&solution));
        let ClueGeneratorResult {
            clues,
            board,
            revealed_tiles: _,
        } = generate_clues(&blank_board);

        let debug_mode = GameState::is_debug_mode();

        GameBoardSet {
            clue_set: Rc::new(ClueSet::new(clues)),
            board,
            solution,
            debug_mode,
        }
    }

    /*
       let game_state_ref = Rc::clone(&game_state);
       action.connect_activate(move |_, variant| {
           if let Some(variant) = variant {
               if let Some(event) = GameEvent::from_variant(variant) {
                   if let Ok(mut state) = game_state_ref.try_borrow_mut() {
                       state.handle_event(event);
                   } else {
                       log::error!("Failed to borrow game state");
                   }
               }
           }
       });
       window.add_action(&action);
    */
    pub fn new(
        submit_button: &Rc<gtk::Button>,
        undo_button: &Rc<gtk::Button>,
        redo_button: &Rc<gtk::Button>,
        resources: &Rc<ResourceSet>,
        game_event_emitter: EventEmitter<GameEvent>,
        game_event_observer: EventObserver<GameEvent>,
    ) -> Rc<RefCell<Self>> {
        let board_set = GameBoardSet::default();
        let puzzle_grid_ui = PuzzleGridUI::new(
            game_event_emitter.clone(),
            &resources,
            board_set.board.solution.n_rows,
            board_set.board.solution.n_variants,
        );
        let clue_set_ui = ClueSetUI::new(game_event_emitter.clone(), resources);
        let game_info = GameInfoUI::new();
        let timer_state = TimerState::default();
        let game_state = Self {
            game_info,
            clue_set: board_set.clue_set.clone(),
            history: vec![Rc::new(board_set.board.clone())],
            current_board: Rc::new(board_set.board),
            puzzle_grid_ui,
            clue_set_ui,
            solution: board_set.solution.clone(),
            debug_mode: board_set.debug_mode,
            history_index: 0,
            submit_button: Rc::clone(submit_button),
            hints_used: 0,
            hint_status: HintStatus {
                history_index: 0,
                hint_level: 0,
            },
            current_playthrough_id: Uuid::new_v4(),
            undo_button: Rc::clone(undo_button),
            redo_button: Rc::clone(redo_button),
            is_paused: false,
            timer_state,
        };
        let refcell = Rc::new(RefCell::new(game_state));
        GameState::wire_subscription(refcell.clone(), game_event_observer.clone());
        refcell
    }

    fn wire_subscription(
        game_state: Rc<RefCell<Self>>,
        game_event_emitter: EventObserver<GameEvent>,
    ) {
        game_event_emitter.subscribe(move |event| {
            let mut game_state = game_state.borrow_mut();
            game_state.handle_event(event.clone());
        });
    }

    fn new_game(&mut self, n_rows: usize) {
        let board_set = GameState::new_board_set(n_rows);
        self.puzzle_grid_ui
            .resize(n_rows, board_set.board.solution.n_variants);
        self.current_board = Rc::new(board_set.board);
        self.clue_set = board_set.clue_set;
        self.solution = board_set.solution;
        self.debug_mode = board_set.debug_mode;
        self.history.clear();
        self.history.push(self.current_board.clone());
        self.history_index = 0;
        self.hints_used = 0;
        self.current_playthrough_id = Uuid::new_v4();
        self.is_paused = false;
        self.timer_state = TimerState::default();
        self.clue_set_ui.set_clues(&self.clue_set);
        self.sync_board_display();
        self.sync_clues_completion_state();
        self.game_info.update_hints_used(self.hints_used);
        self.game_info.update_timer_state(&self.timer_state);
    }

    fn sync_cell(&mut self, row: usize, col: usize) {
        if let Some(cell) = self
            .puzzle_grid_ui
            .cells
            .get(row)
            .and_then(|row| row.get(col))
        {
            // If there's a solution, show it
            if let Some(tile) = self.current_board.selected[row][col] {
                cell.set_solution(Some(&tile));
            } else {
                // Otherwise show candidates
                cell.set_solution(None);
                let correct_tile = self.solution.get(row, col);
                for (i, variant) in self.current_board.get_variants().iter().enumerate() {
                    if let Some(candidate) = self.current_board.get_candidate(row, col, *variant) {
                        cell.set_candidate(i, Some(&candidate));
                        // In debug mode, highlight the correct candidate
                        if self.debug_mode && candidate.tile == correct_tile {
                            cell.highlight_candidate(i, Some("correct-candidate"));
                        } else {
                            cell.highlight_candidate(i, None);
                        }
                    }
                }
            }
        }
    }

    /// moves the GameBoard into an Rc, sets it as the current state, pushes the history
    fn push_board(&mut self, board: GameBoard) {
        self.current_board = Rc::new(board);
        // if we're not at the end of the list, prune redo state
        if self.history_index < self.history.len() - 1 {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(Rc::clone(&self.current_board));
        self.history_index += 1;
        self.sync_board_display();
        self.sync_clues_completion_state();
    }

    fn undo(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.current_board = self.history[self.history_index].clone();
            self.sync_board_display();
            self.sync_clues_completion_state();
        }
    }

    fn redo(&mut self) {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.current_board = self.history[self.history_index].clone();
            self.sync_board_display();
            self.sync_clues_completion_state();
        }
    }

    fn sync_board_display(&mut self) {
        for row in 0..self.current_board.solution.n_rows {
            for col in 0..self.current_board.solution.n_variants {
                self.sync_cell(row, col);
            }
        }
        // sync submit button
        let all_cells_filled = self.current_board.is_complete();
        self.submit_button.set_sensitive(all_cells_filled);
        if all_cells_filled {
            self.submit_button.add_css_class("submit-ready");
        } else {
            self.submit_button.remove_css_class("submit-ready");
        }

        if self.history_index > 0 {
            self.undo_button.set_sensitive(true);
        } else {
            self.undo_button.set_sensitive(false);
        }

        if self.history_index < self.history.len() - 1 {
            self.redo_button.set_sensitive(true);
        } else {
            self.redo_button.set_sensitive(false);
        }
    }

    fn handle_cell_click(&mut self, row: usize, col: usize, variant: Option<char>) {
        // If there's already a solution in this cell, ignore the click
        if self.current_board.selected[row][col].is_some() {
            return;
        }

        if let Some(variant) = variant {
            if let Some(candidate) = self.current_board.get_candidate(row, col, variant) {
                let mut current_board = self.current_board.as_ref().clone();
                match candidate.state {
                    CandidateState::Eliminated => {
                        current_board.show_candidate(row, col, candidate.tile);
                    }
                    CandidateState::Available => {
                        current_board.select_tile_at_position(row, col, candidate.tile);
                        current_board.auto_solve_row(row);
                    }
                }
                self.push_board(current_board);
                self.sync_board_display();
            }
        }
    }

    pub fn handle_event(&mut self, event: GameEvent) {
        log::trace!(target: "game_state", "Handling event: {:?}", event);
        match event {
            GameEvent::CellClick(row, col, variant) => self.handle_cell_click(row, col, variant),
            GameEvent::CellRightClick(row, col, variant) => {
                self.handle_cell_right_click(row, col, variant)
            }
            GameEvent::HorizontalClueClick(clue_idx) => self.handle_horizontal_clue_click(clue_idx),
            GameEvent::VerticalClueClick(clue_idx) => self.handle_vertical_clue_click(clue_idx),
            GameEvent::NewGame(rows) => self.new_game(rows),
            GameEvent::InitDisplay => {
                self.sync_board_display();
                self.sync_clues_completion_state();
            }
            GameEvent::Solve => self.try_solve(),
            GameEvent::RewindLastGood => self.rewind_last_good(),
            GameEvent::IncrementHintsUsed => self.increment_hints_used(),
            GameEvent::ShowHint => {
                self.show_hint();
            }
            GameEvent::Undo => self.undo(),
            GameEvent::Redo => self.redo(),
            GameEvent::Pause => self.pause_game(),
            GameEvent::Resume => self.resume_game(),
            GameEvent::Quit => (),
        }
    }

    fn handle_cell_right_click(&mut self, row: usize, col: usize, variant: Option<char>) {
        let mut current_board = self.current_board.as_ref().clone();
        // First check if there's a solution selected
        if current_board.selected[row][col].is_some() {
            // Reset the cell back to candidates
            current_board.selected[row][col] = None;
            self.push_board(current_board);
            return;
        }

        // If no solution, handle candidate right-click
        if let Some(variant) = variant {
            if let Some(candidate) = self.current_board.get_candidate(row, col, variant) {
                if candidate.state == CandidateState::Available {
                    current_board.remove_candidate(row, col, candidate.tile);
                    current_board.auto_solve_row(row);
                    self.push_board(current_board);
                    self.sync_board_display();
                }
            }
        }
    }

    fn try_solve(&mut self) {
        let all_clues = self
            .clue_set
            .all_clues()
            .iter()
            .map(|c| c.clue.clone())
            .collect();
        let mut current_board = self.current_board.as_ref().clone();
        let solution = perform_evaluation_step(&mut current_board, &all_clues);
        match solution {
            EvaluationStepResult::Nothing => {
                log::info!(
                    "No solution found in seed {:?}",
                    self.current_board.solution.seed
                );
                return;
            }
            EvaluationStepResult::HiddenPairsFound => {
                log::info!("Hidden pairs found");
            }
            EvaluationStepResult::DeductionsFound(clues) => {
                log::info!("Deductions found from clue: {:?}", clues);
            }
        }
        current_board.auto_solve_all();
        self.push_board(current_board);
        self.sync_board_display();
    }

    fn find_deductions(&self) -> Option<DeductionResult> {
        for clue_grouping in self.clue_set.all_clues() {
            let deductions = deduce_clue(&self.current_board, &clue_grouping.clue);
            if !deductions.is_empty() {
                return Some(DeductionResult {
                    deductions,
                    clue: Some(clue_grouping.clone()),
                });
            }
        }

        // look for hidden pairs
        let hidden_pairs = deduce_hidden_pairs(&self.current_board);
        if !hidden_pairs.is_empty() {
            return Some(DeductionResult {
                deductions: hidden_pairs,
                clue: None,
            });
        }
        None
    }

    fn increment_hints_used(&mut self) {
        if self.hint_status.history_index != self.history_index {
            self.hint_status.history_index = self.history_index;
            self.hint_status.hint_level = 0;
            self.hints_used += 1;
        } else if self.hint_status.hint_level < HINT_LEVEL_MAX {
            self.hints_used += 1;
            self.hint_status.hint_level += 1;
        }
        self.game_info.update_hints_used(self.hints_used);
    }

    fn show_hint(&mut self) -> bool {
        let deduction_result = self.find_deductions();

        if deduction_result.is_some() {
            self.increment_hints_used();
        }
        log::info!(
            target: "game_state",
            "Deduction result: {:?}; seed: {:?}",
            deduction_result,
            self.current_board.solution.seed
        );

        if let Some(DeductionResult { deductions, clue }) = deduction_result {
            if let Some(clue) = clue {
                self.clue_set_ui.highlight_clue(
                    clue.orientation,
                    clue.index,
                    Duration::from_secs(1),
                );
            }

            if self.hint_status.hint_level > 0 {
                if let Some(first_deduction) = deductions.first() {
                    // highlight cells
                    self.puzzle_grid_ui.highlight_candidate(
                        first_deduction.tile.row,
                        first_deduction.column,
                        first_deduction.tile.variant,
                        Duration::from_secs(1),
                        // .cells
                        // .get(first_deduction.tile.row)
                        // .and_then(|row| row.get(first_deduction.column))
                        // .unwrap()
                        // .highlight_candidate_for(
                        //     Duration::from_secs(1),
                        //     first_deduction.tile.variant,
                        // );
                    );
                }
            }
            return true;
        }
        false
    }

    fn rewind_last_good(&mut self) {
        while self.history_index > 0 && self.current_board.is_incorrect() {
            self.history_index -= 1;
            self.current_board = self.history[self.history_index].clone();
            self.sync_board_display();
        }
    }

    pub fn get_game_stats(&self) -> GameStats {
        let completion_time = self.timer_state.elapsed();
        let stats = GameStats {
            completion_time,
            hints_used: self.hints_used,
            grid_size: self.current_board.solution.n_rows,
            difficulty: self.get_difficulty(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            playthrough_id: self.current_playthrough_id,
        };
        stats
    }

    fn sync_clues_completion_state(&self) {
        // Display horizontal clues
        let horizontal_clues = self.clue_set.horizontal_clues();

        for (clue_idx, _) in horizontal_clues.iter().enumerate() {
            self.clue_set_ui.set_horiz_completion(
                clue_idx,
                self.current_board
                    .completed_horizontal_clues
                    .contains(&clue_idx),
            );
        }

        for (clue_idx, _) in self.clue_set.vertical_clues().iter().enumerate() {
            self.clue_set_ui.set_vert_completion(
                clue_idx,
                self.current_board
                    .completed_vertical_clues
                    .contains(&clue_idx),
            );
        }
    }

    fn handle_horizontal_clue_click(&mut self, clue_idx: usize) {
        let mut current_board = self.current_board.as_ref().clone();
        current_board.toggle_horizontal_clue_completed(clue_idx);
        self.push_board(current_board);
    }

    fn handle_vertical_clue_click(&mut self, clue_idx: usize) {
        let mut current_board = self.current_board.as_ref().clone();
        current_board.toggle_vertical_clue_completed(clue_idx);
        self.push_board(current_board);
    }

    pub fn get_difficulty(&self) -> Difficulty {
        self.current_board.solution.difficulty
    }

    fn pause_game(&mut self) {
        if !self.is_paused {
            self.is_paused = true;
            self.timer_state.paused_timestamp = Some(Instant::now());
            self.game_info.update_timer_state(&self.timer_state);
            // Hide the puzzle grid
            self.puzzle_grid_ui.hide();
            self.clue_set_ui.hide();
        }
    }

    fn resume_game(&mut self) {
        if self.is_paused {
            self.is_paused = false;
            if let Some(pause_time) = self.timer_state.paused_timestamp.take() {
                // Add the duration of this pause to total_paused_duration
                self.timer_state.paused_duration += pause_time.elapsed();
                self.game_info.update_timer_state(&self.timer_state);
            }
            // Show the puzzle grid
            self.puzzle_grid_ui.show();
            self.clue_set_ui.show();
        }
    }
}
