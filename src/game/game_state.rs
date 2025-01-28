use log::trace;
use std::cell::RefCell;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use super::clue_generator::ClueGeneratorResult;
use super::solver::{deduce_hidden_pairs, perform_evaluation_step, EvaluationStepResult};
use super::stats_manager::GameStats;
use super::{deduce_clue, generate_clues};
use crate::destroyable::Destroyable;
use crate::events::{EventEmitter, EventObserver, Unsubscriber};
use crate::model::{
    CandidateState, ClueSet, ClueWithGrouping, Deduction, Difficulty, GameActionEvent, GameBoard,
    GameStateEvent, Solution, TimerState,
};
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
    clue_set: Rc<ClueSet>,
    history: Vec<Rc<GameBoard>>,
    pub current_board: Rc<GameBoard>,
    solution: Rc<Solution>,
    debug_mode: bool,
    history_index: usize,
    hints_used: u32,
    hint_status: HintStatus,
    current_playthrough_id: Uuid,
    is_paused: bool,
    timer_state: TimerState,
    game_action_observer: EventObserver<GameActionEvent>,
    subscription_id: Option<Unsubscriber<GameActionEvent>>,
    game_state_emitter: EventEmitter<GameStateEvent>,
}

impl Destroyable for GameState {
    fn destroy(&mut self) {
        if let Some(subscription_id) = self.subscription_id.take() {
            subscription_id.unsubscribe();
        }
    }
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

    pub fn new_board_set(difficulty: Difficulty) -> GameBoardSet {
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

    pub fn new(
        game_action_observer: EventObserver<GameActionEvent>,
        game_state_emitter: EventEmitter<GameStateEvent>,
    ) -> Rc<RefCell<Self>> {
        let board_set = GameBoardSet::default();
        let timer_state = TimerState::default();
        let game_state = Self {
            clue_set: board_set.clue_set.clone(),
            history: vec![Rc::new(board_set.board.clone())],
            current_board: Rc::new(board_set.board),
            solution: board_set.solution.clone(),
            debug_mode: board_set.debug_mode,
            history_index: 0,
            hints_used: 0,
            hint_status: HintStatus {
                history_index: 0,
                hint_level: 0,
            },
            current_playthrough_id: Uuid::new_v4(),
            is_paused: false,
            timer_state,
            game_action_observer: game_action_observer.clone(),
            subscription_id: None,
            game_state_emitter,
        };
        let refcell = Rc::new(RefCell::new(game_state));
        GameState::wire_subscription(refcell.clone(), game_action_observer);
        refcell
    }

    fn wire_subscription(
        game_state: Rc<RefCell<Self>>,
        game_action_emitter: EventObserver<GameActionEvent>,
    ) {
        let game_state_handler = game_state.clone();
        let subscription_id = game_action_emitter.subscribe(move |event| {
            let mut game_state = game_state_handler.borrow_mut();
            game_state.handle_event(event.clone());
        });
        game_state.borrow_mut().subscription_id = Some(subscription_id);
    }

    fn new_game(&mut self, difficulty: Difficulty) {
        let board_set = GameState::new_board_set(difficulty);
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
        self.sync_board_display();
        self.sync_clues_completion_state();
        self.game_state_emitter
            .emit(&GameStateEvent::HintUsageChanged(self.hints_used));
        self.game_state_emitter
            .emit(&GameStateEvent::TimerStateChanged(self.timer_state.clone()));
        self.game_state_emitter
            .emit(&GameStateEvent::ClueSetUpdate(self.clue_set.clone()));
        self.game_state_emitter
            .emit(&GameStateEvent::HistoryChanged {
                history_index: self.history_index,
                history_length: self.history.len(),
            });
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

    /// moves the GameBoard into an Rc, sets it as the current state, pushes the history
    fn push_board(&mut self, board: GameBoard) {
        self.current_board = Rc::new(board);
        // if we're not at the end of the list, prune redo state
        if self.history_index < self.history.len() - 1 {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(Rc::clone(&self.current_board));
        self.history_index += 1;

        self.game_state_emitter
            .emit(&GameStateEvent::HistoryChanged {
                history_index: self.history_index,
                history_length: self.history.len(),
            });

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

        self.game_state_emitter
            .emit(&GameStateEvent::HistoryChanged {
                history_index: self.history_index,
                history_length: self.history.len(),
            });
    }

    fn redo(&mut self) {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.current_board = self.history[self.history_index].clone();
            self.sync_board_display();
            self.sync_clues_completion_state();
        }

        self.game_state_emitter
            .emit(&GameStateEvent::HistoryChanged {
                history_index: self.history_index,
                history_length: self.history.len(),
            });
    }

    fn sync_board_display(&mut self) {
        // Emit grid update event
        self.game_state_emitter.emit(&GameStateEvent::GridUpdate(
            self.current_board.as_ref().clone(),
        ));
        // Emit completion state event
        let all_cells_filled = self.current_board.is_complete();
        self.game_state_emitter
            .emit(&GameStateEvent::PuzzleCompletionStateChanged(
                all_cells_filled,
            ));
    }

    pub fn handle_event(&mut self, event: GameActionEvent) {
        log::trace!(target: "game_state", "Handling event: {:?}", event);
        match event {
            GameActionEvent::CellClick(row, col, variant) => {
                self.handle_cell_click(row, col, variant)
            }
            GameActionEvent::CellRightClick(row, col, variant) => {
                self.handle_cell_right_click(row, col, variant)
            }
            GameActionEvent::HorizontalClueClick(clue_idx) => {
                self.handle_horizontal_clue_click(clue_idx)
            }
            GameActionEvent::VerticalClueClick(clue_idx) => {
                self.handle_vertical_clue_click(clue_idx)
            }
            GameActionEvent::NewGame(difficulty) => self.new_game(difficulty),
            GameActionEvent::InitDisplay => {
                self.sync_board_display();
                self.sync_clues_completion_state();
            }
            GameActionEvent::Solve => self.try_solve(),
            GameActionEvent::RewindLastGood => self.rewind_last_good(),
            GameActionEvent::IncrementHintsUsed => self.increment_hints_used(),
            GameActionEvent::ShowHint => {
                self.show_hint();
            }
            GameActionEvent::Undo => self.undo(),
            GameActionEvent::Redo => self.redo(),
            GameActionEvent::Pause => self.pause_game(),
            GameActionEvent::Resume => self.resume_game(),
            GameActionEvent::Quit => (),
            GameActionEvent::Submit => todo!(),
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
        self.game_state_emitter
            .emit(&GameStateEvent::HintUsageChanged(self.hints_used));
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
                self.game_state_emitter
                    .emit(&GameStateEvent::ClueHintHighlight { clue: clue.clone() });
            }

            if self.hint_status.hint_level > 0 {
                if let Some(first_deduction) = deductions.first() {
                    // highlight cells

                    self.game_state_emitter
                        .emit(&GameStateEvent::CellHintHighlight {
                            cell: (first_deduction.tile.row, first_deduction.column),
                            variant: first_deduction.tile.variant,
                        });
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
        self.game_state_emitter
            .emit(&GameStateEvent::HistoryChanged {
                history_index: self.history_index,
                history_length: self.history.len(),
            });
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
        self.game_state_emitter
            .emit(&GameStateEvent::ClueVisibilityChanged {
                horizontal_clues: self.current_board.completed_horizontal_clues.clone(),
                vertical_clues: self.current_board.completed_vertical_clues.clone(),
            });
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
            self.game_state_emitter
                .emit(&GameStateEvent::TimerStateChanged(self.timer_state.clone()));
            self.game_state_emitter
                .emit(&GameStateEvent::PuzzleVisibilityChanged(false));
        }
    }

    fn resume_game(&mut self) {
        if self.is_paused {
            self.is_paused = false;
            if let Some(pause_time) = self.timer_state.paused_timestamp.take() {
                // Add the duration of this pause to total_paused_duration
                self.timer_state.paused_duration += pause_time.elapsed();
                self.game_state_emitter
                    .emit(&GameStateEvent::TimerStateChanged(self.timer_state.clone()));
            }
            self.game_state_emitter
                .emit(&GameStateEvent::PuzzleVisibilityChanged(true));
        }
    }
}
