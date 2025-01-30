use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Difficulty {
    Easy,
    Moderate,
    Hard,
    Veteran,
}

impl Default for Difficulty {
    fn default() -> Self {
        Difficulty::Easy
    }
}

impl Difficulty {
    pub fn grid_size(&self) -> usize {
        match self {
            Difficulty::Easy => 4,
            Difficulty::Moderate => 5,
            Difficulty::Hard => 6,
            Difficulty::Veteran => 7,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Moderate => "Moderate",
            Difficulty::Hard => "Hard",
            Difficulty::Veteran => "Veteran",
        }
        .to_string()
    }

    /// When generating clues, look this far ahead to find a solution that minimizes deductions
    pub fn look_ahead_count(&self) -> usize {
        match self {
            Difficulty::Easy => 1,
            Difficulty::Moderate => 2,
            Difficulty::Hard => 16,
            Difficulty::Veteran => 16,
        }
    }
}
