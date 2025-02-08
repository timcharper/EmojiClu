use super::{Clue, ClueOrientation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClueWithGrouping {
    pub clue: Clue,
    pub orientation: ClueOrientation,
    pub group: usize,
    pub index: usize,
}
