pub mod clue_completion_evaluator;
pub mod clue_generator;
pub mod clue_generator_state;
pub mod game_state;
pub mod hidden_pair_finder;
mod puzzle_variants;
pub mod settings;
pub mod solver;
pub mod stats_manager;
pub use clue_generator::generate_clues;
pub use solver::deduce_clue;

#[cfg(test)]
mod tests {
    use crate::model::{Difficulty, Solution, Tile, MAX_GRID_SIZE};
    use std::rc::Rc;

    pub fn create_test_solution(n_rows: usize, n_variants: usize) -> Rc<Solution> {
        let mut grid = [['0'; MAX_GRID_SIZE]; MAX_GRID_SIZE];
        // Fill first 4x4 of grid with test data
        for row in 0..3 {
            for col in 0..4 {
                grid[row][col] = Tile::usize_to_variant(col);
            }
        }

        let start_variant = Tile::usize_to_variant(0);
        let end_variant = Tile::usize_to_variant(n_variants - 1);

        let variants_range = start_variant..=end_variant;

        Rc::new(Solution {
            variants: variants_range.clone().collect(),
            grid,
            n_rows,
            n_variants,
            variants_range,
            difficulty: Difficulty::Easy,
            seed: 0,
        })
    }
}
