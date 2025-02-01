use super::{
    clue_generator_state::{ClueEvaluation, ClueGeneratorState},
    puzzle_variants::{random_puzzle_variant, PuzzleVariant},
};

use log::{info, trace, warn};
use std::{collections::BTreeSet, rc::Rc};

use crate::{
    game::solver::{perform_evaluation_step, EvaluationStepResult},
    model::{Clue, ClueSet, Difficulty, GameBoard, Tile},
};

use super::deduce_clue;

fn evaluate_clue(
    board: &GameBoard,
    puzzle_variant: &Box<dyn PuzzleVariant>,
    clue: &Clue,
) -> ClueEvaluation {
    let deductions = deduce_clue(board, clue);

    let score = puzzle_variant.score_clue(clue, &deductions);
    ClueEvaluation {
        clue: clue.clone(),
        deductions,
        score,
    }
}

pub struct ClueGeneratorResult {
    pub clues: Vec<Clue>,
    pub revealed_tiles: Vec<Tile>,
    /// The board after revealing initial tiles
    pub board: GameBoard,
}

pub fn apply_selections(board: &GameBoard, tiles: &BTreeSet<Tile>) -> GameBoard {
    let mut board = board.clone();
    for tile in tiles {
        board.select_tile_from_solution(*tile);
    }
    board
}

pub fn generate_clues(init_board: &GameBoard) -> ClueGeneratorResult {
    trace!(
        target: "clue_generator",
        "Generating clues... for board: {:?}; solution is {:?}",
        init_board,
        init_board.solution
    );
    let mut state = ClueGeneratorState::new(init_board.clone());

    let puzzle_variant = random_puzzle_variant(&mut state.rng);
    let clue_weights = puzzle_variant.get_clue_weights();
    info!(
        target: "clue_generator",
        "Generating clues for seed {:?}, puzzle variant {:?}",
        init_board.solution.seed,
        puzzle_variant.get_variant_type()
    );

    puzzle_variant.populate_starter_evidence(&mut state, &init_board);
    if state.board.solution.difficulty == Difficulty::Veteran {
        while state.revealed_tiles.len() < 3 {
            // veteran puzzles need at least three tiles selected, otherwise the clue count is too high
            let tile = state.random_unsolved_tile();
            state.add_selected_tile(tile);
        }
    }
    let seeded_tiles = state.revealed_tiles.clone();
    let init_board = apply_selections(&init_board, &seeded_tiles);

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
            if let Some(clue) = state.generate_random_clue_type(&clue_weights, None) {
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
                let evaluation = evaluate_clue(&state.board.clone(), &puzzle_variant, &clue);
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
            } else {
                trace!(
                    target: "clue_generator",
                    "Failed to generate clue, trying again"
                );
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
            state.add_clue(&evaluated_clue.clue, &evaluated_clue.deductions);

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
            warn!(target: "clue_generator", "Stats: {:?}", state.stats);
            panic!(
                "Failed to generate valid clues after {} attempts. Board is {:?}",
                clue_generation_loops, state.board
            );
        }
    }

    state.quick_prune(&init_board);
    state.deep_prune_clues(&init_board);
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
    fn test_generate_clues_solvable(_: &mut UsingLogger) {
        // CLUE_GEN_ITERATIONS=100 RUST_LOG=info cargo test game::clue_generator::tests::test_generate_clues -- --nocapture --exact

        let n_iterations = std::env::var("CLUE_GEN_ITERATIONS").unwrap_or("1".to_string());
        let n_iterations = n_iterations.parse::<u64>().unwrap();
        let start_seed = 979700061949446372;
        for i in 0..n_iterations {
            // we'd test Veteran if we had all day... needs compiler optimizations to run at reasonable speed
            // let solution = Solution::new(Difficulty::Veteran, Some(broken_seed + i));
            let solution = Solution::new(Difficulty::Hard, Some(start_seed + i));
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
            let clue_set = ClueSet::new(result.clues.clone());
            let mut clues = clue_set
                .all_clues()
                .into_iter()
                .map(|c| c.clue.clone())
                .collect::<Vec<_>>();
            clues.sort();
            let mut unprocessed_clues = result.clues.clone();
            unprocessed_clues.sort();
            println!("Unprocessed clues are {:?}", unprocessed_clues);
            println!("Clues are {:?}", clues);
            loop {
                println!("===================");
                let result = perform_evaluation_step(&mut board, &clues);
                if result == EvaluationStepResult::Nothing {
                    break;
                }
                println!("Result is {:?}", result);
                println!("Board is {:?}", board);
                board.auto_solve_all();
            }
            println!("Board is {:?}", board);
            println!("Clues are {:?}", clues);
            assert!(board.is_complete(), "Board is not solvable");
        }
    }

    // for some reason, our deterministic generation isn't working.
    #[test_context(UsingLogger)]
    #[test]
    fn test_generate_clues_deterministic(_: &mut UsingLogger) {
        let solution = Solution::new(Difficulty::Easy, Some(42));
        let board = GameBoard::new(solution.into());
        println!("Board is {:?}", board);

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
