use super::clue_generator_state::{ClueEvaluation, ClueGeneratorState};

pub const MAX_HORIZ_CLUES: usize = 48;
pub const MAX_VERT_CLUES: usize = 20;

use log::{info, trace};
use rand::{
    seq::{IteratorRandom, SliceRandom},
    Rng,
};
use std::rc::Rc;

use crate::{
    game::solver::{perform_evaluation_step, EvaluationStepResult},
    model::{Clue, ClueSet, ClueType, GameBoard, HorizontalClueType, Tile, VerticalClueType},
};

use super::deduce_clue;

#[derive(Debug, Clone)]
struct ClueGenerator {
    weight: usize,
    clue_type: ClueType,
}

impl ClueGenerator {
    /// `count`: number of additional tiles to get; for 3 adjacent clue, provide 2
    /// returns: (Vec<Tile>, Vec<usize>) where:
    /// - Vec<Tile> contains the seed tile followed by `count` adjacent tiles in the chosen direction
    /// - Vec<usize> are the corresponding columns chosen
    fn get_random_horiz_tiles(
        &self,
        state: &mut ClueGeneratorState,
        count: usize,
        seed: &Tile,
    ) -> (Vec<Tile>, Vec<usize>) {
        let mut tiles = Vec::new();
        let (row, col) = state.board.solution.find_tile(seed);

        let mut possible_directions = Vec::new();
        if col + count < state.board.solution.n_variants {
            possible_directions.push(1)
        }
        if col >= count {
            possible_directions.push(-1)
        }

        if possible_directions.len() == 0 {
            panic!("No possible directions found");
        }

        let direction = *possible_directions.choose_mut(&mut state.rng).unwrap();

        let mut next_col = col as i32;
        let mut columns = Vec::new();

        tiles.push(state.board.solution.get(row, col));
        columns.push(col);
        for _ in 0..count {
            next_col = next_col + direction;
            let next_row = state.rng.gen_range(0..state.board.solution.n_rows);
            let tile = state.board.solution.get(next_row, next_col as usize);
            tiles.push(tile);
            columns.push(next_col as usize);
        }
        (tiles, columns)
    }

    fn get_random_vertical_tiles(
        &self,
        state: &mut ClueGeneratorState,
        seed: &Tile,
        count: usize,
    ) -> Vec<Tile> {
        let mut tiles = Vec::new();
        let (row, col) = state.board.solution.find_tile(seed);

        let mut possible_rows = (0..state.board.solution.n_rows)
            .filter(|&r| r != row)
            .collect::<Vec<_>>();

        possible_rows.shuffle(&mut state.rng);
        let rows = possible_rows.iter().take(count).collect::<Vec<_>>();

        trace!(
            target: "clue_generator",
            "Possible rows {:?}, count: {:?}",
            possible_rows,
            count
        );
        for row in rows {
            trace!(
                target: "clue_generator",
                "Adding tile: {:?}",
                state.board.solution.get(*row, col)
            );
            tiles.push(state.board.solution.get(*row, col));
        }
        tiles
    }

    fn get_random_tile_not_from_columns(
        &self,
        state: &mut ClueGeneratorState,
        not_columns: Vec<i32>,
        tile_predicate: impl Fn(&Tile) -> bool,
    ) -> Tile {
        let col = (0..state.board.solution.n_variants)
            .filter(|&c| !not_columns.contains(&(c as i32)))
            .choose(&mut state.rng)
            .unwrap();

        let candidate_tiles = (0..state.board.solution.n_rows)
            .map(|r| state.board.solution.get(r, col))
            .filter(|t| tile_predicate(t))
            .collect::<Vec<_>>();
        candidate_tiles.choose(&mut state.rng).unwrap().clone()
    }

    fn generate_clue(&self, state: &mut ClueGeneratorState, seed: Tile) -> Option<Clue> {
        match &self.clue_type {
            ClueType::Horizontal(tpe) => match tpe {
                HorizontalClueType::ThreeAdjacent => {
                    let (tiles, _) = self.get_random_horiz_tiles(state, 2, &seed);
                    Some(Clue::three_adjacent(seed, tiles[1], tiles[2]))
                }
                HorizontalClueType::TwoApartNotMiddle => {
                    let (tiles, columns) = self.get_random_horiz_tiles(state, 2, &seed);

                    let not_tile = self.get_random_tile_not_from_columns(
                        state,
                        vec![columns[1] as i32],
                        |t| t != &seed && t != &tiles[2],
                    );
                    Some(Clue::two_apart_not_middle(seed, not_tile, tiles[2]))
                }
                HorizontalClueType::TwoAdjacent => {
                    let (tiles, _) = self.get_random_horiz_tiles(state, 2, &seed);
                    Some(Clue::adjacent(seed, tiles[1]))
                }
                HorizontalClueType::NotAdjacent => {
                    let (_, seed_col) = state.board.solution.find_tile(&seed);

                    let tile = self.get_random_tile_not_from_columns(
                        state,
                        vec![(seed_col as i32) - 1, (seed_col as i32) + 1],
                        |t| t != &seed,
                    );

                    Some(Clue::not_adjacent(seed, tile))
                }

                HorizontalClueType::LeftOf => {
                    let (_, seed_col) = state.board.solution.find_tile(&seed);
                    let possible_cols = (0..state.board.solution.n_variants)
                        .filter(|&c| c != seed_col)
                        .collect::<Vec<_>>();

                    let row = state.rng.gen_range(0..state.board.solution.n_rows);
                    let col = *possible_cols.choose(&mut state.rng).unwrap();
                    let tile = state.board.solution.get(row, col);

                    if seed_col < col {
                        Some(Clue::left_of(seed, tile))
                    } else {
                        Some(Clue::left_of(tile, seed))
                    }
                }
            },
            ClueType::Vertical(tpe) => match tpe {
                VerticalClueType::ThreeInColumn | VerticalClueType::TwoInColumn => {
                    let count = state.rng.gen_range(1..=2);
                    let tiles = self.get_random_vertical_tiles(state, &seed, count);
                    match tiles.len() {
                        2 => Some(Clue::three_in_column(seed, tiles[0], tiles[1])),
                        1 => Some(Clue::two_in_column(seed, tiles[0])),
                        _ => None,
                    }
                }
                VerticalClueType::NotInSameColumn => {
                    let (_, seed_col) = state.board.solution.find_tile(&seed);
                    let not_tile =
                        self.get_random_tile_not_from_columns(state, vec![seed_col as i32], |t| {
                            t != &seed
                        });
                    Some(Clue::two_not_in_same_column(seed, not_tile))
                }
                VerticalClueType::TwoInColumnWithout => {
                    let (_, seed_col) = state.board.solution.find_tile(&seed);
                    let tiles = self.get_random_vertical_tiles(state, &seed, 1);
                    let not_tile =
                        self.get_random_tile_not_from_columns(state, vec![seed_col as i32], |t| {
                            t.row != seed.row && t.row != tiles[0].row
                        });
                    Some(Clue::two_in_column_without(seed, not_tile, tiles[0]))
                }
                VerticalClueType::OneMatchesEither => {
                    let (_, seed_col) = state.board.solution.find_tile(&seed);
                    let tiles = self.get_random_vertical_tiles(state, &seed, 1);
                    let not_tile =
                        self.get_random_tile_not_from_columns(state, vec![seed_col as i32], |t| {
                            t.row != seed.row && t.row != tiles[0].row
                        });
                    Some(Clue::one_matches_either(seed, not_tile, tiles[0]))
                }
            },
        }
    }
}

fn generate_clue(
    state: &mut ClueGeneratorState,
    clue_generators: &Vec<ClueGenerator>,
    seed: Tile,
) -> Option<Clue> {
    let clue_generator = clue_generators
        .choose_weighted(&mut state.rng, |c| c.weight)
        .unwrap();

    let mut clue = None;
    while clue.is_none() {
        clue = clue_generator.generate_clue(state, seed);
        if clue.is_none() {
            trace!(
                target: "clue_generator",
                "Failed to generate clue, trying again ({:?})",
                clue_generator
            );
        }
    }
    clue
}

fn evaluate_clue(board: &GameBoard, clue: &Clue) -> ClueEvaluation {
    let deductions = deduce_clue(board, clue);
    let n_deductions = deductions.len();
    let n_tiles_revealed = deductions
        .iter()
        .filter(|deduction| deduction.is_positive)
        .count();

    let score = n_deductions + (n_tiles_revealed * 6);
    ClueEvaluation {
        clue: clue.clone(),
        deductions,
        n_tiles_revealed,
        score,
    }
}

pub struct ClueGeneratorResult {
    pub clues: Vec<Clue>,
    pub revealed_tiles: Vec<Tile>,
    /// The board after revealing initial tiles
    pub board: GameBoard,
}

pub fn generate_clues(init_board: &GameBoard) -> ClueGeneratorResult {
    trace!(
        target: "clue_generator",
        "Generating clues... for board: {:?}; solution is {:?}",
        init_board,
        init_board.solution
    );
    let mut state = ClueGeneratorState::new(init_board.clone());
    let three_adjacent_clue_generator = ClueGenerator {
        weight: 6,
        clue_type: ClueType::Horizontal(HorizontalClueType::ThreeAdjacent),
    };
    let clue_generators = vec![
        three_adjacent_clue_generator.clone(),
        ClueGenerator {
            weight: 6,
            clue_type: ClueType::Vertical(VerticalClueType::ThreeInColumn),
        },
        ClueGenerator {
            weight: 3,
            clue_type: ClueType::Horizontal(HorizontalClueType::TwoAdjacent),
        },
        ClueGenerator {
            weight: 4,
            clue_type: ClueType::Horizontal(HorizontalClueType::TwoApartNotMiddle),
        },
        ClueGenerator {
            weight: 2,
            clue_type: ClueType::Horizontal(HorizontalClueType::NotAdjacent),
        },
        ClueGenerator {
            weight: 1,
            clue_type: ClueType::Horizontal(HorizontalClueType::LeftOf),
        },
        ClueGenerator {
            weight: 2,
            clue_type: ClueType::Vertical(VerticalClueType::NotInSameColumn),
        },
        ClueGenerator {
            weight: 2,
            clue_type: ClueType::Vertical(VerticalClueType::TwoInColumnWithout),
        },
        ClueGenerator {
            weight: 2,
            clue_type: ClueType::Vertical(VerticalClueType::OneMatchesEither),
        },
    ];

    if state.rng.gen_bool(0.5) {
        let n_tiles = state.rng.gen_range(1..=2);
        for _ in 0..n_tiles {
            let row = state.rng.gen_range(0..init_board.solution.n_rows);
            let col = state.rng.gen_range(0..init_board.solution.n_variants);
            let tile = init_board.solution.get(row, col);
            state.add_selected_tile(tile, col);
        }
    } else {
        let three_only_clue_generator = vec![three_adjacent_clue_generator.clone()];
        let tiles = init_board.solution.all_tiles();

        for _ in 0..3 {
            let seed = tiles.choose(&mut state.rng).unwrap().clone();
            println!("Seed: {:?}", seed);
            let clue = generate_clue(&mut state, &three_only_clue_generator, seed).unwrap();
            state.add_clue(&evaluate_clue(&state.board, &clue));
        }
    }

    while !state.board.is_complete() {
        info!(
            target: "clue_generator",
            "Generating clues..."
        );
        let mut possible_clues = Vec::new();
        let mut clue_generation_loops = 0;
        let clue_candidate_count = state.board.solution.difficulty.look_ahead_count();
        state.reset_stats();
        while possible_clues.len() < clue_candidate_count
            && clue_generation_loops < clue_candidate_count * 1000
        /* TODO - need to make the clue generation guided to try to choose at least one unsolved tile. */
        {
            clue_generation_loops += 1;
            let seed = state.random_tile_with_evidence();
            if let Some(clue) = generate_clue(&mut state, &clue_generators, seed) {
                if state.would_exceed_usage_limits(&clue) {
                    trace!(
                        target: "clue_generator",
                        "Skipping clue with usage limits exceeded: {:?}",
                        clue
                    );
                    continue;
                }
                let non_singleton_intersecting_clues = state
                    .clues
                    .iter()
                    .find(|c| clue.non_singleton_intersects(c));

                if non_singleton_intersecting_clues.is_some() {
                    state.stats.n_rejected_non_singleton_intersecting_clues += 1;
                    trace!(
                        target: "clue_generator",
                        "Skipping clue with non-singleton intersecting clues: {:?} - {:?}",
                        clue,
                        non_singleton_intersecting_clues.unwrap()
                    );
                    trace!(
                        target: "clue_generator",
                        "Board state was {:?}",
                        state.board
                    );
                    continue;
                }
                let evaluation = evaluate_clue(&state.board.clone(), &clue);
                if evaluation.deductions.len() == 0 {
                    state.stats.n_rejected_no_deductions += 1;
                    trace!(
                        target: "clue_generator",
                        "Skipping clue with no deductions: {:?}",
                        clue
                    );
                    trace!(
                        target: "clue_generator",
                        "Board state was {:?}",
                        state.board
                    );
                    continue;
                }
                trace!(
                    target: "clue_generator",
                    "Considering clue {:?} with # deductions {:?}",
                    clue,
                    evaluation.deductions.len()
                );
                possible_clues.push(evaluation);
            }
        }
        info!(
            target: "clue_generator",
            "Clue generation loop done; found {} clues. Stats: {:?}",
            possible_clues.len(),
            state.stats
        );
        possible_clues.sort_by_key(|c| c.score);
        if let Some(evaluated_clue) = possible_clues.first() {
            trace!(
                target: "clue_generator",
                "Adding clue: {:?}",
                evaluated_clue
            );
            state.add_clue(evaluated_clue);

            // re-evaluate clues from the beginning after applying new evidence
            while perform_evaluation_step(&mut state.board, &state.clues)
                != EvaluationStepResult::Nothing
            {}
            assert!(
                state.board.is_valid_possibility(),
                "Error! After clue {:?}, board entered an invalid state",
                evaluated_clue
            );
        } else {
            println!("Stats: {:?}", state.stats);
            panic!(
                "Failed to generate valid clues after {} attempts.",
                clue_generation_loops
            );
        }
    }

    state.prune_clues(&init_board, state.revealed_tiles.clone());

    trace!(
        target: "clue_generator",
        "Solved board: {:?}",
        state.board
    );

    let mut board_with_revealed_tiles = init_board.clone();
    for tile in state.revealed_tiles.iter() {
        board_with_revealed_tiles.select_tile_from_solution(*tile);
    }

    let clue_set = Rc::new(ClueSet::new(state.clues.clone()));
    board_with_revealed_tiles.set_clues(clue_set);

    ClueGeneratorResult {
        clues: state.clues,
        revealed_tiles: state.revealed_tiles.into_iter().collect(),
        board: board_with_revealed_tiles,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        game::tests::UsingLogger,
        model::{Difficulty, GameBoard, Solution},
    };
    use test_context::test_context;

    use super::*;

    #[test_context(UsingLogger)]
    #[test]
    fn test_generate_clues(_: &mut UsingLogger) {
        // CLUE_GEN_ITERATIONS=100 RUST_LOG=info cargo test game::clue_generator::tests::test_generate_clues -- --nocapture --exact

        let n_iterations = std::env::var("CLUE_GEN_ITERATIONS").unwrap_or("1".to_string());
        let n_iterations = n_iterations.parse::<u64>().unwrap();
        for i in 0..n_iterations {
            let solution = Solution::new(Difficulty::Hard, Some(i));
            let init_board = GameBoard::new(solution.into());
            let result = generate_clues(&init_board);
            trace!(
                target: "clue_generator",
                "Generated clues: {:?}",
                result.clues
            );
            assert!(result.clues.len() > 0);
            // assert solvable
            let mut board = result.board.clone();
            while perform_evaluation_step(&mut board, &result.clues)
                != EvaluationStepResult::Nothing
            {
                board.auto_solve_all();
            }
            println!("Board is {:?}", board);
            println!("Clues are {:?}", result.clues);
            assert!(board.is_complete(), "Board is not solvable");
        }
    }

    // for some reason, our deterministic generation isn't working.
    #[test]
    fn test_generate_clues_deterministic() {
        let solution = Solution::new(Difficulty::Easy, Some(42));
        let board = GameBoard::new(solution.into());

        // Generate clues twice with same seed
        let result1 = generate_clues(&board);
        let result2 = generate_clues(&board);

        // Should generate exact same clues in same order
        assert_eq!(result1.clues.len(), result2.clues.len());
        for (clue1, clue2) in result1.clues.iter().zip(result2.clues.iter()) {
            assert_eq!(clue1.assertions, clue2.assertions);
        }

        // Should reveal same tiles
        assert_eq!(result1.revealed_tiles.len(), result2.revealed_tiles.len());
        for (tile1, tile2) in result1
            .revealed_tiles
            .iter()
            .zip(result2.revealed_tiles.iter())
        {
            assert_eq!(tile1, tile2);
        }
    }
}
