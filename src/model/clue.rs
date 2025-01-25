use std::{collections::HashSet, fmt::Debug};

use log::warn;

use crate::model::tile::Tile;

use super::TileAssertion;

// horiz sort index
const SORT_INDEX_THREE_ADJACENT: usize = 0;
const SORT_INDEX_TWO_APART_NOT_MIDDLE: usize = 1;
const SORT_INDEX_LEFT_OF: usize = 2;
const SORT_INDEX_TWO_ADJACENT: usize = 3;
const SORT_INDEX_NOT_ADJACENT: usize = 4;

// vert sort index
const SORT_INDEX_THREE_IN_COLUMN: usize = 0;
const SORT_INDEX_TWO_IN_COLUMN: usize = 1;
const SORT_INDEX_TWO_IN_COLUMN_ONE_NOT: usize = 2;
const SORT_INDEX_NOT_IN_SAME_COLUMN: usize = 3;
const SORT_INDEX_ONE_MATCHES_EITHER: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Copy)]
pub enum HorizontalClueType {
    ThreeAdjacent,     // ABC, either order
    TwoApartNotMiddle, // A, not B, C
    LeftOf,            // A <- B
    TwoAdjacent,       // A next to B
    NotAdjacent,       // A not next to B
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Copy)]
pub enum VerticalClueType {
    ThreeInColumn,      // Three tiles in same column
    TwoInColumn,        // Two tiles in same column
    OneMatchesEither,   // First tile matches column of either second or third, not both
    NotInSameColumn,    // First tile not in same column as second
    TwoInColumnWithout, // Two tiles in same column, one not
}

#[derive(Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Clue {
    pub clue_type: ClueType,
    pub assertions: Vec<TileAssertion>,
    pub sort_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Copy)]
pub enum ClueType {
    Horizontal(HorizontalClueType),
    Vertical(VerticalClueType),
}

impl ClueType {
    pub fn get_title(&self) -> String {
        match self {
            ClueType::Horizontal(hor) => match hor {
                HorizontalClueType::ThreeAdjacent => "Three Adjacent".to_string(),
                HorizontalClueType::TwoApartNotMiddle => {
                    "Two Apart, But Not The Middle".to_string()
                }
                HorizontalClueType::LeftOf => "Left Of".to_string(),
                HorizontalClueType::TwoAdjacent => "Two Adjacent".to_string(),
                HorizontalClueType::NotAdjacent => "Not Adjacent".to_string(),
            },
            ClueType::Vertical(vert) => match vert {
                VerticalClueType::ThreeInColumn => "All In Column".to_string(),
                VerticalClueType::TwoInColumn => "Two In Column".to_string(),
                VerticalClueType::OneMatchesEither => "One Matches Either".to_string(),
                VerticalClueType::NotInSameColumn => "Not In Same Column".to_string(),
                VerticalClueType::TwoInColumnWithout => "Two In Column, One Not".to_string(),
            },
        }
    }
}

impl Clue {
    pub fn concrete_tiles_iter(&self) -> impl Iterator<Item = &Tile> {
        self.assertions
            .iter()
            .filter(|a| a.assertion)
            .map(|a| &a.tile)
    }

    fn new_with_assertions(
        clue_type: ClueType,
        assertions: Vec<TileAssertion>,
        sort_index: usize,
    ) -> Self {
        Self {
            clue_type,
            assertions,
            sort_index,
        }
    }

    pub fn three_adjacent(t1: Tile, t2: Tile, t3: Tile) -> Self {
        Self::new_with_assertions(
            ClueType::Horizontal(HorizontalClueType::ThreeAdjacent),
            vec![t1, t2, t3]
                .into_iter()
                .map(|t| TileAssertion {
                    tile: t,
                    assertion: true,
                })
                .collect(),
            SORT_INDEX_THREE_ADJACENT,
        )
    }

    pub fn two_apart_not_middle(t1: Tile, not_middle: Tile, t2: Tile) -> Self {
        Self::new_with_assertions(
            ClueType::Horizontal(HorizontalClueType::TwoApartNotMiddle),
            vec![
                TileAssertion {
                    tile: t1,
                    assertion: true,
                },
                TileAssertion {
                    tile: not_middle,
                    assertion: false,
                },
                TileAssertion {
                    tile: t2,
                    assertion: true,
                },
            ],
            SORT_INDEX_TWO_APART_NOT_MIDDLE,
        )
    }

    pub fn left_of(left: Tile, right: Tile) -> Self {
        Self::new_with_assertions(
            ClueType::Horizontal(HorizontalClueType::LeftOf),
            vec![left, right]
                .into_iter()
                .map(|t| TileAssertion {
                    tile: t,
                    assertion: true,
                })
                .collect(),
            SORT_INDEX_LEFT_OF,
        )
    }

    pub fn adjacent(t1: Tile, t2: Tile) -> Self {
        Self::new_with_assertions(
            ClueType::Horizontal(HorizontalClueType::TwoAdjacent),
            vec![
                TileAssertion {
                    tile: t1,
                    assertion: true,
                },
                TileAssertion {
                    tile: t2,
                    assertion: true,
                },
            ],
            SORT_INDEX_TWO_ADJACENT,
        )
    }

    pub fn not_adjacent(tile: Tile, not_next_to: Tile) -> Self {
        Self::new_with_assertions(
            ClueType::Horizontal(HorizontalClueType::NotAdjacent),
            vec![
                TileAssertion {
                    tile: tile,
                    assertion: true,
                },
                TileAssertion {
                    tile: not_next_to,
                    assertion: false,
                },
            ],
            SORT_INDEX_NOT_ADJACENT,
        )
    }

    pub fn three_in_column(t1: Tile, t2: Tile, t3: Tile) -> Self {
        assert_ne!(t1.row, t2.row, "Tiles must be in different rows");
        assert_ne!(t1.row, t3.row, "Tiles must be in different rows");
        assert_ne!(t2.row, t3.row, "Tiles must be in different rows");
        let mut assertions = vec![
            TileAssertion {
                tile: t1,
                assertion: true,
            },
            TileAssertion {
                tile: t2,
                assertion: true,
            },
            TileAssertion {
                tile: t3,
                assertion: true,
            },
        ];
        assertions.sort_by(|a, b| a.tile.row.cmp(&b.tile.row));
        Self::new_with_assertions(
            ClueType::Vertical(VerticalClueType::ThreeInColumn),
            assertions,
            SORT_INDEX_THREE_IN_COLUMN,
        )
    }

    pub fn two_in_column(t1: Tile, t2: Tile) -> Self {
        assert_ne!(t1.row, t2.row, "Tiles must be in different rows");
        let mut assertions = vec![
            TileAssertion {
                tile: t1,
                assertion: true,
            },
            TileAssertion {
                tile: t2,
                assertion: true,
            },
        ];
        assertions.sort_by(|a, b| a.tile.row.cmp(&b.tile.row));
        Self::new_with_assertions(
            ClueType::Vertical(VerticalClueType::TwoInColumn),
            assertions,
            SORT_INDEX_TWO_IN_COLUMN,
        )
    }

    pub fn two_in_column_without(t1: Tile, not_between: Tile, t2: Tile) -> Self {
        assert_ne!(t1.row, t2.row, "Tiles must be in different rows");
        assert_ne!(t1.row, not_between.row, "Tiles must be in different rows");
        assert_ne!(t2.row, not_between.row, "Tiles must be in different rows");
        let mut assertions = vec![
            TileAssertion {
                tile: t1,
                assertion: true,
            },
            TileAssertion {
                tile: not_between,
                assertion: false,
            },
            TileAssertion {
                tile: t2,
                assertion: true,
            },
        ];
        assertions.sort_by(|a, b| a.tile.row.cmp(&b.tile.row));
        Self::new_with_assertions(
            ClueType::Vertical(VerticalClueType::TwoInColumnWithout),
            assertions,
            SORT_INDEX_TWO_IN_COLUMN_ONE_NOT,
        )
    }

    pub fn two_not_in_same_column(seed: Tile, not_tile: Tile) -> Self {
        Self::new_with_assertions(
            ClueType::Vertical(VerticalClueType::NotInSameColumn),
            vec![
                TileAssertion {
                    tile: seed,
                    assertion: true,
                },
                TileAssertion {
                    tile: not_tile,
                    assertion: false,
                },
            ],
            SORT_INDEX_NOT_IN_SAME_COLUMN,
        )
    }

    pub fn one_matches_either(target: Tile, option1: Tile, option2: Tile) -> Self {
        assert_ne!(target.row, option1.row, "Tiles must be in different rows");
        assert_ne!(target.row, option2.row, "Tiles must be in different rows");
        assert_ne!(option1.row, option2.row, "Tiles must be in different rows");
        Self::new_with_assertions(
            ClueType::Vertical(VerticalClueType::OneMatchesEither),
            vec![target, option1, option2]
                .into_iter()
                .map(|t| TileAssertion {
                    tile: t,
                    assertion: true,
                })
                .collect(),
            SORT_INDEX_ONE_MATCHES_EITHER,
        )
    }

    pub fn intersects_positive(&self, other: &Self) -> bool {
        if self.clue_type == ClueType::Vertical(VerticalClueType::OneMatchesEither)
            || other.clue_type == ClueType::Vertical(VerticalClueType::OneMatchesEither)
        {
            return false;
        }

        let other_concrete_tiles: Vec<&Tile> = other.concrete_tiles_iter().collect();
        self.concrete_tiles_iter()
            .any(|tile| other_concrete_tiles.contains(&tile))
    }

    pub fn non_singleton_intersects(&self, clue: &Self) -> bool {
        if self.is_horizontal() != clue.is_horizontal() {
            return false;
        }
        let other_concrete_tiles: Vec<&Tile> = clue.concrete_tiles_iter().collect();
        self.concrete_tiles_iter()
            .filter(|tile| other_concrete_tiles.contains(tile))
            .count()
            > 1
    }

    pub fn to_string(&self) -> String {
        match &self.clue_type {
            ClueType::Horizontal(h_type) => match h_type {
                HorizontalClueType::LeftOf => {
                    format!(
                        "{}...{}",
                        self.assertions[0].tile.to_string(),
                        self.assertions[1].tile.to_string()
                    )
                }
                _ => {
                    let assertions = self
                        .assertions
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<String>>()
                        .join(",");
                    format!("<{}>", assertions)
                }
            },
            ClueType::Vertical(v_type) => match v_type {
                VerticalClueType::OneMatchesEither => {
                    format!(
                        "{} is in the same column as exactly one of {} or {}",
                        self.assertions[0].tile.to_string(),
                        self.assertions[1].tile.to_string(),
                        self.assertions[2].tile.to_string()
                    )
                }
                _ => {
                    let assertions = self
                        .assertions
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<String>>()
                        .join(",");
                    format!("|{}|", assertions)
                }
            },
        }
    }

    pub fn is_vertical(&self) -> bool {
        matches!(self.clue_type, ClueType::Vertical(_))
    }

    pub fn is_horizontal(&self) -> bool {
        matches!(self.clue_type, ClueType::Horizontal(_))
    }

    pub(crate) fn merge(&self, other: &Self) -> Option<Vec<Self>> {
        // we only merge clues vertical clues
        if !self.is_vertical() || !other.is_vertical() {
            return None;
        }

        if !self.intersects_positive(other) {
            return None;
        }

        let mut clues = vec![self, other];
        clues.sort_by(|c, other| c.clue_type.cmp(&other.clue_type));
        let clue_types = [clues[0].clue_type, clues[1].clue_type];

        if clue_types
            == [
                ClueType::Vertical(VerticalClueType::TwoInColumn),
                ClueType::Vertical(VerticalClueType::TwoInColumn),
            ]
        {
            let mut tiles_set = HashSet::new();
            tiles_set.extend(self.assertions.iter().map(|a| a.tile));
            tiles_set.extend(other.assertions.iter().map(|a| a.tile));

            match tiles_set.len() {
                3 => {
                    let tiles: Vec<Tile> = tiles_set.into_iter().collect();
                    Some(vec![Clue::three_in_column(tiles[0], tiles[1], tiles[2])])
                }
                2 => {
                    // they are the same
                    Some(vec![clues[0].clone()])
                }
                _ => {
                    // something went wrong
                    return None;
                }
            }
        } else if clue_types
            == [
                ClueType::Vertical(VerticalClueType::TwoInColumn),
                ClueType::Vertical(VerticalClueType::NotInSameColumn),
            ]
        {
            let positive_assertions = &clues[0].assertions;
            assert!(
                positive_assertions.len() == 2,
                "Something went wrong; TwoInColumn clue has more than 2 assertions"
            );
            let negative_assertions = &clues[1].assertions[1];
            assert!(
                !negative_assertions.assertion,
                "Something went wrong; Positive assertion found in second slot of negative clue"
            );
            Some(vec![Clue::two_in_column_without(
                positive_assertions[0].tile,
                negative_assertions.tile,
                positive_assertions[1].tile,
            )])
        } else if clue_types
            == [
                ClueType::Vertical(VerticalClueType::TwoInColumn),
                ClueType::Vertical(VerticalClueType::TwoInColumnWithout),
            ]
        {
            let positive_assertions_1 = &clues[0].assertions;
            assert!(
                positive_assertions_1.len() == 2,
                "Something went wrong; TwoInColumn clue has more than 2 positive assertions"
            );

            let negative_assertions_2: Vec<TileAssertion> = clues[1]
                .assertions
                .iter()
                .filter(|a| !a.assertion)
                .cloned()
                .collect();
            assert!(
                negative_assertions_2.len() == 1,
                "Something went wrong; TwoInColumnWithout clue has more than 1 negative assertions"
            );
            let positive_assertions_2: Vec<TileAssertion> = clues[1]
                .assertions
                .iter()
                .filter(|a| a.assertion)
                .cloned()
                .collect();

            let mut all_assertions =
                [positive_assertions_1.clone(), positive_assertions_2].concat();
            all_assertions.sort();
            all_assertions.dedup();

            if all_assertions.len() != 3 {
                // something went wrong
                warn!(
                    "Something went wrong; Total unique tiles between intersecting clues TwoInColumn and TwoInColumnWithout resulted in {} assertions",
                    all_assertions.len()
                );
                return None;
            }

            Some(vec![
                Clue::three_in_column(
                    all_assertions[0].tile,
                    all_assertions[1].tile,
                    all_assertions[2].tile,
                ),
                Clue::two_not_in_same_column(all_assertions[0].tile, negative_assertions_2[0].tile),
            ])
        } else {
            None
        }
    }

    /// only modifies vertical clues without negative assertions
    /// None means the clue should be removed
    pub(crate) fn without_negative_assertions(&self) -> Option<Clue> {
        match self.clue_type {
            ClueType::Vertical(VerticalClueType::TwoInColumnWithout) => {
                let positive_assertions: Vec<&TileAssertion> =
                    self.assertions.iter().filter(|a| a.assertion).collect();

                if positive_assertions.len() != 2 {
                    return None;
                }

                Some(Clue::two_in_column(
                    positive_assertions[0].tile,
                    positive_assertions[1].tile,
                ))
            }
            ClueType::Vertical(VerticalClueType::NotInSameColumn) => None,
            _ => Some(self.clone()),
        }
    }
}

impl Debug for Clue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_two_in_column() {
        let clue1 = Clue::two_in_column(Tile::parse("0a"), Tile::parse("1a"));
        let clue2 = Clue::two_in_column(Tile::parse("1a"), Tile::parse("2a"));

        let merged = clue1.merge(&clue2);
        assert!(merged.is_some());
        let merged = merged.unwrap();

        // Should merge into a three_in_column clue
        assert!(matches!(
            merged[0].clue_type,
            ClueType::Vertical(VerticalClueType::ThreeInColumn)
        ));
        assert_eq!(merged[0].assertions.len(), 3);
        assert!(merged[0].assertions.iter().all(|a| a.assertion)); // All assertions should be positive

        // Check that all original tiles are present
        let tiles: Vec<_> = merged[0].assertions.iter().map(|a| a.tile).collect();
        assert!(tiles.contains(&Tile::parse("0a")));
        assert!(tiles.contains(&Tile::parse("1a")));
        assert!(tiles.contains(&Tile::parse("2a")));
    }

    #[test]
    fn test_merge_two_not_in_column() {
        let clue1 = Clue::two_not_in_same_column(Tile::parse("0a"), Tile::parse("1a"));
        let clue2 = Clue::two_not_in_same_column(Tile::parse("1a"), Tile::parse("2a"));

        let merged = clue1.merge(&clue2);
        // Should not merge since they have negative assertions
        assert!(merged.is_none());
    }

    #[test]
    fn test_merge_two_in_column_and_two_not_in_same_column() {
        let clue1 = Clue::two_in_column(Tile::parse("0a"), Tile::parse("2a"));
        let clue2 = Clue::two_not_in_same_column(Tile::parse("0a"), Tile::parse("1b"));

        let merged = clue1.merge(&clue2);
        // Should not merge since one has a negative assertion
        assert!(merged.is_some());
        let merged = merged.unwrap();
        assert_eq!(merged.len(), 1);
        assert!(matches!(
            merged[0].clue_type,
            ClueType::Vertical(VerticalClueType::TwoInColumnWithout)
        ));
        assert_eq!(merged[0].assertions.len(), 3);
        assert_eq!(merged[0].assertions[0].tile, Tile::parse("0a"));
        assert_eq!(merged[0].assertions[1].tile, Tile::parse("1b"));
        assert_eq!(merged[0].assertions[2].tile, Tile::parse("2a"));

        assert_eq!(merged[0].assertions[0].assertion, true);
        assert_eq!(merged[0].assertions[1].assertion, false);
        assert_eq!(merged[0].assertions[2].assertion, true);
    }

    #[test]
    fn test_merge_horizontal_clues() {
        let clue1 = Clue::adjacent(Tile::parse("0a"), Tile::parse("0b"));
        let clue2 = Clue::adjacent(Tile::parse("0b"), Tile::parse("0c"));

        let merged = clue1.merge(&clue2);
        // Should not merge since they are horizontal
        assert!(merged.is_none());
    }

    #[test]
    fn test_merge_different_lengths() {
        let clue1 = Clue::two_in_column(Tile::parse("0a"), Tile::parse("1a"));
        let clue2 = Clue::three_in_column(Tile::parse("0a"), Tile::parse("1a"), Tile::parse("2a"));

        let merged = clue1.merge(&clue2);
        // Should not merge since they have different lengths
        assert!(merged.is_none());
    }

    #[test]
    fn test_merge_not_enough_unique_tiles() {
        let clue1 = Clue::two_in_column(Tile::parse("0a"), Tile::parse("1a"));
        let clue2 = Clue::two_in_column(Tile::parse("0a"), Tile::parse("1a"));

        let merged = clue1.merge(&clue2);
        // Just return the same clue because they are identical
        assert!(merged.is_some());
        let merged = merged.unwrap();
        assert_eq!(merged.len(), 1);
        assert!(matches!(
            merged[0].clue_type,
            ClueType::Vertical(VerticalClueType::TwoInColumn)
        ));
        assert_eq!(merged[0].assertions.len(), 2);
    }

    #[test]
    fn test_merge_two_in_column_and_two_in_column_without() {
        let clue1 = Clue::two_in_column(Tile::parse("0a"), Tile::parse("5a"));
        let clue2 =
            Clue::two_in_column_without(Tile::parse("0a"), Tile::parse("2a"), Tile::parse("3a"));

        let merged = clue1.merge(&clue2);
        assert!(merged.is_some());
        let merged = merged.unwrap();
        assert_eq!(merged.len(), 2);
        assert!(matches!(
            merged[0].clue_type,
            ClueType::Vertical(VerticalClueType::ThreeInColumn)
        ));
        assert_eq!(merged[0].assertions.len(), 3);

        assert_eq!(
            merged[1].clue_type,
            ClueType::Vertical(VerticalClueType::NotInSameColumn)
        );
        assert_eq!(merged[1].assertions.len(), 2);

        assert_eq!(merged[0].assertions[0].tile, Tile::parse("0a"));
        assert_eq!(merged[0].assertions[1].tile, Tile::parse("3a"));
        assert_eq!(merged[0].assertions[2].tile, Tile::parse("5a"));

        assert_eq!(merged[1].assertions[0].tile, Tile::parse("0a"));
        assert_eq!(merged[1].assertions[1].tile, Tile::parse("2a"));
    }
}
