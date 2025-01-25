use super::Tile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateState {
    Available,
    Eliminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    pub tile: Tile,
    pub state: CandidateState,
}

impl Candidate {
    pub fn new(tile: Tile) -> Self {
        Self {
            tile,
            state: CandidateState::Available,
        }
    }
}
