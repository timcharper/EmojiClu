use log::{info, trace};
use rand::{rngs::StdRng, seq::IteratorRandom, RngCore, SeedableRng};
use std::collections::{HashMap, HashSet};

use crate::{
    game::solver::{perform_evaluation_step, EvaluationStepResult},
    model::{Clue, ClueOrientation, Deduction, Tile, TileAssertion},
};

use super::GameBoard;

pub const MAX_HORIZ_CLUES: usize = 48;
pub const MAX_VERT_CLUES: usize = 16;
const MAX_HORIZONTAL_TILE_USAGE: usize = 3;
const MAX_VERTICAL_TILE_USAGE: usize = 2;

#[derive(Debug)]
pub struct ClueEvaluation {
    pub clue: Clue,
    pub deductions: Vec<Deduction>,
    pub n_tiles_revealed: usize,
    pub score: usize,
}

pub struct ClueGeneratorState {
    pub board: GameBoard,
    pub revealed_tiles: HashSet<Tile>,
    pub tiles_with_evidence: HashSet<(usize, Tile)>,
    pub tiles_without_evidence: HashSet<(usize, Tile)>,
    pub clues: Vec<Clue>,
    pub rng: Box<dyn RngCore>,
    pub horizontal_usage: HashMap<Tile, usize>,
    pub vertical_usage: HashMap<Tile, usize>,
    pub horizontal_clues: usize,
    pub vertical_clues: usize,
    pub unsolved_columns: HashSet<usize>,
    pub unsolved_rows: HashSet<usize>,
    pub selection_count_by_row: Vec<usize>,
    pub selection_count_by_column: Vec<usize>,
    pub unsolved_tiles: HashSet<Tile>,
}

impl ClueGeneratorState {
    pub(crate) fn new(board: GameBoard, random_seed: Option<u64>) -> Self {
        let board = board.clone();
        let selection_count_by_row = vec![0; board.solution.n_rows];
        let selection_count_by_column = vec![0; board.solution.n_variants];
        let unsolved_columns: HashSet<usize> = (0..board.solution.n_variants).collect();
        let unsolved_rows: HashSet<usize> = (0..board.solution.n_rows).collect();
        let unsolved_tiles: HashSet<Tile> = board.solution.all_tiles().into_iter().collect();
        let mut tiles_without_evidence: HashSet<(usize, Tile)> = HashSet::new();

        for row in 0..board.solution.n_rows {
            for col in 0..board.solution.n_variants {
                let tile = board.solution.get(row, col);
                tiles_without_evidence.insert((col, tile));
            }
        }

        Self {
            selection_count_by_row,
            selection_count_by_column,
            board,
            revealed_tiles: HashSet::new(),
            tiles_with_evidence: HashSet::new(),
            tiles_without_evidence,
            clues: Vec::new(),
            rng: match random_seed {
                Some(s) => Box::new(StdRng::seed_from_u64(s)),
                None => Box::new(rand::thread_rng()),
            },
            horizontal_usage: HashMap::new(),
            vertical_usage: HashMap::new(),
            horizontal_clues: 0,
            vertical_clues: 0,
            unsolved_columns,
            unsolved_rows,
            unsolved_tiles,
        }
    }

    /// pick a random tile from the board, prioritizing tiles with evidence from unsolved columns / rows
    pub fn random_tile_with_evidence(&mut self) -> Tile {
        trace!(
            target: "clue_generator",
            "Tiles with evidence: {:?}",
            self.tiles_with_evidence
        );

        self.tiles_with_evidence
            .iter()
            .choose(&mut self.rng)
            .map(|(_, t)| t.clone())
            .unwrap()
    }

    fn increment_horizontal_usage(&mut self, tile: &Tile) {
        *self.horizontal_usage.entry(tile.clone()).or_insert(0) += 1;
    }

    fn increment_vertical_usage(&mut self, tile: &Tile) {
        *self.vertical_usage.entry(tile.clone()).or_insert(0) += 1;
    }

    fn get_horizontal_usage(&self, tile: &Tile) -> usize {
        *self.horizontal_usage.get(tile).unwrap_or(&0)
    }

    fn get_vertical_usage(&self, tile: &Tile) -> usize {
        *self.vertical_usage.get(tile).unwrap_or(&0)
    }

    pub(crate) fn would_exceed_usage_limits(&self, clue: &Clue) -> bool {
        if clue.is_horizontal() && self.horizontal_clues >= MAX_HORIZ_CLUES {
            return true;
        }
        if clue.is_vertical() && self.vertical_clues >= MAX_VERT_CLUES {
            return true;
        }
        if clue.is_horizontal() {
            clue.assertions
                .iter()
                .filter(|a| a.assertion)
                .any(|a| self.get_horizontal_usage(&a.tile) >= MAX_HORIZONTAL_TILE_USAGE)
        } else {
            clue.assertions
                .iter()
                .filter(|a| a.assertion)
                .any(|a| self.get_vertical_usage(&a.tile) >= MAX_VERTICAL_TILE_USAGE)
        }
    }

    fn record_clue_usage(&mut self, clue: &Clue) {
        if clue.is_horizontal() {
            for TileAssertion { tile, assertion } in clue.assertions.iter() {
                if *assertion {
                    self.increment_horizontal_usage(&tile);
                }
            }
        } else {
            for TileAssertion { tile, assertion } in clue.assertions.iter() {
                if *assertion {
                    self.increment_vertical_usage(&tile);
                }
            }
        }
    }

    fn record_selections(&mut self, selections: Vec<(usize, Tile)>) {
        for (col, tile) in selections.into_iter() {
            self.selection_count_by_column[col] += 1;
            if (self.selection_count_by_column[col]) == self.board.solution.n_rows {
                self.unsolved_columns.remove(&col);
            }
            self.selection_count_by_row[tile.row] += 1;
            if (self.selection_count_by_row[tile.row]) == self.board.solution.n_variants {
                self.unsolved_rows.remove(&tile.row);
            }
            self.unsolved_tiles.remove(&tile);
        }
    }

    pub(crate) fn add_selected_tile(&mut self, tile: Tile, column: usize) {
        trace!(
            target: "clue_generator",
            "Adding selected tile: {:?}",
            tile
        );
        self.board.select_tile_from_solution(tile);
        self.revealed_tiles.insert(tile);
        self.tiles_with_evidence.insert((column, tile));
        self.tiles_without_evidence.remove(&(column, tile));
        self.update_evidence_from_deduction(&Deduction {
            tile,
            column,
            is_positive: true,
        });
        self.record_selections(vec![(column, tile)]);
        let (_, selections) = self.board.auto_solve_all();
        self.record_selections(selections);
    }

    pub(crate) fn add_clue(&mut self, clue_eval: &ClueEvaluation) {
        if clue_eval.clue.is_horizontal() {
            self.horizontal_clues += 1
        } else {
            self.vertical_clues += 1
        }

        if self.horizontal_clues > MAX_HORIZ_CLUES || self.vertical_clues > MAX_VERT_CLUES {
            panic!("Exceeded clue usage limits!");
        }

        self.record_clue_usage(&clue_eval.clue);
        self.clues.push(clue_eval.clue.clone());
        self.board.apply_deductions(&clue_eval.deductions);
        for deduction in clue_eval.deductions.iter() {
            self.update_evidence_from_deduction(deduction);
        }
        let (_, selections) = self.board.auto_solve_all();
        self.record_selections(selections);
    }

    fn update_evidence_from_deduction(&mut self, deduction: &Deduction) {
        if deduction.is_positive {
            // all variants get evidence when a positive selection is made
            for variant in self.board.solution.variants_range.clone() {
                self.add_evidence(Tile::new(deduction.tile.row, variant), deduction.column);
            }
        } else {
            // only the tile that was removed gets evidence
            self.add_evidence(deduction.tile.clone(), deduction.column);
        }
    }

    fn add_evidence(&mut self, tile: Tile, column: usize) {
        self.tiles_with_evidence.insert((column, tile));
        self.tiles_without_evidence.remove(&(column, tile));
    }

    pub fn prune_clues(&mut self, board: &GameBoard, revealed_tiles: HashSet<Tile>) {
        let mut board = board.clone();
        revealed_tiles.into_iter().for_each(|t| {
            board.select_tile_from_solution(t);
        });
        // simulate solving the board from scratch. Remove any unused clues
        for _ in 0..=1 {
            let mut board = board.clone();
            // once forward, once backwards
            self.clues.reverse();

            trace!(
                target: "clue_generator",
                "Beginning prune; Initial board: {:?}",
                board
            );
            let mut used_clues = HashSet::new();

            while !board.is_complete() {
                board.auto_solve_all();
                let result = perform_evaluation_step(&mut board, &self.clues);
                match result {
                    EvaluationStepResult::Nothing => break,
                    EvaluationStepResult::HiddenPairsFound => {}
                    EvaluationStepResult::DeductionsFound(clue) => {
                        trace!(
                            target: "clue_generator",
                            "Used clue {:?}; current board: {:?}",
                            clue,
                            board
                        );
                        used_clues.insert(clue.clone());
                    }
                }
            }
            if !board.is_complete() {
                panic!("Failed to solve board!");
            }
            info!(
                target: "clue_generator",
                "Used {} / {} clues to solve the puzzle",
                used_clues.len(),
                self.clues.len()
            );

            trace!(
                target: "clue_generator",
                "Board after solving: {:?}",
                board
            );
            self.clues.retain(|c| used_clues.contains(c));
        }
    }
}
