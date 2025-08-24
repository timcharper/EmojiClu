use super::{ClueSet, ClueWithAddress, Deduction, Difficulty, TimerState};
use crate::model::{ClueAddress, GameBoard, GameStats};
use std::{collections::HashSet, rc::Rc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClueSelection {
    pub clue: ClueWithAddress,
    pub is_focused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PuzzleCompletionState {
    Incomplete,
    Correct(GameStats),
    Incorrect,
}

#[derive(Debug)]
pub enum GameStateEvent {
    HistoryChanged {
        history_index: usize,
        history_length: usize,
    },
    GridUpdated(GameBoard),
    ClueStatusUpdated {
        horizontal_hidden_tiles: Vec<usize>,
        vertical_hidden_tiles: Vec<usize>,
    },
    HintSuggested(Deduction),
    HintUsageChanged(u32),
    TimerStateChanged(TimerState),
    PuzzleSubmissionReadyChanged(bool),
    PuzzleCompleted(PuzzleCompletionState),
    ClueHintHighlighted(Option<ClueWithAddress>),
    ClueSetUpdated(Rc<ClueSet>, Difficulty, HashSet<ClueAddress>),
    ClueSelected(Option<ClueSelection>),
}

impl GameStateEvent {}
