use crate::{
    game::deduce_clue,
    model::{Clue, ClueType, Deduction, GameBoard, HorizontalClueType, Tile, VerticalClueType},
};
use log::{info, trace};
use rand::{seq::SliceRandom, Rng, RngCore};
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

/// try and "stripe" the puzzle by eliminating event and odd columns
fn puzzle_weights_striping() -> Vec<WeightedClueType> {
    vec![
        WeightedClueType {
            weight: 25,
            clue_type: ClueType::Horizontal(HorizontalClueType::TwoAdjacent),
        },
        WeightedClueType {
            weight: 1,
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
            clue_type: ClueType::Vertical(VerticalClueType::ThreeInColumn),
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
            weight: 0,
            clue_type: ClueType::Vertical(VerticalClueType::TwoInColumnWithout),
        },
        WeightedClueType {
            weight: 1,
            clue_type: ClueType::Vertical(VerticalClueType::OneMatchesEither),
        },
        WeightedClueType {
            weight: 10,
            clue_type: ClueType::Horizontal(HorizontalClueType::ThreeAdjacent),
        },
    ]
}

/// narrowing puzzle experience; crux involves narrowing and narrowing
fn puzzle_weights_narrowing() -> Vec<WeightedClueType> {
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
            weight: 10,
            clue_type: ClueType::Horizontal(HorizontalClueType::LeftOf),
        },
        WeightedClueType {
            weight: 0,
            clue_type: ClueType::Vertical(VerticalClueType::ThreeInColumn),
        },
        WeightedClueType {
            weight: 1,
            clue_type: ClueType::Vertical(VerticalClueType::TwoInColumn),
        },
        WeightedClueType {
            weight: 4,
            clue_type: ClueType::Vertical(VerticalClueType::NotInSameColumn),
        },
        WeightedClueType {
            weight: 0,
            clue_type: ClueType::Vertical(VerticalClueType::TwoInColumnWithout),
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

// standard puzzle experience, mix of clues
fn puzzle_weights_standard() -> Vec<WeightedClueType> {
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
            weight: 2,
            clue_type: ClueType::Vertical(VerticalClueType::ThreeInColumn),
        },
        WeightedClueType {
            weight: 4,
            clue_type: ClueType::Vertical(VerticalClueType::TwoInColumn),
        },
        WeightedClueType {
            weight: 4,
            clue_type: ClueType::Vertical(VerticalClueType::NotInSameColumn),
        },
        // ClueGenerator {
        //     weight: 0,
        //     clue_type: ClueType::Vertical(VerticalClueType::TwoInColumnWithout),
        // },
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

#[derive(Debug, Clone, Copy)]
pub enum PuzzleVariantType {
    Standard,
    Narrowing,
    Striping,
}

#[derive(Debug, Clone, Copy)]
struct StandardPuzzleVariant {}
#[derive(Debug, Clone, Copy)]
struct NarrowingPuzzleVariant {}
#[derive(Debug, Clone, Copy)]
struct StripingPuzzleVariant {}

pub trait PuzzleVariantCloneBox {
    fn clone_box(&self) -> Box<dyn PuzzleVariant>;
}

/// basic scoring which punishes clues for revealing too much
fn generic_score_clue(_: &Clue, deducations: &Vec<Deduction>) -> usize {
    let n_deductions = deducations.len();
    let n_tiles_revealed = deducations
        .iter()
        .filter(|deduction| deduction.is_positive)
        .count();
    // return a score between 0 and 18 (obv score of 0 here will be pruned as the clue yields no deductions)
    (n_deductions + (n_tiles_revealed * 6)) * 3
}

fn generic_starter_evidence(state: &mut ClueGeneratorState, init_board: &GameBoard) {
    if state.rng.gen_bool(0.5) {
        let n_tiles = state.rng.gen_range(1..=2);
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

impl PuzzleVariant for StandardPuzzleVariant {
    fn get_clue_weights(&self) -> Vec<WeightedClueType> {
        puzzle_weights_standard()
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
        puzzle_weights_narrowing()
    }

    fn score_clue(&self, clue: &Clue, deducations: &Vec<Deduction>) -> usize {
        // its enough to just change the clue weights, we don't need to boost certain clue types, although perhaps experiment with it later
        generic_score_clue(clue, deducations)
    }

    fn get_variant_type(&self) -> PuzzleVariantType {
        PuzzleVariantType::Narrowing
    }

    fn populate_starter_evidence(&self, state: &mut ClueGeneratorState, init_board: &GameBoard) {
        generic_starter_evidence(state, init_board);
    }
}

impl PuzzleVariant for StripingPuzzleVariant {
    fn get_clue_weights(&self) -> Vec<WeightedClueType> {
        puzzle_weights_striping()
    }

    fn score_clue(&self, clue: &Clue, deductions: &Vec<Deduction>) -> usize {
        let score = generic_score_clue(clue, deductions);
        if deductions.len() <= 1 {
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
        let max_columns = deductions_per_variant
            .values()
            .map(|c| c.len())
            .max()
            .unwrap_or(1);

        if all_striped {
            // we really want these, so reduce their score by a factor of up to 6 so they float to the top.
            score / max_columns * 3
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
            .map(|_| state.rng.gen_range(0..init_board.solution.n_rows))
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

        // let selected_col = middle_col_min + state.rng.gen_range(0..=1);
        // let selected_row = randomly_chosen_rows[selected_col as usize];

        let seed_tile = state.random_unsolved_tile();
        let (_, selected_col) = init_board.solution.find_tile(&seed_tile);
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

pub fn random_puzzle_variant(rng: &mut Box<dyn RngCore>) -> Box<dyn PuzzleVariant> {
    let puzzle_variants: Vec<(Box<dyn PuzzleVariant>, i32)> = vec![
        (Box::new(StandardPuzzleVariant {}), 2),
        (Box::new(NarrowingPuzzleVariant {}), 1),
        (Box::new(StripingPuzzleVariant {}), 1),
    ];
    let lol = puzzle_variants
        .choose_weighted(rng, |(_, weight)| *weight)
        .map(|(variant, _)| variant)
        .expect("No puzzle variant chosen");
    lol.clone_box()
}
