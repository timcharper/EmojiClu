use crate::{
    game::deduce_clue,
    model::{
        Clue, ClueType, Deduction, Difficulty, GameBoard, HorizontalClueType, Tile,
        VerticalClueType,
    },
};
use log::{info, trace};
use rand::{seq::IndexedRandom, Rng, RngCore};
use std::{fmt::Debug, ops::RangeInclusive};

use super::{clue_generator_state::ClueGeneratorState, solver::deduce_hidden_sets_in_row};

const MAX_BOOST: usize = 100;

#[derive(Debug, Clone)]
pub struct WeightedClueType {
    pub weight: usize,
    pub clue_type: ClueType,
}

#[derive(Debug, Clone, Copy)]
pub enum PuzzleVariantType {
    Standard,
    Narrowing,
    Striping,
}

#[derive(Debug, Clone, Copy)]
struct StandardPuzzleVariant {}
#[derive(Debug, Clone, Copy)]
struct NarrowingPuzzleVariant {
    difficulty: Difficulty,
}
#[derive(Debug, Clone, Copy)]
struct StripingPuzzleVariant {}

pub trait PuzzleVariantCloneBox {
    fn clone_box(&self) -> Box<dyn PuzzleVariant>;
}

/// unique using scan because sometimes its more efficient to do the dumb thing
fn unique_scan<T: Eq>(iter: impl Iterator<Item = T>) -> Vec<T> {
    let mut unique_items = Vec::new();

    for item in iter {
        if !unique_items.contains(&item) {
            unique_items.push(item);
        }
    }

    unique_items
}

struct ScoreBoost {
    amount: usize,
    expected_range: RangeInclusive<usize>,
    weight: usize,
}

impl ScoreBoost {
    fn multipler(&self) -> usize {
        let min = *self.expected_range.start();
        let max = *self.expected_range.end();
        let range = max - min;
        let clamped = self.amount.clamp(min, max) - min;

        (((clamped * self.weight) / range) + 1).clamp(1, MAX_BOOST)
    }
}

fn reduce_score(base_score: usize, boosts: &Vec<ScoreBoost>) -> usize {
    let overall_boost = boosts.iter().fold(1, |acc, b| acc * b.multipler());
    base_score / overall_boost
}

/// basic scoring which punishes clues for revealing too much. Lower score = preferred.
/// Basic scoring function that punishes clues for revealing too much information
///
/// # Arguments
/// * `board` - The game board state (after applying the proposed deductions)
/// * `clue` - The clue being scored
/// * `deductions` - The deductions made from this clue
///
/// # Returns
/// A score where lower values are preferred. The score is calculated as:
/// (number of deductions + (number of positive deductions * 6)) * 10
fn compute_base_score(
    board: &GameBoard,
    _: &Clue,
    deducations: &Vec<Deduction>,
) -> (usize, Vec<ScoreBoost>) {
    let n_deductions = deducations.len();
    let deduction_rows = unique_scan(deducations.iter().map(|d| d.tile.row));

    let n_rows_with_hidden_pair_deductions = deduction_rows
        .iter()
        .filter(|row| {
            let hidden_pairs = deduce_hidden_sets_in_row(&board, **row);
            hidden_pairs.len() > 0
        })
        .count();

    let n_tiles_revealed = deducations
        .iter()
        .filter(|deduction| deduction.is_positive)
        .count();

    let base_score = (n_deductions + (n_tiles_revealed * 10)) * 100;

    let boosts = vec![ScoreBoost {
        amount: n_rows_with_hidden_pair_deductions,
        expected_range: 0..=1,
        weight: 20,
    }];

    (base_score, boosts)
}

fn generic_starter_evidence(state: &mut ClueGeneratorState, init_board: &GameBoard) {
    if state.rng.random_bool(0.5) {
        let n_tiles = state.rng.random_range(1..=2);
        for _ in 0..n_tiles {
            let tile = state.random_unsolved_tile();
            state.add_selected_tile(tile);
        }
    } else {
        let starter_clue_generators = vec![
            WeightedClueType {
                weight: 1,
                clue_type: ClueType::Horizontal(HorizontalClueType::ThreeAdjacent),
            },
            WeightedClueType {
                weight: 1,
                clue_type: ClueType::Horizontal(HorizontalClueType::LeftOf),
            },
        ];
        let tiles = init_board.solution.all_tiles();

        for _ in 0..3 {
            let seed = tiles.choose(&mut state.rng).unwrap().clone();
            trace!(target: "clue_generator", "Seed: {:?}", seed);
            let clue = state
                .generate_random_clue_type(&starter_clue_generators, Some(seed))
                .unwrap();
            let deductions = deduce_clue(&state.board, &clue);
            state.add_clue(&clue, &deductions);
        }
    }
}

pub trait PuzzleVariant: Debug + PuzzleVariantCloneBox {
    fn get_clue_weights(&self) -> Vec<WeightedClueType>;

    /// score a clue based on the deductions; smaller score = better
    fn score_clue(&self, board: &GameBoard, clue: &Clue, deducations: &Vec<Deduction>) -> usize;
    fn get_variant_type(&self) -> PuzzleVariantType;
    fn populate_starter_evidence(&self, state: &mut ClueGeneratorState, init_board: &GameBoard);
}

impl<T> PuzzleVariantCloneBox for T
where
    T: 'static + PuzzleVariant + Clone,
{
    fn clone_box(&self) -> Box<dyn PuzzleVariant> {
        Box::new(self.clone())
    }
}

/// standard puzzle experience, mix of clues
impl PuzzleVariant for StandardPuzzleVariant {
    fn get_clue_weights(&self) -> Vec<WeightedClueType> {
        vec![
            WeightedClueType {
                weight: 3,
                clue_type: ClueType::Horizontal(HorizontalClueType::TwoAdjacent),
            },
            WeightedClueType {
                weight: 4,
                clue_type: ClueType::Horizontal(HorizontalClueType::TwoApartNotMiddle),
            },
            WeightedClueType {
                weight: 2,
                clue_type: ClueType::Horizontal(HorizontalClueType::NotAdjacent),
            },
            WeightedClueType {
                weight: 1,
                clue_type: ClueType::Horizontal(HorizontalClueType::LeftOf),
            },
            WeightedClueType {
                weight: 6,
                clue_type: ClueType::Vertical(VerticalClueType::TwoInColumn),
            },
            WeightedClueType {
                weight: 2,
                clue_type: ClueType::Vertical(VerticalClueType::NotInSameColumn),
            },
            WeightedClueType {
                weight: 2,
                clue_type: ClueType::Vertical(VerticalClueType::OneMatchesEither),
            },
            WeightedClueType {
                weight: 6,
                clue_type: ClueType::Horizontal(HorizontalClueType::ThreeAdjacent),
            },
        ]
    }

    fn score_clue(&self, board: &GameBoard, clue: &Clue, deducations: &Vec<Deduction>) -> usize {
        let (base_score, boosts) = compute_base_score(board, clue, deducations);
        reduce_score(base_score, &boosts)
    }

    fn get_variant_type(&self) -> PuzzleVariantType {
        PuzzleVariantType::Standard
    }

    fn populate_starter_evidence(&self, state: &mut ClueGeneratorState, init_board: &GameBoard) {
        generic_starter_evidence(state, init_board);
    }
}

impl PuzzleVariant for NarrowingPuzzleVariant {
    fn get_clue_weights(&self) -> Vec<WeightedClueType> {
        vec![
            WeightedClueType {
                weight: 3,
                clue_type: ClueType::Horizontal(HorizontalClueType::TwoAdjacent),
            },
            WeightedClueType {
                weight: 1,
                clue_type: ClueType::Horizontal(HorizontalClueType::TwoApartNotMiddle),
            },
            WeightedClueType {
                weight: 4,
                clue_type: ClueType::Horizontal(HorizontalClueType::NotAdjacent),
            },
            WeightedClueType {
                weight: 6,
                clue_type: ClueType::Horizontal(HorizontalClueType::LeftOf),
            },
            WeightedClueType {
                weight: 3,
                clue_type: ClueType::Vertical(VerticalClueType::TwoInColumn),
            },
            WeightedClueType {
                weight: 4,
                clue_type: ClueType::Vertical(VerticalClueType::NotInSameColumn),
            },
            WeightedClueType {
                weight: 2,
                clue_type: ClueType::Vertical(VerticalClueType::OneMatchesEither),
            },
            WeightedClueType {
                weight: 1,
                clue_type: ClueType::Horizontal(HorizontalClueType::ThreeAdjacent),
            },
        ]
    }

    fn score_clue(&self, board: &GameBoard, clue: &Clue, deducations: &Vec<Deduction>) -> usize {
        let (base_score, mut boosts) = compute_base_score(board, clue, deducations);

        let middle_col = self.difficulty.grid_size() as f32 / 2.0;
        // take the average distance of deductions from the center
        // and use that to boost the score
        let avg_distance_from_center = deducations
            .iter()
            .map(|d| (d.column as f32 - middle_col).abs())
            .sum::<f32>()
            / deducations.len() as f32;

        boosts.push(ScoreBoost {
            amount: avg_distance_from_center as usize,
            expected_range: 0..=((board.solution.n_variants - 1) / 2),
            weight: 1,
        });

        reduce_score(base_score, &boosts)
    }

    fn get_variant_type(&self) -> PuzzleVariantType {
        PuzzleVariantType::Narrowing
    }

    fn populate_starter_evidence(&self, state: &mut ClueGeneratorState, init_board: &GameBoard) {
        generic_starter_evidence(state, init_board);
    }
}

/// try and "stripe" the puzzle by eliminating event and odd columns
impl PuzzleVariant for StripingPuzzleVariant {
    fn get_clue_weights(&self) -> Vec<WeightedClueType> {
        vec![
            WeightedClueType {
                weight: 4,
                clue_type: ClueType::Horizontal(HorizontalClueType::TwoAdjacent),
            },
            WeightedClueType {
                weight: 2,
                clue_type: ClueType::Horizontal(HorizontalClueType::TwoApartNotMiddle),
            },
            WeightedClueType {
                weight: 1,
                clue_type: ClueType::Horizontal(HorizontalClueType::NotAdjacent),
            },
            WeightedClueType {
                weight: 1,
                clue_type: ClueType::Horizontal(HorizontalClueType::LeftOf),
            },
            WeightedClueType {
                weight: 1,
                clue_type: ClueType::Vertical(VerticalClueType::TwoInColumn),
            },
            WeightedClueType {
                weight: 1,
                clue_type: ClueType::Vertical(VerticalClueType::NotInSameColumn),
            },
            WeightedClueType {
                weight: 1,
                clue_type: ClueType::Vertical(VerticalClueType::OneMatchesEither),
            },
            WeightedClueType {
                weight: 2,
                clue_type: ClueType::Horizontal(HorizontalClueType::ThreeAdjacent),
            },
        ]
    }

    fn score_clue(&self, board: &GameBoard, clue: &Clue, deductions: &Vec<Deduction>) -> usize {
        let (base_score, mut boosts) = compute_base_score(board, clue, deductions);

        // if this advances the board state towards striping, boost clue
        let deduced_tiles = unique_scan(deductions.iter().map(|d| d.tile));

        let max_striped_cols = deduced_tiles
            .iter()
            .map(|tile| {
                let deduced_columns = (0..board.solution.n_variants)
                    .filter(|col| board.has_negative_deduction(&tile, *col))
                    .collect::<Vec<usize>>();

                if deduced_columns.len() <= 1 {
                    return 0;
                }

                let all_even = deduced_columns.iter().all(|&col| col % 2 == 0);
                let all_odd = deduced_columns.iter().all(|&col| col % 2 == 1);

                if all_even || all_odd {
                    return deduced_columns.len() - 1 /* 2 deduced columns counts as 1 striping */;
                } else {
                    return 0;
                }
            })
            .max()
            .unwrap_or(0);

        boosts.push(ScoreBoost {
            amount: max_striped_cols,
            expected_range: 0..=(board.solution.n_variants / 2),
            weight: 1,
        });

        reduce_score(base_score, &boosts)
    }

    fn get_variant_type(&self) -> PuzzleVariantType {
        PuzzleVariantType::Striping
    }

    /// select a tile and then add a three-in-a-row clue with it that can go either direction
    fn populate_starter_evidence(&self, state: &mut ClueGeneratorState, init_board: &GameBoard) {
        let randomly_chosen_rows = (0..init_board.solution.n_variants)
            .map(|_| state.rng.random_range(0..init_board.solution.n_rows))
            .collect::<Vec<usize>>();

        // select a tile in the middle
        // cols 4: ((4 - 1) / 2) = col 1
        // cols 5: ((5 - 1) / 2) = col 2
        // cols 6: ((6 - 1) / 2) = col 2
        let middle_col_min = ((init_board.solution.n_variants as i32 + 1) / 2) - 1 /* 1 based to 0 based */;

        info!(
            target: "clue_generator",
            "middle_col_min: {:?}; randomly_chosen_rows: {:?}",
            middle_col_min, randomly_chosen_rows
        );

        let selected_col = middle_col_min + state.rng.random_range(0..=1);
        let selected_row = randomly_chosen_rows[selected_col as usize];
        let seed_tile = init_board.solution.get(selected_row, selected_col as usize);

        let selected_col = selected_col as i32;
        state.add_selected_tile(seed_tile);

        let possible_offsets: Vec<i32> = vec![-2, -1, 0]
            .into_iter()
            .filter(|&offset| {
                let left_col = selected_col + offset;
                let right_col = selected_col + offset + 2;

                left_col >= 0 && right_col < init_board.solution.n_variants as i32
            })
            .collect::<Vec<i32>>();
        let random_offset = possible_offsets.choose(&mut state.rng).unwrap();

        let left_col = selected_col + random_offset;
        let right_col = selected_col + random_offset + 2;

        let tiles = (left_col..=right_col)
            .map(|col| {
                init_board
                    .solution
                    .get(randomly_chosen_rows[col as usize], col as usize)
            })
            .collect::<Vec<Tile>>();

        let clue = Clue::three_adjacent(tiles[0], tiles[1], tiles[2]);
        let deductions = deduce_clue(&state.board, &clue);
        state.add_clue(&clue, &deductions);
    }
}

pub fn random_puzzle_variant(
    difficulty: Difficulty,
    rng: &mut Box<dyn RngCore>,
) -> Box<dyn PuzzleVariant> {
    let puzzle_variants: Vec<(Box<dyn PuzzleVariant>, i32)> = vec![
        (Box::new(StandardPuzzleVariant {}), 3),
        (Box::new(NarrowingPuzzleVariant { difficulty }), 1),
        (Box::new(StripingPuzzleVariant {}), 3),
    ];
    let lol = puzzle_variants
        .choose_weighted(rng, |(_, weight)| *weight)
        .map(|(variant, _)| variant)
        .expect("No puzzle variant chosen");
    lol.clone_box()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_boost_multiplier_maximum() {
        let boost = ScoreBoost {
            amount: 10,
            expected_range: 0..=10,
            weight: MAX_BOOST * 10,
        };
        assert_eq!(
            boost.multipler(),
            MAX_BOOST,
            "Maximum value should be clamped to MAX_BOOST"
        );
    }

    #[test]
    fn test_score_boost_multiplier_non_zero_range() {
        let mut boost = ScoreBoost {
            amount: 4,
            expected_range: 2..=6,
            weight: 4,
        };
        assert_eq!(
            boost.multipler(),
            3,
            "Multiplier should be 3 (middle point)"
        );

        boost.amount = 10;
        assert_eq!(
            boost.multipler(),
            5,
            "Multiplier should be 4 (clamped, weight + 1)"
        );

        boost.amount = 0;
        assert_eq!(
            boost.multipler(),
            1,
            "Multiplier should be 1 (clamped at minimum)"
        );

        boost.amount = 6;
        assert_eq!(
            boost.multipler(),
            5,
            "Multiplier should be 5 (max, weight + 1)"
        );
    }

    #[test]
    fn test_reduce_score_boosts_zero() {
        let base_score = 100;
        let boosts = vec![
            ScoreBoost {
                amount: 0,
                expected_range: 0..=10,
                weight: 10,
            },
            ScoreBoost {
                amount: 0,
                expected_range: 0..=10,
                weight: 10,
            },
        ];
        let reduced_score = reduce_score(base_score, &boosts);
        assert_eq!(reduced_score, 100, "Reduced score should be 100");
    }

    #[test]
    fn test_reduce_score_boosts_non_zero() {
        let base_score = 100;
        let boosts = vec![
            ScoreBoost {
                amount: 10,
                expected_range: 0..=10,
                weight: 1,
            },
            ScoreBoost {
                amount: 10,
                expected_range: 0..=10,
                weight: 1,
            },
        ];
        let reduced_score = reduce_score(base_score, &boosts);
        assert_eq!(reduced_score, 25, "Reduced score should be 25");
    }

    #[test]
    fn test_reduce_score_boosts_uneven_weights() {
        let base_score = 100;
        let boosts = vec![
            ScoreBoost {
                amount: 10,
                expected_range: 0..=10,
                weight: 1,
            },
            ScoreBoost {
                amount: 10,
                expected_range: 0..=10,
                weight: 4,
            },
        ];
        let reduced_score = reduce_score(base_score, &boosts);
        assert_eq!(
            reduced_score, 10,
            "Reduced score should be 10 (100 / 2 / 5)"
        );
    }
}
