use std::fmt::{self, Debug};

#[cfg(test)]
use super::tile::Tile;

use crate::model::tile_assertion::TileAssertion;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeductionKind {
    // Simple deduction where a tile simply isn't possible
    Constraint,
    // More complicated deduction because all possible solutions converge on a single cell
    Converging,
    // Last remaining tile in a row/column
    LastRemaining,
    HiddenSet,
}

impl DeductionKind {
    #[cfg(test)]
    pub fn from_str(input: &str) -> Option<Self> {
        match input {
            "Constraint" => Some(Self::Constraint),
            "Converging" => Some(Self::Converging),
            "LastRemaining" => Some(Self::LastRemaining),
            "HiddenSet" => Some(Self::HiddenSet),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn to_string(&self) -> &str {
        match self {
            Self::Constraint => "Constraint",
            Self::Converging => "Converging",
            Self::LastRemaining => "LastRemaining",
            Self::HiddenSet => "HiddenSet",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Deduction {
    pub column: usize,
    pub tile_assertion: TileAssertion,
    pub deduction_kind: Option<DeductionKind>,
}

impl Debug for Deduction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(kind) = &self.deduction_kind {
            write!(
                f,
                "{} {} col {} ({:?})",
                self.tile_assertion.tile,
                if self.tile_assertion.assertion {
                    "is"
                } else {
                    "not"
                },
                self.column,
                kind
            )
        } else {
            write!(
                f,
                "{} {} col {}",
                self.tile_assertion.tile,
                if self.tile_assertion.assertion {
                    "is"
                } else {
                    "not"
                },
                self.column
            )
        }
    }
}

impl Deduction {
    pub fn new(column: usize, tile_assertion: TileAssertion) -> Self {
        Self {
            column,
            tile_assertion,
            deduction_kind: None,
        }
    }

    pub fn new_with_kind(
        column: usize,
        tile_assertion: TileAssertion,
        deduction_kind: DeductionKind,
    ) -> Self {
        Self {
            column,
            tile_assertion,
            deduction_kind: Some(deduction_kind),
        }
    }

    #[cfg(test)]
    pub fn parse(input: &str) -> Self {
        // Split input into parts, excluding the DeductionKind in parentheses
        let parts: Vec<&str> = input
            .split_whitespace()
            .filter(|&part| !part.starts_with('(') && !part.ends_with(')'))
            .collect();

        // Need at least 4 parts: "1a", "not"/"is", "col", "0"
        if parts.len() != 4 || parts[2] != "col" {
            panic!("Invalid deduction: {}", input);
        }

        // Parse tile from first part (e.g. "1a")
        if parts[0].len() < 2 {
            panic!("Invalid deduction: {}", input);
        }
        let row = parts[0][0..1]
            .parse::<usize>()
            .expect(&format!("Invalid row in {}", input));
        let variant = parts[0]
            .chars()
            .nth(1)
            .expect(&format!("Invalid variant in {}", input));
        let tile = Tile::new(row, variant);

        // Parse column number
        let column = parts[3]
            .parse::<usize>()
            .expect(&format!("Invalid column in {}", input));

        // Check for DeductionKind in parentheses
        let kind = if let Some(start) = input.find('(') {
            if let Some(end) = input.find(')') {
                let kind_str = &input[start + 1..end];
                DeductionKind::from_str(kind_str)
            } else {
                None
            }
        } else {
            None
        };

        match parts[1] {
            "not" => Deduction {
                column,
                tile_assertion: TileAssertion {
                    tile,
                    assertion: false,
                },
                deduction_kind: kind,
            },
            "is" => Deduction {
                column,
                tile_assertion: TileAssertion {
                    tile,
                    assertion: true,
                },
                deduction_kind: kind,
            },
            _ => panic!("Invalid deduction: {}", input),
        }
    }

    pub fn is_positive(&self) -> bool {
        self.tile_assertion.assertion
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tile::Tile;

    #[test]
    fn test_parse_with_deduction_kind() {
        let input = "1a is col 2 (LastRemaining)";
        let deduction = Deduction::parse(input);
        assert_eq!(deduction.column, 2);
        assert_eq!(deduction.tile_assertion.tile, Tile::new(1, 'a'));
        assert_eq!(deduction.tile_assertion.assertion, true);
        assert_eq!(deduction.deduction_kind, Some(DeductionKind::LastRemaining));
    }

    #[test]
    fn test_parse_without_deduction_kind() {
        let input = "1a not col 2";
        let deduction = Deduction::parse(input);
        assert_eq!(deduction.column, 2);
        assert_eq!(deduction.tile_assertion.tile, Tile::new(1, 'a'));
        assert_eq!(deduction.tile_assertion.assertion, false);
        assert_eq!(deduction.deduction_kind, None);
    }

    #[test]
    fn test_debug_representation() {
        let deduction_with_kind = Deduction::new_with_kind(
            2,
            TileAssertion {
                tile: Tile::new(1, 'a'),
                assertion: true,
            },
            DeductionKind::LastRemaining,
        );
        let deduction_without_kind = Deduction::new(
            2,
            TileAssertion {
                tile: Tile::new(1, 'a'),
                assertion: false,
            },
        );

        assert_eq!(
            format!("{:?}", deduction_with_kind),
            "1a is col 2 (LastRemaining)"
        );
        assert_eq!(format!("{:?}", deduction_without_kind), "1a not col 2");
    }
}
