use super::{ClueOrientation, Difficulty};

#[derive(Debug, Clone)]
pub enum GameActionEvent {
    CellClick(usize, usize, Option<char>),
    CellRightClick(usize, usize, Option<char>),
    ClueToggleComplete(ClueOrientation, usize), // clue_idx
    ClueToggleSelectedComplete,
    ClueSelect(Option<(ClueOrientation, usize)>), // clue_idx when Some
    ClueSelectNext(i32),
    NewGame(Difficulty, Option<u64>), // grid rows, grid columns
    InitDisplay,
    CompletePuzzle,
    Solve,
    RewindLastGood,
    IncrementHintsUsed,
    ShowHint,
    Undo,
    Redo,
    Pause,
    Resume,
    Quit,
    Submit,
    Restart,
}
