use fluent_i18n::t;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Difficulty {
    Tutorial,
    Easy,
    Moderate,
    Hard,
    Veteran,
}

impl Default for Difficulty {
    fn default() -> Self {
        Difficulty::Tutorial
    }
}

impl Difficulty {
    pub fn all() -> Vec<Difficulty> {
        vec![
            Difficulty::Tutorial,
            Difficulty::Easy,
            Difficulty::Moderate,
            Difficulty::Hard,
            Difficulty::Veteran,
        ]
    }

    pub fn index(&self) -> usize {
        match self {
            Difficulty::Tutorial => 0,
            Difficulty::Easy => 1,
            Difficulty::Moderate => 2,
            Difficulty::Hard => 3,
            Difficulty::Veteran => 4,
        }
    }

    pub fn from_index(index: usize) -> Difficulty {
        match index {
            0 => Difficulty::Tutorial,
            1 => Difficulty::Easy,
            2 => Difficulty::Moderate,
            3 => Difficulty::Hard,
            4 => Difficulty::Veteran,
            _ => Difficulty::Easy,
        }
    }

    pub fn n_cols(&self) -> usize {
        self.grid_size()
    }

    pub fn n_rows(&self) -> usize {
        self.grid_size()
    }

    pub fn grid_size(&self) -> usize {
        match self {
            Difficulty::Tutorial => 4,
            Difficulty::Easy => 4,
            Difficulty::Moderate => 5,
            Difficulty::Hard => 6,
            Difficulty::Veteran => 8,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Difficulty::Tutorial => t!("difficulty-tutorial"),
            Difficulty::Easy => t!("difficulty-easy"),
            Difficulty::Moderate => t!("difficulty-moderate"),
            Difficulty::Hard => t!("difficulty-hard"),
            Difficulty::Veteran => t!("difficulty-veteran"),
        }
    }

    /// When generating clues, look this far ahead to find a solution that minimizes deductions
    pub fn look_ahead_count(&self) -> usize {
        match self {
            Difficulty::Tutorial => 1,
            Difficulty::Easy => 1,
            Difficulty::Moderate => 2,
            Difficulty::Hard => 16,
            Difficulty::Veteran => 16,
        }
    }
}
