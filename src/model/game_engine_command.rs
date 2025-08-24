use super::{ClueAddress, Difficulty, GameStateSnapshot};

#[derive(Debug, Clone)]
pub enum GameEngineCommand {
    CellSelect(usize, usize, Option<char>),
    CellClear(usize, usize, Option<char>),
    ClueToggleComplete(ClueAddress), // clue_idx
    ClueToggleSelectedComplete,
    ClueFocus(Option<ClueAddress>), // clue_idx when Some
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
    LoadState(GameStateSnapshot),
}
