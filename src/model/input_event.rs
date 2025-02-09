use super::ClueAddress;
use gtk4::gdk;
use std::time::Duration;

pub const LONG_PRESS_DURATION: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct CandidateCellTileData {
    pub row: usize,
    pub col: usize,
    pub variant: char,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct SolutionTileData {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Clickable {
    CandidateCellTile(CandidateCellTileData),
    SolutionTile(SolutionTileData),
    Clue(ClueAddress),
    Surface,
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    LeftClick(Clickable),
    TouchEvent(Clickable, Duration),
    RightClick(Clickable),
    KeyPressed(gdk::Key),
}
