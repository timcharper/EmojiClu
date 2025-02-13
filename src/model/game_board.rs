use super::{
    solution::{Solution, MAX_GRID_SIZE},
    ClueAddress, ClueSet,
};
use crate::model::tile_assertion::TileAssertion;
use crate::model::{Candidate, Deduction, PartialSolution, Tile};
use std::{collections::HashSet, rc::Rc};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct GameBoard {
    candidates: [[u8; MAX_GRID_SIZE]; MAX_GRID_SIZE],
    resolved_candidates: [[u8; MAX_GRID_SIZE]; MAX_GRID_SIZE],
    selected: [[Option<char>; MAX_GRID_SIZE]; MAX_GRID_SIZE],
    pub solution: Rc<Solution>,
    pub clue_set: Rc<ClueSet>,
    pub completed_clues: HashSet<ClueAddress>,
}

impl std::fmt::Debug for GameBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = String::new();
        output.push('\n');

        for row in 0..self.solution.n_rows {
            // Row header
            output.push_str(&format!("{}|", row));

            // Cells
            for col in 0..self.solution.n_variants {
                if let Some(tile) = self.selected[row][col] {
                    output.push_str(&format!(
                        "{:<width$}|",
                        format!("<{}>", tile.to_ascii_uppercase()),
                        width = self.solution.n_variants
                    ));
                } else {
                    let mut cell = String::new();
                    for variant in self.solution.variants.iter() {
                        if self.is_candidate_available(row, col, *variant) {
                            cell.push(*variant);
                        } else {
                            cell.push(' ');
                        }
                    }
                    output.push_str(&format!("{}|", cell));
                }
            }
            output.push('\n');
            output.push_str(&"-".repeat(self.solution.n_variants * self.solution.n_variants));
            output.push('\n');
        }

        write!(f, "{}", output)
    }
}

impl Default for GameBoard {
    fn default() -> Self {
        let candidates = [[0xFF; MAX_GRID_SIZE]; MAX_GRID_SIZE];
        let resolved_candidates = [[0x00; MAX_GRID_SIZE]; MAX_GRID_SIZE];
        let selected = std::array::from_fn(|_| std::array::from_fn(|_| None));
        let solution = Rc::new(Solution::default());
        let clue_set = Rc::new(ClueSet::new(vec![]));
        let completed_clues = HashSet::new();

        Self {
            candidates,
            resolved_candidates,
            selected,
            solution,
            clue_set,
            completed_clues,
        }
    }
}

impl GameBoard {
    pub fn new(solution: Rc<Solution>) -> Self {
        let candidates = [[0xFF; MAX_GRID_SIZE]; MAX_GRID_SIZE];
        let resolved_candidates = [[0x00; MAX_GRID_SIZE]; MAX_GRID_SIZE];
        let selected = std::array::from_fn(|_| std::array::from_fn(|_| None));

        let mut board = Self {
            candidates,
            resolved_candidates,
            selected,
            solution,
            clue_set: Rc::new(ClueSet::new(vec![])),
            completed_clues: HashSet::new(),
        };
        board.recompute_resolved();
        board
    }

    pub fn remove_candidate(&mut self, col: usize, tile: Tile) {
        let tile_idx = Tile::variant_to_usize(tile.variant);
        self.candidates[tile.row][col] &= !(1 << tile_idx);
        self.recompute_resolved_row(tile.row);
    }

    pub fn show_candidate(&mut self, col: usize, tile: Tile) {
        let tile_idx = Tile::variant_to_usize(tile.variant);
        self.candidates[tile.row][col] |= 1 << tile_idx;
        self.recompute_resolved_row(tile.row);
    }

    fn recompute_resolved(&mut self) {
        for row in 0..self.solution.n_rows {
            self.recompute_resolved_row(row);
        }
    }

    fn recompute_resolved_row(&mut self, row: usize) {
        let row_selections = self.selected[row]
            .iter()
            .flatten()
            .map(|&variant| 1u8 << Tile::variant_to_usize(variant))
            .fold(0u8, |acc, bit| acc | bit);

        for col in 0..self.solution.n_variants {
            match self.selected[row][col] {
                Some(selected_variant) => {
                    // Only the selected variant is available here
                    let variant_idx = Tile::variant_to_usize(selected_variant);
                    self.resolved_candidates[row][col] = 1 << variant_idx;
                }
                None => {
                    // Available candidates are those that are still candidates and not selected elsewhere in row
                    self.resolved_candidates[row][col] =
                        self.candidates[row][col] & !row_selections;
                }
            }
        }
    }

    pub fn get_candidate(&self, row: usize, col: usize, variant: char) -> Option<Candidate> {
        let variant_idx = Tile::variant_to_usize(variant);
        if variant_idx >= self.solution.n_variants
            || row >= self.solution.n_rows
            || col >= self.solution.n_variants
        {
            return None;
        }
        return Some(Candidate::from_bool(
            row,
            variant,
            (self.resolved_candidates[row][col] & (1 << variant_idx)) != 0,
        ));
    }

    pub fn is_candidate_available(&self, row: usize, col: usize, variant: char) -> bool {
        let variant_idx = Tile::variant_to_usize(variant);
        (self.resolved_candidates[row][col] & (1 << variant_idx)) != 0
    }

    pub fn get_variants(&self) -> Vec<char> {
        self.solution.variants.clone()
    }

    pub fn select_tile_at_position(&mut self, col: usize, tile: Tile) {
        self.selected[tile.row][col] = Some(tile.variant);
        self.recompute_resolved_row(tile.row);
    }

    pub fn select_tile_from_solution(&mut self, tile: Tile) {
        let row = tile.row;
        let col = self.solution.grid[row]
            .iter()
            .position(|t| t == &tile.variant)
            .unwrap();
        self.selected[row][col] = Some(tile.variant);
        self.recompute_resolved_row(row);
    }

    pub fn auto_solve_all(&mut self) -> (usize, Vec<(usize, Tile)>) {
        let mut iterations = 0;
        let mut selections = Vec::new();
        for row in 0..MAX_GRID_SIZE {
            let (row_iterations, row_selections) = self.auto_solve_row(row);
            iterations += row_iterations;
            selections.extend(row_selections);
        }

        (iterations, selections)
    }

    pub fn auto_solve_row(&mut self, row: usize) -> (usize, Vec<(usize, Tile)>) {
        let mut iterations = 0;
        let mut selections = Vec::new();
        while iterations < 64 {
            let mut found_solution = false;

            // look in row where there's only one possible place a variant can be
            for variant in self.solution.variants_range.clone() {
                if self.has_tile_selected_anywhere(&Tile::new(row, variant)) {
                    // already selected, nothing to do
                    continue;
                }

                let mut col_candidates = Vec::new();
                for col in 0..self.solution.n_variants {
                    if !self.has_negative_deduction(&Tile::new(row, variant), col) {
                        col_candidates.push(col);
                    }
                }

                if col_candidates.len() == 1 {
                    let col = col_candidates[0];
                    let tile = Tile::new(row, variant);
                    self.select_tile_at_position(col, tile);
                    selections.push((col, tile));
                    found_solution = true;
                }
            }

            // look in cells with only one remaining variant
            for col in 0..self.solution.n_variants {
                // if column has a selection, nothing to do
                if self.selected[row][col].is_some() {
                    continue;
                }

                // Count available candidates by checking each variant
                let mut available_candidates = Vec::new();
                for variant in self.solution.variants.iter() {
                    if self.is_candidate_available(row, col, *variant) {
                        available_candidates.push(Tile::new(row, *variant));
                    }
                }

                // If only one candidate remains, select it and continue auto-solving
                if available_candidates.len() == 1 {
                    let tile = available_candidates[0];
                    self.select_tile_at_position(col, tile);
                    selections.push((col, tile));
                    found_solution = true; // indicate that we've found a solution this sweep of the row, so we should keep on solving
                }
            }

            if !found_solution {
                return (iterations, selections);
            }
            iterations += 1;
        }

        println!("Something went wrong, auto-solve completed after 64 tries");
        return (iterations, selections);
    }

    #[cfg(test)]
    pub fn parse(input: &str, solution: Rc<Solution>) -> Self {
        let mut selected: [[Option<char>; MAX_GRID_SIZE]; MAX_GRID_SIZE] =
            std::array::from_fn(|_| std::array::from_fn(|_| None));
        let mut candidates = [[0xFF; MAX_GRID_SIZE]; MAX_GRID_SIZE];
        let resolved_candidates = [[0x00; MAX_GRID_SIZE]; MAX_GRID_SIZE];
        let lines: Vec<&str> = input.lines().collect();
        let mut row = 0;

        for line in lines {
            if line.starts_with('-') {
                continue;
            }

            if !line.starts_with(char::is_numeric) {
                continue;
            }

            let cells: Vec<&str> = line[2..].split('|').filter(|c| !c.is_empty()).collect();
            for (col, cell) in cells.iter().enumerate() {
                let cell = cell.trim();

                // Check for selected tile
                if cell.starts_with('<') && cell.ends_with('>') {
                    let variant = cell[1..2].chars().next().unwrap().to_ascii_lowercase();
                    selected[row][col] = Some(variant);
                    continue;
                }

                // Parse available candidates
                candidates[row][col] = 0;
                for (idx, c) in solution.variants.iter().enumerate() {
                    if cell.contains(*c) {
                        candidates[row][col] |= 1 << idx;
                    }
                }
            }
            row += 1;
        }

        let mut board = Self {
            solution,
            selected,
            candidates,
            resolved_candidates,
            clue_set: Rc::new(ClueSet::new(vec![])),
            completed_clues: HashSet::new(),
        };
        board.recompute_resolved();
        board
    }

    pub fn set_clues(&mut self, clues: Rc<ClueSet>) {
        self.clue_set = clues;
    }

    /// Check if a tile has already been placed in any column
    pub fn has_tile_selected_anywhere(&self, tile: &Tile) -> bool {
        let row = tile.row as usize;
        let selected = self.selected[row]
            .iter()
            .any(|selected| selected.as_ref() == Some(&tile.variant));

        selected
    }

    /// Check if a tile has been selected in a specific column
    ///
    /// # Arguments
    /// * `tile` - The tile to check for
    /// * `column` - The column to check for
    ///
    /// # Returns
    /// `true` if the tile has been selected in the column, `false` otherwise
    pub fn is_selected_in_column(&self, tile: &Tile, column: usize) -> bool {
        let row = tile.row as usize;
        let selected = self.selected[row][column];

        if let Some(selected_tile) = selected {
            selected_tile == tile.variant
        } else {
            false
        }
    }

    /// Check if a tile has been eliminated from a specific column
    pub fn has_negative_deduction(&self, tile: &Tile, column: usize) -> bool {
        return !self.is_candidate_available(tile.row, column, tile.variant);
    }

    pub fn apply_partial_solution(&mut self, solution: &PartialSolution) {
        for (column, tile_assertion) in solution.iter() {
            if tile_assertion.assertion {
                self.select_tile_at_position(*column, tile_assertion.tile);
            } else {
                self.remove_candidate(*column, tile_assertion.tile);
            }
        }
    }

    pub fn apply_deduction(&mut self, deduction: &Deduction) {
        if deduction.tile_assertion.assertion {
            self.select_tile_at_position(deduction.column, deduction.tile_assertion.tile);
        } else {
            self.remove_candidate(deduction.column, deduction.tile_assertion.tile);
        }
    }

    pub fn is_known_deduction(&self, column: usize, tile_assertion: TileAssertion) -> bool {
        if tile_assertion.assertion {
            self.is_selected_in_column(&tile_assertion.tile, column)
        } else {
            self.has_negative_deduction(&tile_assertion.tile, column)
        }
    }

    pub fn is_valid_row_possibility(&self, row: usize) -> bool {
        for col in 0..self.solution.n_variants {
            let mut col_has_candidate = false;
            for variant in self.solution.variants.iter() {
                if self.is_candidate_available(row, col, *variant) {
                    col_has_candidate = true;
                }
            }
            if !col_has_candidate {
                return false;
            }
        }
        true
    }

    pub fn is_valid_possibility(&self) -> bool {
        // check if all rows have at least one candidate
        for row in 0..self.solution.n_rows {
            if !self.is_valid_row_possibility(row) {
                return false;
            }
        }
        true
    }

    /// Checks if the game board is fully solved.
    pub(crate) fn is_complete(&self) -> bool {
        for row in 0..self.solution.n_rows {
            for col in 0..self.solution.n_variants {
                if self.selected[row][col].is_none() {
                    return false;
                }
            }
        }
        true
    }

    pub(crate) fn apply_deductions(&mut self, deductions: &[Deduction]) {
        for deduction in deductions {
            self.apply_deduction(deduction);
        }
    }

    /// returns final state, is clue completed
    pub(crate) fn toggle_clue_completed(&mut self, clue_address: ClueAddress) -> bool {
        let cwa = self.clue_set.get_clue(clue_address);
        if !cwa.is_some() {
            return false;
        }

        let is_completed = if !self.completed_clues.remove(&clue_address) {
            self.completed_clues.insert(clue_address.clone());
            true
        } else {
            false
        };
        is_completed
    }

    pub(crate) fn is_clue_completed(&self, clue_address: &ClueAddress) -> bool {
        self.completed_clues.contains(clue_address)
    }

    /// Check if the board is incorrect. Returns false for boards that are not complete, but have no errors.
    pub(crate) fn is_incorrect(&self) -> bool {
        for row in 0..self.solution.n_rows {
            for col in 0..self.solution.n_variants {
                let solution_tile = self.solution.get(row, col);
                // check selections
                if let Some(selected_variant) = self.selected[row][col] {
                    if selected_variant != solution_tile.variant {
                        return true;
                    }
                } else {
                    if !self.is_candidate_available(row, col, solution_tile.variant) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn get_selected_tiles(&self) -> Vec<Tile> {
        let mut tiles = Vec::new();
        for row in 0..self.solution.n_rows {
            for col in 0..self.solution.n_variants {
                if let Some(variant) = self.selected[row][col] {
                    tiles.push(Tile::new(row, variant));
                }
            }
        }
        tiles
    }

    pub fn get_selection(&self, row: usize, col: usize) -> Option<Tile> {
        if let Some(variant) = self.selected[row][col] {
            Some(Tile::new(row, variant))
        } else {
            None
        }
    }

    pub fn has_selection(&self, row: usize, col: usize) -> bool {
        self.selected[row][col].is_some()
    }

    pub(crate) fn remove_selection(&mut self, row: usize, col: usize) {
        self.selected[row][col] = None;
        self.recompute_resolved_row(row);
    }

    pub(crate) fn completed_clues(&self) -> &HashSet<ClueAddress> {
        &self.completed_clues
    }

    pub(crate) fn get_possible_cols_for_tile(
        &self,
        tile: Tile,
    ) -> impl Iterator<Item = usize> + '_ {
        (0..self.solution.n_variants)
            .filter(move |col| self.is_candidate_available(tile.row, *col, tile.variant))
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{CandidateState, Difficulty};

    use super::*;

    fn create_test_solution() -> Rc<Solution> {
        let mut grid = [['0'; MAX_GRID_SIZE]; MAX_GRID_SIZE];
        // Fill first 4x4 of grid with test data
        for row in 0..4 {
            for col in 0..4 {
                grid[row][col] = Tile::usize_to_variant(col);
            }
        }

        Rc::new(Solution {
            variants: vec!['a', 'b', 'c', 'd'],
            grid,
            n_rows: 4,
            n_variants: 4,
            variants_range: 'a'..='d',
            difficulty: Difficulty::Easy,
            seed: 0,
        })
    }

    #[test]
    fn test_parse_empty_board() {
        let input = "\
1|abcd|abcd|abcd|abcd|
-----------------
2|abcd|abcd|abcd|abcd|
-----------------
3|abcd|abcd|abcd|abcd|
-----------------
4|abcd|abcd|abcd|abcd|";

        let board = GameBoard::parse(input, create_test_solution());

        // Check no tiles are selected
        for row in 0..4 {
            for col in 0..4 {
                assert!(board.selected[row][col].is_none());
                // Check all candidates are available
                for variant in ['a', 'b', 'c', 'd'] {
                    let candidate = board.get_candidate(row, col, variant).unwrap();
                    assert_eq!(candidate.state, CandidateState::Available);
                }
            }
        }
    }

    #[test]
    fn test_parse_with_selected_tiles() {
        let input = "\
0|<A>|abcd|abcd|abcd|
-----------------
1|abcd|<B>|abcd|abcd|
-----------------
2|abcd|abcd|<C>|abcd|
-----------------
3|abcd|abcd|abcd|<D>|";

        let board = GameBoard::parse(input, create_test_solution());

        // Check selected tiles
        assert_eq!(board.selected[0][0], Some('a'));
        assert_eq!(board.selected[1][1], Some('b'));
        assert_eq!(board.selected[2][2], Some('c'));
        assert_eq!(board.selected[3][3], Some('d'));

        // Check other positions are empty
        for row in 0..4 {
            for col in 0..4 {
                if row != col {
                    assert!(board.selected[row][col].is_none());
                }
            }
        }
    }

    #[test]
    fn test_parse_with_eliminated_candidates() {
        let input = "\
0|a   |abcd|abcd|abcd|
-----------------
1|abcd|b   |abcd|abcd|
-----------------
2|abcd|abcd|c   |abcd|
-----------------
3|abcd|abcd|abcd|d   |";

        let board = GameBoard::parse(input, create_test_solution());

        // Check specific cells with eliminated candidates
        let check_cell = |row: usize, col: usize, available: char| {
            for variant in ['a', 'b', 'c', 'd'] {
                let candidate = board.get_candidate(row, col, variant).unwrap();
                if variant == available {
                    assert_eq!(candidate.state, CandidateState::Available);
                } else {
                    assert_eq!(candidate.state, CandidateState::Eliminated);
                }
            }
        };

        check_cell(0, 0, 'a');
        check_cell(1, 1, 'b');
        check_cell(2, 2, 'c');
        check_cell(3, 3, 'd');
    }

    #[test]
    fn test_auto_solve_row_simple() {
        let input = "\
0|a   |abcd|abcd|abcd|
-----------------
1|abcd|b   |abcd|abcd|
-----------------
2|abcd|abcd|c   |abcd|
-----------------
3|abcd|abcd|abcd|d   |";

        let mut board = GameBoard::parse(input, create_test_solution());

        println!("Board before solving row 0: {:?}", board);
        // Auto-solve row 0 - should select 'a' in first column since it's the only candidate
        let (iterations, _) = board.auto_solve_row(0);
        assert_eq!(iterations, 1);

        // Verify first cell has 'a' selected
        assert!(board.is_selected_in_column(&Tile::parse("0a"), 0));
        assert!(board.has_negative_deduction(&Tile::parse("0a"), 1));
        assert!(board.has_negative_deduction(&Tile::parse("0a"), 2));
        assert!(board.has_negative_deduction(&Tile::parse("0a"), 3));

        // Verify 'a' candidacy removed from

        println!("Board after solving row 0: {:?}", board);
    }

    #[test]
    fn test_auto_solve_row_cascade() {
        // Set up a more complex scenario where solving one cell reveals another
        let input2 = "\
0|abcd|abc |ab  |a   |
-----------------
1|abcd|abcd|abcd|abcd|
-----------------
2|abcd|abcd|abcd|abcd|
-----------------
3|abcd|abcd|abcd|abcd|";

        let mut board = GameBoard::parse(input2, create_test_solution());

        println!("Board before solving row 0: {:?}", board);
        // Auto-solve row 0 - should cascade: first 'a', then 'b', then 'c', then 'd'
        let (iterations, _) = board.auto_solve_row(0);
        assert!(iterations <= 4); // assert that we're properly deducing when there is nothing else to do

        println!("Board after solving row 0: {:?}", board);

        // Verify first three cells are selected
        assert!(board.is_selected_in_column(&Tile::parse("0a"), 3));
        assert!(board.is_selected_in_column(&Tile::parse("0b"), 2));
        assert!(board.is_selected_in_column(&Tile::parse("0c"), 1));
        assert!(board.is_selected_in_column(&Tile::parse("0d"), 0));
    }

    #[test]
    fn test_auto_solve_row_last_candidate() {
        // Set up a more complex scenario where solving one cell reveals another
        let input2 = "\
0|abcd|abc |abc |abc |
-----------------
1|abcd|abcd|abcd|abcd|
-----------------
2|abcd|abcd|abcd|abcd|
-----------------
3|abcd|abcd|abcd|abcd|";

        let mut board = GameBoard::parse(input2, create_test_solution());

        println!("Board before solving row 0: {:?}", board);
        // Auto-solve row 0 - should cascade: first 'a', then 'b', then 'c', then 'd'
        let (iterations, _) = board.auto_solve_row(0);
        assert_eq!(iterations, 1); // assert that we're properly deducing when there is nothing else to do

        println!("Board after solving row 0: {:?}", board);

        // Verify first three cells are selected
        assert!(board.is_selected_in_column(&Tile::parse("0d"), 0));
    }
}
