use log::{info, trace};
use rand::{
    rngs::StdRng,
    seq::{IteratorRandom, SliceRandom},
    Rng, RngCore, SeedableRng,
};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    game::solver::{perform_evaluation_step, EvaluationStepResult},
    model::{
        Clue, ClueOrientation, ClueType, Deduction, GameBoard, HorizontalClueType, Tile,
        TileAssertion, VerticalClueType,
    },
};

use super::puzzle_variants::WeightedClueType;

pub const MAX_HORIZ_CLUES: usize = 96;
pub const MAX_VERT_CLUES: usize = 48;
const MAX_HORIZONTAL_TILE_USAGE: usize = 3;
const MAX_VERTICAL_TILE_USAGE: usize = 2;

#[derive(Debug, Default)]
pub struct ClueGeneratorStats {
    pub n_rejected_no_deductions: usize,
    pub n_rejected_tile_usage_horiz: usize,
    pub n_rejected_tile_usage_vert: usize,
    pub n_rejected_max_vert: usize,
    pub n_rejected_max_horiz: usize,
    pub n_rejected_non_singleton_intersecting_clues: usize,
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
    pub horizontal_clues: usize,
    pub vertical_clues: usize,
    pub unsolved_columns: BTreeSet<usize>,
    pub unsolved_rows: BTreeSet<usize>,
    pub selection_count_by_row: Vec<usize>,
    pub selection_count_by_column: Vec<usize>,
    pub unsolved_tiles: BTreeSet<Tile>,
    pub unsolved_tiles_by_column: BTreeMap<usize, BTreeSet<Tile>>,
    pub unsolved_tiles_by_row: BTreeMap<usize, BTreeSet<Tile>>,
    pub tile_horiz_usage_remaining: BTreeMap<Tile, usize>,
    pub tile_vert_usage_remaining: BTreeMap<Tile, usize>,
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
        let mut unsolved_tiles_by_column: BTreeMap<usize, BTreeSet<Tile>> = BTreeMap::new();
        let mut unsolved_tiles_by_row: BTreeMap<usize, BTreeSet<Tile>> = BTreeMap::new();
        let mut tiles_without_evidence: BTreeSet<(usize, Tile)> = BTreeSet::new();
        let mut tile_horiz_usage_remaining: BTreeMap<Tile, usize> = BTreeMap::new();
        let mut tile_vert_usage_remaining: BTreeMap<Tile, usize> = BTreeMap::new();

        for row in 0..board.solution.n_rows {
            for col in 0..board.solution.n_variants {
                let tile = board.solution.get(row, col);
                tiles_without_evidence.insert((col, tile));
                unsolved_tiles_by_column
                    .entry(col)
                    .or_insert(BTreeSet::new())
                    .insert(tile);
                unsolved_tiles_by_row
                    .entry(row)
                    .or_insert(BTreeSet::new())
                    .insert(tile);
                tile_horiz_usage_remaining.insert(tile, MAX_HORIZONTAL_TILE_USAGE);
                tile_vert_usage_remaining.insert(tile, MAX_VERTICAL_TILE_USAGE);
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
            horizontal_clues: 0,
            vertical_clues: 0,
            unsolved_columns,
            unsolved_rows,
            unsolved_tiles,
            unsolved_tiles_by_column,
            unsolved_tiles_by_row,
            tile_horiz_usage_remaining,
            tile_vert_usage_remaining,
            stats: ClueGeneratorStats::default(),
        }
    }
    pub fn reset_stats(&mut self) {
        self.stats = ClueGeneratorStats::default();
    }

    /// pick a random tile from the board, prioritizing tiles with evidence from unsolved columns / rows
    pub fn random_tile_with_evidence(&mut self, orientation: ClueOrientation) -> Tile {
        trace!(
            target: "clue_generator",
            "Tiles with evidence: {:?}",
            self.tiles_with_evidence
        );

        if orientation == ClueOrientation::Horizontal {
            self.tiles_with_evidence
                .iter()
                .filter(|(_, t)| self.tile_horiz_usage_remaining.contains_key(t))
                .choose(&mut self.rng)
                .map(|(_, t)| t.clone())
                .expect("Tile horiz usage consumed and puzzle not solved? Unpossible!")
        } else {
            self.tiles_with_evidence
                .iter()
                .filter(|(_, t)| self.tile_vert_usage_remaining.contains_key(t))
                .choose(&mut self.rng)
                .map(|(_, t)| t.clone())
                .expect("Tile vert usage consumed and puzzle not solved? Unpossible!")
        }
    }

    fn consume_horiz_tile(&mut self, tile: &Tile) {
        let remaining = self.tile_horiz_usage_remaining.get_mut(tile).unwrap();
        if *remaining <= 1 {
            self.tile_horiz_usage_remaining.remove(tile);
        } else {
            *remaining -= 1;
        }
    }

    fn increment_vertical_usage(&mut self, tile: &Tile) {
        trace!(
            target: "clue_generator",
            "increment_vertical_usage: {:?}",
            tile
        );
        let remaining = self.tile_vert_usage_remaining.get_mut(tile).unwrap();
        if *remaining <= 1 {
            trace!(
                target: "clue_generator",
                "increment_vertical_usage: removing {:?}",
                tile
            );
            self.tile_vert_usage_remaining.remove(tile);
        } else {
            trace!(
                target: "clue_generator",
                "increment_vertical_usage: decrementing {:?}",
                tile
            );
            *remaining -= 1;
        }
    }

    pub(crate) fn would_exceed_usage_limits(&mut self, clue: &Clue) -> bool {
        if clue.is_horizontal() {
            // too many clues?
            if self.horizontal_clues >= MAX_HORIZ_CLUES {
                self.stats.n_rejected_max_horiz += 1;
                return true;
            }
            // too many hints on the same tile?
            let exceeds = clue
                .assertions
                .iter()
                .filter(|a| a.assertion)
                .any(|a| !self.tile_horiz_usage_remaining.contains_key(&a.tile));
            if exceeds {
                self.stats.n_rejected_tile_usage_horiz += 1;
            }
            exceeds
        } else {
            // too many clues?
            if self.vertical_clues >= MAX_VERT_CLUES {
                self.stats.n_rejected_max_vert += 1;
                return true;
            }
            // too many hints on the same tile?
            let exceeds = clue
                .assertions
                .iter()
                .filter(|a| a.assertion)
                .any(|a| !self.tile_vert_usage_remaining.contains_key(&a.tile));
            if exceeds {
                self.stats.n_rejected_tile_usage_vert += 1;
            }
            exceeds
        }
    }

    fn record_clue_usage(&mut self, clue: &Clue) {
        if clue.is_horizontal() {
            for TileAssertion { tile, assertion } in clue.assertions.iter() {
                if *assertion {
                    self.consume_horiz_tile(&tile);
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
            self.unsolved_tiles_by_column
                .get_mut(&col)
                .expect(&format!(
                    "Error - unsolved tiles not populated in column {}",
                    col
                ))
                .remove(&tile);
            self.unsolved_tiles_by_row
                .get_mut(&tile.row)
                .expect(&format!(
                    "Error - unsolved tiles not populated in row {}",
                    tile.row
                ))
                .remove(&tile);
        }
    }

    pub(crate) fn add_selected_tile(&mut self, tile: Tile) {
        trace!(
            target: "clue_generator",
            "Adding selected tile: {:?}",
            tile
        );
        let (_, column) = self.board.solution.find_tile(&tile);

        self.board.select_tile_from_solution(tile);
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

    pub fn quick_prune(&mut self, board: &GameBoard) {
        // solve puzzle backwards, any unused clues, discard
        let mut board = board.clone();
        let reversed_clues = self.clues.clone().into_iter().rev().collect::<Vec<_>>();
        let mut used_clues = BTreeSet::new();
        while !board.is_complete() {
            let deduction = perform_evaluation_step(&mut board, &reversed_clues);
            match deduction {
                EvaluationStepResult::Nothing => {
                    panic!("Puzzle not solvable! {:?}", board);
                }
                EvaluationStepResult::DeductionsFound(clue) => {
                    used_clues.insert(clue);
                }
                EvaluationStepResult::HiddenPairsFound => {
                    // nothing
                }
            }
            board.auto_solve_all();
        }
        info!(
            target: "clue_generator",
            "Quick prune reduced clues from {} to {}",
            self.clues.len(),
            used_clues.len()
        );
        self.clues.retain(|c| used_clues.contains(c));
    }

    pub fn deep_prune_clues(&mut self, board: &GameBoard) {
        let original_clue_count = self.clues.len();

        trace!(
            target: "clue_generator",
            "Original clues: {:?}",
            self.clues
        );

        // Try removing each clue one at a time
        let mut i = 0;
        while i < self.clues.len() {
            info!(
                target: "clue_generator",
                "Deep prune cycles remaining: {}",
                self.clues.len() - i
            );
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
        let mut num_unsolved_tiles = 1; // try to get at least one unsolved tile.
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
            let unsolved_tiles_in_column = self
                .unsolved_tiles_by_column
                .get(&(next_col as usize))
                .expect("No unsolved tiles in column");
            let mut maybe_tile: Option<Tile> = None;
            if num_unsolved_tiles == 0 && !unsolved_tiles_in_column.is_empty() {
                maybe_tile = unsolved_tiles_in_column
                    .iter()
                    .filter(|t| self.tile_horiz_usage_remaining.contains_key(t))
                    .choose(&mut self.rng)
                    .map(|t| t.clone());
                if maybe_tile.is_some() {
                    num_unsolved_tiles += 1;
                }
            }
            let tile = maybe_tile.unwrap_or_else(|| {
                let next_row = self.rng.gen_range(0..self.board.solution.n_rows);
                self.board.solution.get(next_row, next_col as usize)
            });

            tiles.push(tile);
            columns.push(next_col as usize);
        }
        (tiles, columns)
    }

    fn get_random_vertical_tiles(&mut self, seed: &Tile, count: usize) -> Vec<Tile> {
        let mut tiles: Vec<Tile> = Vec::new();
        let mut num_unsolved_tiles = 0; // try to get at least one unsolved tile.
        let (row, col) = self.board.solution.find_tile(seed);

        let mut possible_rows = (0..self.board.solution.n_rows)
            .filter(|&r| r != row)
            .collect::<Vec<_>>();

        possible_rows.shuffle(&mut self.rng);

        for _ in 0..count {
            let unsolved_tiles = self
                .unsolved_tiles_by_column
                .get(&col)
                .expect("No unsolved tiles in column");

            let mut maybe_unsolved_tile: Option<Tile> = None;
            if num_unsolved_tiles == 0 && !unsolved_tiles.is_empty() {
                maybe_unsolved_tile = unsolved_tiles
                    .iter()
                    .filter(|t| possible_rows.contains(&t.row))
                    .choose(&mut self.rng)
                    .map(|t| t.clone());
                if maybe_unsolved_tile.is_some() {
                    num_unsolved_tiles += 1;
                }
            }

            let selected_tile = maybe_unsolved_tile.unwrap_or_else(|| {
                let next_row = possible_rows
                    .choose(&mut self.rng)
                    .expect("no remaining possible rows; this shouldn't happen");
                self.board.solution.get(*next_row, col)
            });

            possible_rows.retain(|r| r != &selected_tile.row);
            tiles.push(selected_tile);
        }

        trace!(
            target: "clue_generator",
            "Possible rows {:?}, count: {:?}, tiles: {:?}",
            possible_rows,
            count,
            tiles
        );
        tiles
    }

    fn generate_clue(&mut self, clue_type: &ClueType, seed: Option<Tile>) -> Option<Clue> {
        match &clue_type {
            ClueType::Horizontal(tpe) => {
                let seed = seed
                    .unwrap_or_else(|| self.random_tile_with_evidence(ClueOrientation::Horizontal));

                match tpe {
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
                }
            }
            ClueType::Vertical(tpe) => {
                let seed = seed
                    .unwrap_or_else(|| self.random_tile_with_evidence(ClueOrientation::Vertical));

                match tpe {
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
                            .get_random_tile_not_from_columns(vec![seed_col as i32], |t| {
                                t != &seed
                            });
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
                }
            }
        }
    }

    pub fn generate_random_clue_type(
        &mut self,
        clue_generators: &Vec<WeightedClueType>,
        seed: Option<Tile>,
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

    pub(crate) fn random_unsolved_tile(&mut self) -> Tile {
        self.unsolved_tiles
            .iter()
            .choose(&mut self.rng)
            .unwrap()
            .clone()
    }
}
