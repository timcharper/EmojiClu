pub mod game_engine;
pub mod settings;
pub mod stats_manager;

#[cfg(test)]
pub mod tests {
    use crate::model::{Difficulty, Solution, Tile, MAX_GRID_SIZE};
    use std::sync::Arc;

    pub fn create_test_solution(n_rows: usize, n_variants: usize) -> Arc<Solution> {
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

        Arc::new(Solution {
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
