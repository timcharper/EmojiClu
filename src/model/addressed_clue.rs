use super::{Clue, ClueAddress};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClueWithAddress {
    pub clue: Clue,
    pub group: usize,
    pub address: ClueAddress,
}

impl ClueWithAddress {
    pub fn new(clue: Clue, address: ClueAddress, group: usize) -> Self {
        Self {
            clue,
            address,
            group,
        }
    }

    pub fn address(&self) -> ClueAddress {
        self.address
    }
}
