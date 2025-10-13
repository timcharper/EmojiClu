use super::{ClueAddress, Difficulty, GameStateSnapshot};

#[derive(Debug, Clone, Default)]

pub struct SettingsChange {
    pub clue_tooltips_enabled: Option<bool>,
    pub clue_spotlight_enabled: Option<bool>,
    pub touch_screen_controls: Option<bool>,
    pub auto_solve_enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub enum GameEngineCommand {
    CellSelect(usize, usize, Option<char>),
    CellClear(usize, usize, Option<char>),
    ClueToggleComplete(ClueAddress), // clue_idx
    ClueToggleSelectedComplete,
    ClueFocus(Option<ClueAddress>), // clue_idx when Some
    ClueFocusNext(i32),
    NewGame(Option<Difficulty>, Option<u64>), // grid rows, grid columns
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
    ChangeSettings(SettingsChange),
}
