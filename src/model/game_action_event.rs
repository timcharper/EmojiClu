use super::{ClueOrientation, Difficulty};

#[derive(Debug, Clone)]
pub enum GameActionEvent {
    CellSelect(usize, usize, Option<char>),
    CellClear(usize, usize, Option<char>),
    ClueToggleComplete(ClueOrientation, usize), // clue_idx
    ClueToggleSelectedComplete,
    ClueFocus(Option<(ClueOrientation, usize)>), // clue_idx when Some
    ClueFocusNext(i32),
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
