use std::fmt::Display;
use std::ops::RangeInclusive;

use crate::model::{Difficulty, Tile};
use log::trace;
use rand::{seq::SliceRandom, SeedableRng};
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};

pub const MAX_GRID_SIZE: usize = 8;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Solution {
    pub variants: Vec<char>,
    pub variants_range: RangeInclusive<char>,
    pub grid: [[Tile; MAX_GRID_SIZE]; MAX_GRID_SIZE], // [row][col]
    pub n_rows: usize,
    pub n_variants: usize,
    pub difficulty: Difficulty,
    pub seed: u64,
}

impl Default for Solution {
    fn default() -> Self {
        Solution {
            variants: vec![],
            variants_range: 'a'..='a',
            grid: [[Tile::new(0, 'a'); MAX_GRID_SIZE]; MAX_GRID_SIZE],
            n_rows: 0,
            n_variants: 0,
            difficulty: Difficulty::default(),
            seed: 0,
        }
    }
}

impl Solution {
    pub fn variants_range(n_variants: usize) -> RangeInclusive<char> {
        let last_variant = (b'a' + (n_variants - 1) as u8) as char;
        'a'..=last_variant
    }

    pub fn new(difficulty: Difficulty, seed: Option<u64>) -> Self {
        let n_rows = difficulty.grid_size();
        let n_variants = n_rows;

        if n_rows == 0 || n_variants == 0 {
            return Self::default();
        }
        assert!(
            n_rows <= MAX_GRID_SIZE,
            "n_rows must be <= {}",
            MAX_GRID_SIZE
        );
        assert!(
            n_variants <= MAX_GRID_SIZE,
            "n_variants must be <= {}",
            MAX_GRID_SIZE
        );

        let mut grid = [[Tile::new(0, '0'); MAX_GRID_SIZE]; MAX_GRID_SIZE];

        let seed = seed.unwrap_or(rand::thread_rng().next_u64());

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        let variants_range = Self::variants_range(n_variants);
        let variants = variants_range.clone().collect::<Vec<char>>();

        for row in 0..n_rows {
            // Generate tiles for this row
            let mut row_tiles: Vec<Tile> = variants
                .iter()
                .map(|&variant| Tile::new(row, variant))
                .collect();

            // Shuffle just this row's tiles
            row_tiles.shuffle(&mut rng);

            // Copy shuffled tiles directly into grid row
            for col in 0..n_variants {
                grid[row][col] = row_tiles[col];
            }
        }
        trace!(target: "solution", "Solution grid: {:?}", grid);

        Self {
            variants,
            variants_range,
            grid,
            n_rows,
            n_variants,
            difficulty,
            seed,
        }
    }

    pub fn get(&self, row: usize, col: usize) -> Tile {
        self.grid[row][col]
    }

    pub fn find_tile(&self, tile: &Tile) -> (usize, usize) {
        for row in 0..self.n_rows {
            for col in 0..self.n_variants {
                if self.grid[row][col] == *tile {
                    return (row, col);
                }
            }
        }
        panic!("Tile {:?} not found in solution", tile);
    }

    pub fn all_tiles(&self) -> Vec<Tile> {
        let mut tiles = Vec::new();
        for row in 0..self.n_rows {
            for variant in self.variants_range.clone() {
                tiles.push(Tile::new(row, variant));
            }
        }
        tiles
    }
}

impl Display for Solution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = String::new();

        for row in 0..self.n_rows {
            // Row header
            output.push_str(&format!("{}|", row));

            // Cells
            for col in 0..self.n_variants {
                let tile = self.grid[row][col];
                output.push_str(&format!("{}|", tile.variant.to_ascii_uppercase()));
            }
            output.push('\n');
            output.push_str(&"-".repeat(self.n_variants * 2 + 2));
            output.push('\n');
        }

        write!(f, "{}", output)
    }
}
