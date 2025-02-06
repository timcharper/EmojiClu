use crate::{
    game::deduce_clue,
    model::{
        Clue, ClueType, Deduction, Difficulty, GameBoard, HorizontalClueType, Tile,
        VerticalClueType,
    },
};
use itertools::Itertools;
use log::{info, trace};
use rand::{seq::IndexedRandom, Rng, RngCore};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
};

use super::clue_generator_state::ClueGeneratorState;

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

/// basic scoring which punishes clues for revealing too much. Lower score = preferred.
fn generic_score_clue(_: &Clue, deducations: &Vec<Deduction>) -> usize {
    let n_deductions = deducations.len();
    let n_tiles_revealed = deducations
        .iter()
        .filter(|deduction| deduction.is_positive)
        .count();
    (n_deductions + (n_tiles_revealed * 6)) * 10
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
    fn score_clue(&self, clue: &Clue, deducations: &Vec<Deduction>) -> usize;
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

    fn score_clue(&self, clue: &Clue, deducations: &Vec<Deduction>) -> usize {
        generic_score_clue(clue, deducations)
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

    fn score_clue(&self, clue: &Clue, deducations: &Vec<Deduction>) -> usize {
        let score = generic_score_clue(clue, deducations);

        let middle_col = self.difficulty.grid_size() as f32 / 2.0;
        // take the average distance of deductions from the center
        // and use that to boost the score
        let avg_distance_from_center = deducations
            .iter()
            .map(|d| (d.column as f32 - middle_col).abs())
            .sum::<f32>()
            / deducations.len() as f32;

        let boosted_score = score as f32 / avg_distance_from_center;
        boosted_score as usize
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

    fn score_clue(&self, clue: &Clue, deductions: &Vec<Deduction>) -> usize {
        let score = generic_score_clue(clue, deductions);

        if deductions.len() <= 1 {
            return score;
        }

        let n_unique_cols = deductions.iter().map(|d| d.column).unique().count() as usize;
        if n_unique_cols <= 1 {
            // all in same column? don't boost.
            return score;
        }

        // unique columns
        let mut unique_columns = BTreeSet::new();
        for deduction in deductions {
            unique_columns.insert(deduction.column);
        }
        if unique_columns.len() <= 1 {
            return score;
        }

        // if deduction columns are even, or odd, boost clue
        let mut deductions_per_variant: BTreeMap<char, BTreeSet<usize>> = BTreeMap::new();

        for deduction in deductions {
            deductions_per_variant
                .entry(deduction.tile.variant)
                .or_insert(BTreeSet::new())
                .insert(deduction.column);
        }

        let all_striped = deductions_per_variant.iter().all(|(_, columns)| {
            if columns.len() <= 1 {
                return true;
            }
            let all_even = columns.iter().all(|&c| c % 2 == 0);
            let all_odd = columns.iter().all(|&c| c % 2 == 1);
            all_even || all_odd
        });

        if all_striped {
            // we really want these, so reduce their score by a factor of up to 6 so they float to the top.
            score / n_unique_cols
        } else {
            score
        }
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
        (Box::new(StandardPuzzleVariant {}), 1),
        (Box::new(NarrowingPuzzleVariant { difficulty }), 3),
        (Box::new(StripingPuzzleVariant {}), 3),
    ];
    let lol = puzzle_variants
        .choose_weighted(rng, |(_, weight)| *weight)
        .map(|(variant, _)| variant)
        .expect("No puzzle variant chosen");
    lol.clone_box()
}
