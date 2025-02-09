use super::{Clue, ClueAddress};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClueWithAddress {
    pub clue: Clue,
    pub group: usize,
    pub index: usize,
}

impl ClueWithAddress {
    pub fn new_from_address(clue: Clue, address: ClueAddress, group: usize) -> Self {
        Self {
            clue,
            group,
            index: address.index,
        }
    }

    pub fn address(&self) -> ClueAddress {
        ClueAddress {
            orientation: self.clue.orientation(),
            index: self.index,
        }
    }

    pub(crate) fn new(clue: Clue, group: usize, index: usize) -> ClueWithAddress {
        Self { clue, group, index }
    }
}
