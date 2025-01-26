use super::Clue;
use crate::model::GameBoard;

#[derive(Debug)]
pub enum UIEvent {
    GamePaused(bool),
    HistoryChanged {
        undo_available: bool,
        redo_available: bool,
    },
    GridUpdate(GameBoard),
    ClueStatusUpdate {
        horizontal_hidden_tiles: Vec<usize>,
        vertical_hidden_tiles: Vec<usize>,
    },
    ShowHint {
        clue: Clue,
        cell: (usize, usize),
    },
}

impl UIEvent {}
