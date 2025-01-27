use super::{Clue, ClueSet, ClueWithGrouping, TimerState};
use crate::model::GameBoard;
use std::{collections::HashSet, rc::Rc, time::Duration};

#[derive(Debug)]
pub enum GameStateEvent {
    HistoryChanged {
        history_index: usize,
        history_length: usize,
    },
    GridUpdate(GameBoard),
    ClueStatusUpdate {
        horizontal_hidden_tiles: Vec<usize>,
        vertical_hidden_tiles: Vec<usize>,
    },
    CellHintHighlight {
        cell: (usize, usize),
        variant: char,
    },
    HintUsageChanged(u32),
    TimerStateChanged(TimerState),
    PuzzleCompletionStateChanged(bool),
    ClueHintHighlight {
        clue: ClueWithGrouping,
    },
    ClueSetUpdate(Rc<ClueSet>),
    PuzzleVisibilityChanged(bool),
    ClueVisibilityChanged {
        horizontal_clues: HashSet<usize>,
        vertical_clues: HashSet<usize>,
    },
}

impl GameStateEvent {}
