use log::{info, trace};
use rand::{
    rngs::StdRng,
    seq::{IteratorRandom, SliceRandom},
    Rng, RngCore, SeedableRng,
};
use std::collections::{BTreeSet, HashMap};

use crate::{
    game::solver::{perform_evaluation_step, EvaluationStepResult},
    model::{
        Clue, ClueType, Deduction, GameBoard, HorizontalClueType, Tile, TileAssertion,
        VerticalClueType,
    },
};

use super::puzzle_variants::WeightedClueType;

pub const MAX_HORIZ_CLUES: usize = 48;
pub const MAX_VERT_CLUES: usize = 32;
const MAX_HORIZONTAL_TILE_USAGE: usize = 3;
const MAX_VERTICAL_TILE_USAGE: usize = 2;

#[derive(Debug)]
pub struct ClueGeneratorStats {
    pub n_rejected_no_deductions: usize,
    pub n_rejected_exceeds_usage_limits: usize,
    pub n_rejected_exceeds_max_vert_clues: usize,
    pub n_rejected_exceeds_max_horiz_clues: usize,
    pub n_rejected_non_singleton_intersecting_clues: usize,
}
impl Default for ClueGeneratorStats {
    fn default() -> Self {
        ClueGeneratorStats {
            n_rejected_exceeds_usage_limits: 0,
            n_rejected_exceeds_max_vert_clues: 0,
            n_rejected_exceeds_max_horiz_clues: 0,
            n_rejected_no_deductions: 0,
            n_rejected_non_singleton_intersecting_clues: 0,
        }
    }
}

#[derive(Debug)]
pub struct ClueEvaluation {
    pub clue: Clue,
    pub deductions: Vec<Deduction>,
    pub score: usize,
}

pub struct ClueGeneratorState {
    pub board: GameBoard,
    pub revealed_tiles: BTreeSet<Tile>,
    pub tiles_with_evidence: BTreeSet<(usize, Tile)>,
    pub tiles_without_evidence: BTreeSet<(usize, Tile)>,
    pub clues: Vec<Clue>,
    pub rng: Box<dyn RngCore>,
    pub horizontal_usage: HashMap<Tile, usize>,
    pub vertical_usage: HashMap<Tile, usize>,
    pub horizontal_clues: usize,
    pub vertical_clues: usize,
    pub unsolved_columns: BTreeSet<usize>,
    pub unsolved_rows: BTreeSet<usize>,
    pub selection_count_by_row: Vec<usize>,
    pub selection_count_by_column: Vec<usize>,
    pub unsolved_tiles: BTreeSet<Tile>,
    pub stats: ClueGeneratorStats,
}

impl ClueGeneratorState {
    pub(crate) fn new(board: GameBoard) -> Self {
        let board = board.clone();
        let selection_count_by_row = vec![0; board.solution.n_rows];
        let selection_count_by_column = vec![0; board.solution.n_variants];
        let unsolved_columns: BTreeSet<usize> = (0..board.solution.n_variants).collect();
        let unsolved_rows: BTreeSet<usize> = (0..board.solution.n_rows).collect();
        let unsolved_tiles: BTreeSet<Tile> = board.solution.all_tiles().into_iter().collect();
        let mut tiles_without_evidence: BTreeSet<(usize, Tile)> = BTreeSet::new();

        for row in 0..board.solution.n_rows {
            for col in 0..board.solution.n_variants {
                let tile = board.solution.get(row, col);
                tiles_without_evidence.insert((col, tile));
            }
        }

        let rng = Box::new(StdRng::seed_from_u64(board.solution.seed));

        Self {
            selection_count_by_row,
            selection_count_by_column,
            board,
            revealed_tiles: BTreeSet::new(),
            tiles_with_evidence: BTreeSet::new(),
            tiles_without_evidence,
            clues: Vec::new(),
            rng,
            horizontal_usage: HashMap::new(),
            vertical_usage: HashMap::new(),
            horizontal_clues: 0,
            vertical_clues: 0,
            unsolved_columns,
            unsolved_rows,
            unsolved_tiles,
            stats: ClueGeneratorStats::default(),
        }
    }
    pub fn reset_stats(&mut self) {
        self.stats = ClueGeneratorStats::default();
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

    pub(crate) fn would_exceed_usage_limits(&mut self, clue: &Clue) -> bool {
        // too many clues?
        if clue.is_horizontal() && self.horizontal_clues >= MAX_HORIZ_CLUES {
            self.stats.n_rejected_exceeds_max_horiz_clues += 1;
            return true;
        }
        if clue.is_vertical() && self.vertical_clues >= MAX_VERT_CLUES {
            self.stats.n_rejected_exceeds_max_vert_clues += 1;
            return true;
        }
        // too many hints on the same tile?
        let result = if clue.is_horizontal() {
            clue.assertions
                .iter()
                .filter(|a| a.assertion)
                .any(|a| self.get_horizontal_usage(&a.tile) >= MAX_HORIZONTAL_TILE_USAGE)
        } else {
            clue.assertions
                .iter()
                .filter(|a| a.assertion)
                .any(|a| self.get_vertical_usage(&a.tile) >= MAX_VERTICAL_TILE_USAGE)
        };
        if result {
            self.stats.n_rejected_exceeds_usage_limits += 1;
        }
        result
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

    pub(crate) fn add_clue(&mut self, clue: &Clue, deductions: &Vec<Deduction>) {
        if clue.is_horizontal() {
            self.horizontal_clues += 1
        } else {
            self.vertical_clues += 1
        }

        if self.horizontal_clues > MAX_HORIZ_CLUES || self.vertical_clues > MAX_VERT_CLUES {
            panic!("Exceeded clue usage limits!");
        }

        self.record_clue_usage(&clue);
        self.clues.push(clue.clone());
        self.board.apply_deductions(&deductions);
        for deduction in deductions.iter() {
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

    pub fn prune_clues(&mut self, board: &GameBoard, revealed_tiles: BTreeSet<Tile>) {
        let mut board = board.clone();
        revealed_tiles.into_iter().for_each(|t| {
            board.select_tile_from_solution(t);
        });

        let original_clue_count = self.clues.len();

        trace!(
            target: "clue_generator",
            "Original clues: {:?}",
            self.clues
        );

        // Try removing each clue one at a time
        let mut i = 0;
        while i < self.clues.len() {
            let clue = self.clues.remove(i);
            let mut test_board = board.clone();

            // Try solving without this clue
            while perform_evaluation_step(&mut test_board, &self.clues)
                != EvaluationStepResult::Nothing
            {
                test_board.auto_solve_all();
            }

            if !test_board.is_complete() {
                trace!(
                    target: "clue_generator",
                    "Board wasn't solvable without clue {:?}; keeping it",
                    clue
                );
                trace!(
                    target: "clue_generator",
                    "Board state: {:?}",
                    test_board
                );
                // Board wasn't solvable without this clue, put it back
                self.clues.insert(i, clue);
                i += 1;
            }
            // If board was solvable without the clue, leave it removed and don't increment i
            // since we need to test the next clue at the same index
        }

        info!(
            target: "clue_generator",
            "Deep prune reduced clues from {} to {} clues",
            original_clue_count,
            self.clues.len()
        );
        trace!(
            target: "clue_generator",
            "Pruned clues: {:?}",
            self.clues
        );
    }

    fn get_random_tile_not_from_columns(
        &mut self,
        not_columns: Vec<i32>,
        tile_predicate: impl Fn(&Tile) -> bool,
    ) -> Tile {
        let col = (0..self.board.solution.n_variants)
            .filter(|&c| !not_columns.contains(&(c as i32)))
            .choose(&mut self.rng)
            .unwrap();

        let candidate_tiles = (0..self.board.solution.n_rows)
            .map(|r| self.board.solution.get(r, col))
            .filter(|t| tile_predicate(t))
            .collect::<Vec<_>>();
        candidate_tiles.choose(&mut self.rng).unwrap().clone()
    }

    /// `count`: number of additional tiles to get; for 3 adjacent clue, provide 2
    /// returns: (Vec<Tile>, Vec<usize>) where:
    /// - Vec<Tile> contains the seed tile followed by `count` adjacent tiles in the chosen direction
    /// - Vec<usize> are the corresponding columns chosen
    fn get_random_horiz_tiles(&mut self, count: usize, seed: &Tile) -> (Vec<Tile>, Vec<usize>) {
        let mut tiles = Vec::new();
        let (row, col) = self.board.solution.find_tile(seed);

        let mut possible_directions = Vec::new();
        if col + count < self.board.solution.n_variants {
            possible_directions.push(1)
        }
        if col >= count {
            possible_directions.push(-1)
        }

        if possible_directions.len() == 0 {
            panic!("No possible directions found");
        }

        let direction = *possible_directions.choose_mut(&mut self.rng).unwrap();

        let mut next_col = col as i32;
        let mut columns = Vec::new();

        tiles.push(self.board.solution.get(row, col));
        columns.push(col);
        for _ in 0..count {
            next_col = next_col + direction;
            let next_row = self.rng.gen_range(0..self.board.solution.n_rows);
            let tile = self.board.solution.get(next_row, next_col as usize);
            tiles.push(tile);
            columns.push(next_col as usize);
        }
        (tiles, columns)
    }

    fn get_random_vertical_tiles(&mut self, seed: &Tile, count: usize) -> Vec<Tile> {
        let mut tiles = Vec::new();
        let (row, col) = self.board.solution.find_tile(seed);

        let mut possible_rows = (0..self.board.solution.n_rows)
            .filter(|&r| r != row)
            .collect::<Vec<_>>();

        possible_rows.shuffle(&mut self.rng);
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
                self.board.solution.get(*row, col)
            );
            tiles.push(self.board.solution.get(*row, col));
        }
        tiles
    }

    fn generate_clue(&mut self, clue_type: &ClueType, seed: Tile) -> Option<Clue> {
        match &clue_type {
            ClueType::Horizontal(tpe) => match tpe {
                HorizontalClueType::ThreeAdjacent => {
                    let (tiles, _) = self.get_random_horiz_tiles(2, &seed);
                    Some(Clue::three_adjacent(seed, tiles[1], tiles[2]))
                }
                HorizontalClueType::TwoApartNotMiddle => {
                    let (tiles, columns) = self.get_random_horiz_tiles(2, &seed);

                    let not_tile = self
                        .get_random_tile_not_from_columns(vec![columns[1] as i32], |t| {
                            t != &seed && t != &tiles[2]
                        });
                    Some(Clue::two_apart_not_middle(seed, not_tile, tiles[2]))
                }
                HorizontalClueType::TwoAdjacent => {
                    let (tiles, _) = self.get_random_horiz_tiles(2, &seed);
                    Some(Clue::adjacent(seed, tiles[1]))
                }
                HorizontalClueType::NotAdjacent => {
                    let (_, seed_col) = self.board.solution.find_tile(&seed);

                    let tile = self.get_random_tile_not_from_columns(
                        vec![(seed_col as i32) - 1, (seed_col as i32) + 1],
                        |t| t != &seed,
                    );

                    Some(Clue::not_adjacent(seed, tile))
                }

                HorizontalClueType::LeftOf => {
                    let (_, seed_col) = self.board.solution.find_tile(&seed);
                    let possible_cols = (0..self.board.solution.n_variants)
                        .filter(|&c| c != seed_col)
                        .collect::<Vec<_>>();

                    let row = self.rng.gen_range(0..self.board.solution.n_rows);
                    let col = *possible_cols.choose(&mut self.rng).unwrap();
                    let tile = self.board.solution.get(row, col);

                    if seed_col < col {
                        Some(Clue::left_of(seed, tile))
                    } else {
                        Some(Clue::left_of(tile, seed))
                    }
                }
            },
            ClueType::Vertical(tpe) => match tpe {
                VerticalClueType::ThreeInColumn | VerticalClueType::TwoInColumn => {
                    let count = self.rng.gen_range(1..=2);
                    let tiles = self.get_random_vertical_tiles(&seed, count);
                    match tiles.len() {
                        2 => Some(Clue::three_in_column(seed, tiles[0], tiles[1])),
                        1 => Some(Clue::two_in_column(seed, tiles[0])),
                        _ => None,
                    }
                }
                VerticalClueType::NotInSameColumn => {
                    let (_, seed_col) = self.board.solution.find_tile(&seed);
                    let not_tile = self
                        .get_random_tile_not_from_columns(vec![seed_col as i32], |t| t != &seed);
                    Some(Clue::two_not_in_same_column(seed, not_tile))
                }
                VerticalClueType::TwoInColumnWithout => {
                    let (_, seed_col) = self.board.solution.find_tile(&seed);
                    let tiles = self.get_random_vertical_tiles(&seed, 1);
                    let not_tile = self
                        .get_random_tile_not_from_columns(vec![seed_col as i32], |t| {
                            t.row != seed.row && t.row != tiles[0].row
                        });
                    Some(Clue::two_in_column_without(seed, not_tile, tiles[0]))
                }
                VerticalClueType::OneMatchesEither => {
                    let (_, seed_col) = self.board.solution.find_tile(&seed);
                    let tiles = self.get_random_vertical_tiles(&seed, 1);
                    let not_tile = self
                        .get_random_tile_not_from_columns(vec![seed_col as i32], |t| {
                            t.row != seed.row && t.row != tiles[0].row
                        });
                    Some(Clue::one_matches_either(seed, not_tile, tiles[0]))
                }
            },
        }
    }

    pub fn generate_random_clue_type(
        &mut self,
        clue_generators: &Vec<WeightedClueType>,
        seed: Tile,
    ) -> Option<Clue> {
        let weighted_clue_type = clue_generators
            .choose_weighted(&mut self.rng, |c| c.weight)
            .unwrap();

        let mut clue = None;
        while clue.is_none() {
            clue = self.generate_clue(&weighted_clue_type.clue_type, seed);
            if clue.is_none() {
                trace!(
                    target: "clue_generator",
                    "Failed to generate clue, trying again ({:?})",
                    weighted_clue_type
                );
            }
        }
        clue
    }
}
