use log::{error, trace};
use std::cell::RefCell;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use super::settings::Settings;
use crate::destroyable::Destroyable;
use crate::events::{EventEmitter, EventObserver, Unsubscriber};
use crate::model::game_state_snapshot::GameStateSnapshot;
use crate::model::{
    CandidateState, ClueAddress, ClueSelection, ClueSet, ClueWithAddress, Deduction, Difficulty,
    GameBoard, GameEngineCommand, GameEngineEvent, GameStats, GlobalEvent, PuzzleCompletionState,
    Solution, TimerState,
};
use crate::solver::candidate_solver::{
    deduce_hidden_sets, perform_evaluation_step, EvaluationStepResult,
};
use crate::solver::{deduce_clue, simplify_deductions, ConstraintSolver};
use std::rc::Rc;

const HINT_LEVEL_MAX: u8 = 1;

struct DeductionResult {
    deductions: Vec<Deduction>,
    clue: Option<ClueWithAddress>,
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

impl Default for HintStatus {
    fn default() -> Self {
        Self {
            history_index: usize::MAX,
            hint_level: 0,
        }
    }
}

pub struct GameEngine {
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
    subscription_id: Option<Unsubscriber<GameEngineCommand>>,
    global_subscription_id: Option<Unsubscriber<GlobalEvent>>,
    game_engine_event_emitter: EventEmitter<GameEngineEvent>,
    settings: Settings,
    current_selected_clue: Option<ClueWithAddress>,
    clue_focused: bool,
    current_clue_hint: Option<ClueWithAddress>,
}

impl Destroyable for GameEngine {
    fn destroy(&mut self) {
        if let Some(subscription_id) = self.subscription_id.take() {
            subscription_id.unsubscribe();
        }
        if let Some(subscription_id) = self.global_subscription_id.take() {
            subscription_id.unsubscribe();
        }
    }
}

impl GameEngine {
    pub fn new(
        game_engine_command_observer: EventObserver<GameEngineCommand>,
        game_engine_event_emitter: EventEmitter<GameEngineEvent>,
        global_event_observer: EventObserver<GlobalEvent>,
        settings: Settings,
    ) -> Rc<RefCell<Self>> {
        let empty_board = Rc::new(GameBoard::default());
        let game_state = Self {
            clue_set: empty_board.clue_set.clone(),
            history: vec![empty_board.clone()],
            current_board: empty_board.clone(),
            solution: empty_board.solution.clone(),
            debug_mode: Settings::is_debug_mode(),
            history_index: 0,
            hints_used: 0,
            hint_status: HintStatus::default(),
            current_playthrough_id: Uuid::new_v4(),
            is_paused: false,
            timer_state: TimerState::default(),
            subscription_id: None,
            global_subscription_id: None,
            game_engine_event_emitter,
            settings,
            current_selected_clue: None,
            clue_focused: false,
            current_clue_hint: None,
        };
        let refcell = Rc::new(RefCell::new(game_state));
        GameEngine::wire_subscription(refcell.clone(), game_engine_command_observer);
        GameEngine::wire_global_subscription(refcell.clone(), global_event_observer);
        refcell
    }

    fn wire_subscription(
        game_state: Rc<RefCell<Self>>,
        game_engine_command_emitter: EventObserver<GameEngineCommand>,
    ) {
        let game_state_handler = game_state.clone();
        let subscription_id = game_engine_command_emitter.subscribe(move |event| {
            let mut game_state = game_state_handler.borrow_mut();
            game_state.handle_command(event.clone());
        });
        game_state.borrow_mut().subscription_id = Some(subscription_id);
    }

    fn wire_global_subscription(
        game_state: Rc<RefCell<Self>>,
        global_event_observer: EventObserver<GlobalEvent>,
    ) {
        let game_state_handler = game_state.clone();
        let subscription_id = global_event_observer.subscribe(move |event| {
            let mut game_state = game_state_handler.borrow_mut();
            game_state.handle_global_event(event);
        });
        game_state.borrow_mut().global_subscription_id = Some(subscription_id);
    }

    fn set_game_state(&mut self, game_state_snapshot: &GameStateSnapshot) {
        println!(
            "New game; difficulty: {:?}; seed: {:?}",
            game_state_snapshot.board.solution.difficulty, game_state_snapshot.board.solution.seed
        );
        self.current_board = Rc::new(game_state_snapshot.board.clone());
        self.clue_set = Rc::clone(&self.current_board.clue_set);
        self.solution = Rc::clone(&self.current_board.solution);
        self.debug_mode = Settings::is_debug_mode();
        self.history.clear();
        self.history.push(self.current_board.clone());
        self.history_index = 0;
        self.hints_used = game_state_snapshot.hints_used;
        self.current_playthrough_id = Uuid::new_v4();
        self.is_paused = false;
        self.timer_state = game_state_snapshot.timer_state.resumed();
        self.current_selected_clue = None;
        self.clue_focused = false;
        self.hint_status = HintStatus::default();
        self.sync_board_display();
        self.game_engine_event_emitter
            .emit(GameEngineEvent::HintUsageChanged(self.hints_used));
        self.game_engine_event_emitter
            .emit(GameEngineEvent::TimerStateChanged(self.timer_state.clone()));
        self.game_engine_event_emitter
            .emit(GameEngineEvent::ClueSetUpdated(
                self.clue_set.clone(),
                self.current_board.solution.difficulty,
                self.current_board.completed_clues.clone(),
            ));
        self.game_engine_event_emitter
            .emit(GameEngineEvent::HistoryChanged {
                history_index: self.history_index,
                history_length: self.history.len(),
            });
        self.sync_clue_selection();
    }

    fn handle_cell_select(&mut self, row: usize, col: usize, variant: Option<char>) {
        // If there's already a solution in this cell, ignore the click
        if self.current_board.get_selection(row, col).is_some() {
            return;
        }

        if let Some(variant) = variant {
            if let Some(candidate) = self.current_board.get_candidate(row, col, variant) {
                let mut current_board = self.current_board.as_ref().clone();
                match candidate.state {
                    CandidateState::Eliminated => {
                        current_board.show_candidate(col, candidate.tile);
                    }
                    CandidateState::Available => {
                        current_board.select_tile_at_position(col, candidate.tile);
                        current_board.auto_solve_row(row);
                    }
                }
                self.push_board(current_board);
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

        self.game_engine_event_emitter
            .emit(GameEngineEvent::HistoryChanged {
                history_index: self.history_index,
                history_length: self.history.len(),
            });

        self.maybe_reset_clue_hint();
        self.sync_board_display();
    }

    fn undo(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.current_board = self.history[self.history_index].clone();
            self.sync_board_display();
        }

        self.game_engine_event_emitter
            .emit(GameEngineEvent::HistoryChanged {
                history_index: self.history_index,
                history_length: self.history.len(),
            });
    }

    fn redo(&mut self) {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.current_board = self.history[self.history_index].clone();
            self.sync_board_display();
        }

        self.game_engine_event_emitter
            .emit(GameEngineEvent::HistoryChanged {
                history_index: self.history_index,
                history_length: self.history.len(),
            });
    }

    fn sync_board_display(&mut self) {
        // Emit grid update event
        self.game_engine_event_emitter
            .emit(GameEngineEvent::GameBoardUpdated(
                self.current_board.as_ref().clone(),
            ));
        // Emit completion state event
        let all_cells_filled = self.current_board.is_complete();
        if self.get_difficulty() != Difficulty::Tutorial {
            // we don't want to show submission screen for tutorial
            self.game_engine_event_emitter
                .emit(GameEngineEvent::PuzzleSubmissionReadyChanged(
                    all_cells_filled,
                ));
        }
        if all_cells_filled {
            self.clue_focused = false;
            self.sync_clue_selection();
        }
    }

    fn handle_command(&mut self, event: GameEngineCommand) {
        log::trace!(target: "game_state", "Handling event: {:?}", event);
        match event {
            GameEngineCommand::CellSelect(row, col, variant) => {
                self.handle_cell_select(row, col, variant)
            }
            GameEngineCommand::CellClear(row, col, variant) => {
                self.handle_cell_clear(row, col, variant)
            }
            GameEngineCommand::NewGame(difficulty, seed) => {
                self.set_game_state(&GameStateSnapshot::generate_new(difficulty, seed));
            }
            GameEngineCommand::LoadState(save_state) => {
                trace!(target: "game_state", "Loading saved state {:?}", save_state);
                self.set_game_state(&save_state);
            }
            GameEngineCommand::InitDisplay => {
                self.sync_board_display();
            }
            GameEngineCommand::Solve => self.try_solve(),
            GameEngineCommand::RewindLastGood => self.rewind_last_good(),
            GameEngineCommand::IncrementHintsUsed => self.increment_hints_used(),
            GameEngineCommand::ShowHint => {
                self.show_hint();
            }
            GameEngineCommand::Undo => self.undo(),
            GameEngineCommand::Redo => self.redo(),
            GameEngineCommand::Pause => self.pause_game(),
            GameEngineCommand::Resume => self.resume_game(),
            GameEngineCommand::Quit => (),
            GameEngineCommand::Submit => todo!(),
            GameEngineCommand::CompletePuzzle => self.complete_puzzle(),
            GameEngineCommand::Restart => {
                // Start a new game with current difficulty and seed
                let current_seed = self.current_board.solution.seed;
                let current_difficulty = self.current_board.solution.difficulty;
                self.set_game_state(&GameStateSnapshot::generate_new(
                    current_difficulty,
                    Some(current_seed),
                ));
            }
            GameEngineCommand::ClueToggleComplete(clue_address) => {
                self.handle_clue_toggle_complete(clue_address)
            }
            GameEngineCommand::ClueToggleSelectedComplete => {
                if let Some(addressed_clue) = &self.current_selected_clue {
                    self.handle_clue_toggle_complete(addressed_clue.address())
                }
            }
            GameEngineCommand::ClueFocus(maybe_clue) => self.focus_clue(maybe_clue),
            GameEngineCommand::ClueFocusNext(direction) => self.focus_next_clue(direction),
        }
    }
    fn focus_next_clue(&mut self, direction: i32) {
        match &self.current_selected_clue {
            Some(addressed_clue) => {
                let mut tries = self.clue_set.all_clues().count() + 1;
                let mut orientation = addressed_clue.address().orientation;
                let mut clue_idx = addressed_clue.address().index as i32;
                self.current_selected_clue = None;
                // if all clues are hidden, we don't want to try forever
                while tries > 0 {
                    clue_idx = clue_idx + direction;

                    if clue_idx < 0 {
                        orientation = orientation.invert();
                        clue_idx = self.clue_set.get_clue_count(orientation) as i32 - 1;
                    } else if clue_idx >= self.clue_set.get_clue_count(orientation) as i32 {
                        orientation = orientation.invert();
                        clue_idx = 0;
                    }

                    if !self.current_board.is_clue_completed(&ClueAddress {
                        orientation,
                        index: clue_idx as usize,
                    }) {
                        self.current_selected_clue = self
                            .current_board
                            .clue_set
                            .get_clue(ClueAddress {
                                orientation,
                                index: clue_idx as usize,
                            })
                            .cloned();
                        break;
                    }
                    tries -= 1;
                }
            }
            None => {
                self.current_selected_clue = self.clue_set.horizontal_clues().first().cloned();
            }
        }
        self.clue_focused = true;

        self.sync_clue_selection();
    }

    fn complete_puzzle(&mut self) {
        if self.current_board.is_complete() {
            if self.current_board.is_incorrect() {
                self.game_engine_event_emitter
                    .emit(GameEngineEvent::PuzzleCompleted(
                        PuzzleCompletionState::Incorrect,
                    ));
            } else {
                self.game_engine_event_emitter
                    .emit(GameEngineEvent::PuzzleCompleted(
                        PuzzleCompletionState::Correct(self.get_game_stats()),
                    ));

                self.timer_state = self.timer_state.ended(SystemTime::now());
                self.game_engine_event_emitter
                    .emit(GameEngineEvent::TimerStateChanged(self.timer_state.clone()));
            }
        } else {
            self.game_engine_event_emitter
                .emit(GameEngineEvent::PuzzleCompleted(
                    PuzzleCompletionState::Incomplete,
                ));
        }
    }

    fn handle_cell_clear(&mut self, row: usize, col: usize, variant: Option<char>) {
        let mut current_board = self.current_board.as_ref().clone();
        // First check if there's a solution selected
        if current_board.has_selection(row, col) {
            // Reset the cell back to candidates
            current_board.remove_selection(row, col);
            self.push_board(current_board);
            return;
        }

        // If no solution, handle candidate right-click
        if let Some(variant) = variant {
            if let Some(candidate) = self.current_board.get_candidate(row, col, variant) {
                if candidate.state == CandidateState::Available {
                    current_board.remove_candidate(col, candidate.tile);
                    current_board.auto_solve_row(row);
                    self.push_board(current_board);
                }
            }
        }
    }

    fn try_solve(&mut self) {
        let all_clues = self.clue_set.all_clues().map(|c| c.clue.clone()).collect();
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
            EvaluationStepResult::HiddenSetsFound => {
                log::info!("Hidden pairs found");
                self.game_engine_event_emitter
                    .emit(GameEngineEvent::ClueSelected(None));
            }
            EvaluationStepResult::DeductionsFound(clue) => {
                log::info!("Deductions found from clue: {:?}", clue);
                let addressed_clue = self
                    .clue_set
                    .find_clue(&clue)
                    .cloned()
                    .expect("This should have returned a clue");

                self.game_engine_event_emitter
                    .emit(GameEngineEvent::ClueSelected(Some(ClueSelection {
                        clue: addressed_clue,
                        is_focused: true,
                    })));
            }
        }
        current_board.auto_solve_all();
        self.push_board(current_board);
    }

    fn find_deductions(&self) -> Option<DeductionResult> {
        // First, look for obvious deductions using the simpler solver
        for clue_grouping in self.clue_set.all_clues() {
            let simple_deductions =
                ConstraintSolver::deduce_clue(&self.current_board, &clue_grouping.clue);
            if !simple_deductions.is_empty() {
                return Some(DeductionResult {
                    deductions: simplify_deductions(
                        &self.current_board,
                        simple_deductions,
                        &clue_grouping.clue,
                    ),
                    clue: Some(clue_grouping.clone()),
                });
            }
        }

        // Scan again using the advanced solver (which emits admittedly less obvious hints)
        for clue_grouping in self.clue_set.all_clues() {
            let deductions = deduce_clue(&self.current_board, &clue_grouping.clue);
            if !deductions.is_empty() {
                return Some(DeductionResult {
                    deductions: simplify_deductions(
                        &self.current_board,
                        deductions,
                        &clue_grouping.clue,
                    ),
                    clue: Some(clue_grouping.clone()),
                });
            }
        }

        // look for hidden pairs
        let hidden_pairs = deduce_hidden_sets(&self.current_board);
        if !hidden_pairs.is_empty() {
            return Some(DeductionResult {
                deductions: hidden_pairs,
                clue: None,
            });
        }
        // Nothing found! Oof.
        error!(
            target: "game_state",
            "No deductions found; seed: {:?}",
            self.current_board.solution.seed
        );
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
        self.game_engine_event_emitter
            .emit(GameEngineEvent::HintUsageChanged(self.hints_used));
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
            if let Some(addressed_clue) = &clue {
                self.game_engine_event_emitter
                    .emit(GameEngineEvent::ClueSelected(Some(ClueSelection {
                        clue: addressed_clue.clone(),
                        is_focused: true,
                    })));
                self.current_clue_hint = Some(addressed_clue.clone());

                self.focus_clue(Some(addressed_clue.address()));

                // is the clue disabled? Automatically re-enable it
                if self
                    .current_board
                    .is_clue_completed(&addressed_clue.address())
                {
                    let mut current_board = self.current_board.as_ref().clone();
                    current_board.toggle_clue_completed(addressed_clue.address());
                    self.push_board(current_board);
                }

                self.game_engine_event_emitter
                    .emit(GameEngineEvent::ClueHintHighlighted(Some(
                        addressed_clue.clone(),
                    )));
            }

            if self.hint_status.hint_level > 0 || clue.is_none() {
                if let Some(first_deduction) = deductions.first() {
                    // highlight cells

                    self.game_engine_event_emitter
                        .emit(GameEngineEvent::HintSuggested(first_deduction.clone()));
                }
            }
            return true;
        } else {
            log::error!(
                target: "game_state",
                "No deduction result found; seed: {:?}",
                self.current_board.solution.seed
            );
        }
        false
    }

    fn rewind_last_good(&mut self) {
        while self.history_index > 0 && self.current_board.is_incorrect() {
            self.history_index -= 1;
            self.current_board = self.history[self.history_index].clone();
            self.sync_board_display();
        }
        self.game_engine_event_emitter
            .emit(GameEngineEvent::HistoryChanged {
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

    fn handle_clue_toggle_complete(&mut self, clue_address: ClueAddress) {
        let mut current_board = self.current_board.as_ref().clone();
        current_board.toggle_clue_completed(clue_address);
        self.push_board(current_board);
        self.sync_clue_selection();
    }

    pub fn get_difficulty(&self) -> Difficulty {
        self.current_board.solution.difficulty
    }

    fn pause_game(&mut self) {
        if !self.is_paused {
            self.is_paused = true;
            self.timer_state = self.timer_state.paused(SystemTime::now());
            self.game_engine_event_emitter
                .emit(GameEngineEvent::TimerStateChanged(self.timer_state.clone()));
        }
    }

    fn resume_game(&mut self) {
        if self.is_paused {
            self.is_paused = false;
            self.timer_state = self.timer_state.resumed();
            self.game_engine_event_emitter
                .emit(GameEngineEvent::TimerStateChanged(self.timer_state.clone()));
        }
    }

    fn focus_clue(&mut self, maybe_clue_selection: Option<ClueAddress>) {
        if let Some(clue_address) = maybe_clue_selection {
            self.current_selected_clue = self.clue_set.get_clue(clue_address).cloned();
            self.clue_focused = true;
        } else {
            self.clue_focused = false;
        }
        self.maybe_reset_clue_hint();
        self.sync_clue_selection();
    }

    fn maybe_reset_clue_hint(&mut self) {
        if let Some(addressed_clue) = self.current_clue_hint.clone() {
            // different clue selected? Clear it.
            if self.current_clue_hint != self.current_selected_clue {
                self.current_clue_hint = None;
            }

            // no more deductions remaining? Clear it
            let deductions = deduce_clue(&self.current_board, &addressed_clue.clue);
            if deductions.is_empty() {
                self.current_clue_hint = None;
            }

            if self.current_clue_hint.is_none() {
                self.game_engine_event_emitter
                    .emit(GameEngineEvent::ClueHintHighlighted(None));
            }
        }
    }

    fn sync_clue_selection(&mut self) {
        let clue = self.current_selected_clue.clone();

        self.game_engine_event_emitter
            .emit(GameEngineEvent::ClueSelected(clue.map(|c| ClueSelection {
                clue: c,
                is_focused: self.clue_focused,
            })));
    }

    fn update_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }

    fn handle_global_event(&mut self, event: &GlobalEvent) {
        match event {
            GlobalEvent::SettingsChanged(settings) => self.update_settings(settings.clone()),
            _ => (),
        }
    }

    pub fn get_game_save_state(&self) -> GameStateSnapshot {
        GameStateSnapshot::new(
            self.current_board.as_ref().clone(),
            self.timer_state.paused(SystemTime::now()),
            self.hints_used,
        )
    }
}
