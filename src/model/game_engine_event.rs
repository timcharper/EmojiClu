use super::{ClueSet, ClueWithAddress, Deduction, Difficulty, TimerState};
use crate::game::settings::Settings;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameBoardChangeReason {
    NewGame,
    Undo,
    Redo,
    ClueStatusChanged,
    TileStatusChanged,
    GameLoaded,
}

#[derive(Debug)]
pub enum GameEngineEvent {
    GameBoardUpdated {
        board: GameBoard,
        history_index: usize,
        history_length: usize,
        change_reason: GameBoardChangeReason,
    },
    ClueStatusUpdated {
        horizontal_hidden_tiles: Vec<usize>,
        vertical_hidden_tiles: Vec<usize>,
    },
    ClueHintHighlighted(Option<ClueWithAddress>),
    ClueSetUpdated(Rc<ClueSet>, Difficulty, HashSet<ClueAddress>),
    ClueSelected(Option<ClueSelection>),
    HintSuggested(Deduction),
    HintUsageChanged(u32),
    TimerStateChanged(TimerState),
    PuzzleSubmissionReadyChanged(bool),
    PuzzleCompleted(PuzzleCompletionState),
    SettingsChanged(Settings),
}

impl GameEngineEvent {}
