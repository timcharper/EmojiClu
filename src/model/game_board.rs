use super::{
    solution::{Solution, MAX_GRID_SIZE},
    Clue, ClueSet,
};
use crate::model::{Candidate, CandidateState, Deduction, PartialSolution, Tile};
use std::{collections::HashSet, rc::Rc};

#[derive(Clone)]
pub struct GameBoard {
    pub candidates: [[Vec<Candidate>; MAX_GRID_SIZE]; MAX_GRID_SIZE],
    pub selected: [[Option<Tile>; MAX_GRID_SIZE]; MAX_GRID_SIZE],
    pub solution: Rc<Solution>,
    pub clue_set: Rc<ClueSet>,
    pub completed_horizontal_clues: HashSet<usize>,
    pub completed_vertical_clues: HashSet<usize>,
    pub completed_clues: HashSet<Clue>,
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
                        format!("<{}>", tile.variant.to_ascii_uppercase()),
                        width = self.solution.n_variants
                    ));
                } else {
                    let mut cell = String::new();
                    for variant in self.solution.variants.iter() {
                        if let Some(candidate) = self.get_candidate(row, col, *variant) {
                            if candidate.state == CandidateState::Available {
                                cell.push(candidate.tile.variant);
                            } else {
                                cell.push(' ');
                            }
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

impl GameBoard {
    pub fn new(solution: Rc<Solution>) -> Self {
        let mut candidates: [[Vec<Candidate>; MAX_GRID_SIZE]; MAX_GRID_SIZE] =
            std::array::from_fn(|_| std::array::from_fn(|_| Vec::new()));
        let selected = std::array::from_fn(|_| std::array::from_fn(|_| None));

        for row in 0..MAX_GRID_SIZE {
            for col in 0..MAX_GRID_SIZE {
                for &variant in solution.variants.iter() {
                    candidates[row][col].push(Candidate::new(Tile::new(row, variant)));
                }
            }
        }

        Self {
            candidates,
            selected,
            solution,
            clue_set: Rc::new(ClueSet::new(vec![])),
            completed_horizontal_clues: HashSet::new(),
            completed_vertical_clues: HashSet::new(),
            completed_clues: HashSet::new(),
        }
    }

    pub fn remove_candidate(&mut self, row: usize, col: usize, tile: Tile) {
        if let Some(candidate) = self.candidates[row][col]
            .iter_mut()
            .find(|c| c.tile == tile)
        {
            candidate.state = CandidateState::Eliminated;
        }
    }

    pub fn show_candidate(&mut self, row: usize, col: usize, tile: Tile) {
        if let Some(candidate) = self.candidates[row][col]
            .iter_mut()
            .find(|c| c.tile == tile)
        {
            candidate.state = CandidateState::Available;
        }
    }

    pub fn get_candidate(&self, row: usize, col: usize, variant: char) -> Option<Candidate> {
        // this cell has a selection
        if let Some(selected_tile) = self.selected[row][col] {
            // is this the selection?
            if selected_tile.variant == variant {
                return Some(Candidate {
                    tile: selected_tile,
                    state: CandidateState::Available,
                });
            } else {
                return Some(Candidate {
                    tile: selected_tile,
                    state: CandidateState::Eliminated,
                });
            }
        }

        let maybe_candidate = self.candidates[row][col]
            .iter()
            .find(|c| c.tile.variant == variant);

        if let Some(candidate) = maybe_candidate {
            let mut candidate = candidate.clone();
            // if another cell in this row has selected this variant as a solution, return state as unavailable
            if self.selected[row]
                .iter()
                .any(|c| c.is_some() && c.unwrap().variant == variant)
            {
                candidate.state = CandidateState::Eliminated;
            }
            Some(candidate)
        } else {
            None
        }
    }

    pub fn get_variants(&self) -> Vec<char> {
        self.solution.variants.clone()
    }

    pub fn select_tile_at_position(&mut self, row: usize, col: usize, tile: Tile) {
        self.selected[row][col] = Some(tile);
    }

    pub fn select_tile_from_solution(&mut self, tile: Tile) {
        let row = tile.row;
        let col = self.solution.grid[row]
            .iter()
            .position(|t| t == &tile)
            .unwrap();
        self.selected[row][col] = Some(tile);
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
                if self.has_selection(&Tile::new(row, variant)) {
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
                    self.select_tile_at_position(row, col, tile);
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
                    if let Some(candidate) = self.get_candidate(row, col, *variant) {
                        if candidate.state == CandidateState::Available {
                            available_candidates.push(candidate.tile);
                        }
                    }
                }

                // If only one candidate remains, select it and continue auto-solving
                if available_candidates.len() == 1 {
                    let tile = available_candidates[0];
                    self.select_tile_at_position(row, col, tile);
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
        let mut selected = std::array::from_fn(|_| std::array::from_fn(|_| None));
        let mut candidates = std::array::from_fn(|_| std::array::from_fn(|_| Vec::new()));

        let lines: Vec<&str> = input.lines().collect();
        let mut row = 0;

        for line in lines {
            if line.starts_with('-') {
                continue;
            }

            if !line.starts_with(char::is_numeric) {
                continue;
            }

            let cells: Vec<&str> = line[2..].split('|').collect();
            for (col, cell) in cells.iter().enumerate() {
                let cell = cell.trim();

                // Check for selected tile
                if cell.starts_with('<') && cell.ends_with('>') {
                    let variant = cell[1..2].chars().next().unwrap().to_ascii_lowercase();
                    selected[row][col] = Some(Tile::new(row, variant));
                    continue;
                }

                // Parse available candidates
                let mut cell_candidates = Vec::new();
                for (_, c) in "abcdef".chars().enumerate() {
                    if cell.contains(c) {
                        cell_candidates.push(Candidate {
                            tile: Tile::new(row, c),
                            state: CandidateState::Available,
                        });
                    } else {
                        cell_candidates.push(Candidate {
                            tile: Tile::new(row, c),
                            state: CandidateState::Eliminated,
                        });
                    }
                }
                candidates[row][col] = cell_candidates;
            }
            row += 1;
        }

        Self {
            solution,
            selected,
            candidates,
            clue_set: Rc::new(ClueSet::new(vec![])),
            completed_horizontal_clues: HashSet::new(),
            completed_vertical_clues: HashSet::new(),
            completed_clues: HashSet::new(),
        }
    }

    pub fn set_clues(&mut self, clues: Rc<ClueSet>) {
        self.clue_set = clues;
    }

    /// Check if a tile has already been placed in any column
    pub fn has_selection(&self, tile: &Tile) -> bool {
        let row = tile.row as usize;
        let selected = self.selected[row]
            .iter()
            .any(|selected| selected.as_ref() == Some(tile));

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
            selected_tile.variant == tile.variant
        } else {
            false
        }
    }

    /// Check if a tile has been eliminated from a specific column
    pub fn has_negative_deduction(&self, tile: &Tile, column: usize) -> bool {
        let row = tile.row;
        let col = column;

        let candidate = self.get_candidate(row, col, tile.variant);
        // Use get_candidate to check if this tile is eliminated in this position
        if let Some(candidate) = candidate {
            candidate.state == CandidateState::Eliminated
        } else {
            // If we can't find the candidate, consider it eliminated
            true
        }
    }

    pub fn apply_partial_solution(&mut self, solution: &PartialSolution) {
        for (column, tile_assertion) in solution.iter() {
            if tile_assertion.assertion {
                self.select_tile_at_position(tile_assertion.tile.row, *column, tile_assertion.tile);
            } else {
                self.remove_candidate(tile_assertion.tile.row, *column, tile_assertion.tile);
            }
        }
    }

    pub fn apply_deduction(&mut self, deduction: &Deduction) {
        if deduction.is_positive {
            self.select_tile_at_position(
                deduction.tile.row,
                deduction.column,
                deduction.tile.clone(),
            );
        } else {
            self.remove_candidate(deduction.tile.row, deduction.column, deduction.tile.clone());
        }
    }

    pub fn is_valid_row_possibility(&self, row: usize) -> bool {
        for col in 0..self.solution.n_variants {
            let mut col_has_candidate = false;
            for variant in self.solution.variants.iter() {
                if let Some(candidate) = self.get_candidate(row, col, *variant) {
                    if candidate.state == CandidateState::Available {
                        col_has_candidate = true;
                    }
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
    pub(crate) fn toggle_clue_completed(
        &mut self,
        clue_orientation: super::ClueOrientation,
        clue_idx: usize,
    ) -> bool {
        let cwg = self.clue_set.get_clue(clue_orientation, clue_idx);
        if !cwg.is_some() {
            return false;
        }
        let clue = &cwg.unwrap().clue;

        let is_completed = match clue_orientation {
            super::ClueOrientation::Horizontal => {
                if !self.completed_horizontal_clues.remove(&clue_idx) {
                    self.completed_horizontal_clues.insert(clue_idx);
                    true
                } else {
                    false
                }
            }
            super::ClueOrientation::Vertical => {
                if !self.completed_vertical_clues.remove(&clue_idx) {
                    self.completed_vertical_clues.insert(clue_idx);
                    true
                } else {
                    false
                }
            }
        };
        if is_completed {
            self.completed_clues.insert(clue.clone());
        } else {
            self.completed_clues.remove(clue);
        }
        is_completed
    }

    pub(crate) fn is_clue_completed(
        &self,
        clue_orientation: super::ClueOrientation,
        clue_idx: usize,
    ) -> bool {
        match clue_orientation {
            super::ClueOrientation::Horizontal => {
                self.completed_horizontal_clues.contains(&clue_idx)
            }
            super::ClueOrientation::Vertical => self.completed_vertical_clues.contains(&clue_idx),
        }
    }

    /// Check if the board is incorrect. Returns false for boards that are not complete, but have no errors.
    pub(crate) fn is_incorrect(&self) -> bool {
        for row in 0..self.solution.n_rows {
            for col in 0..self.solution.n_variants {
                let solution_tile = self.solution.get(row, col);
                // check selections
                if let Some(selected_tile) = self.selected[row][col] {
                    if selected_tile != solution_tile {
                        return true;
                    }
                } else {
                    // check candidates
                    if self
                        .get_candidate(row, col, solution_tile.variant)
                        .unwrap()
                        .state
                        != CandidateState::Available
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn get_selected_tiles(&self) -> Vec<Tile> {
        self.selected
            .iter()
            .flat_map(|row| row.iter().filter_map(|tile| tile.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::model::Difficulty;

    use super::*;

    fn create_test_solution() -> Rc<Solution> {
        let mut grid = [[Tile::new(0, '0'); MAX_GRID_SIZE]; MAX_GRID_SIZE];
        // Fill first 4x4 of grid with test data
        for row in 0..4 {
            for col in 0..4 {
                grid[row][col] = Tile::new(row, (b'a' + col as u8) as char);
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
        assert_eq!(board.selected[0][0], Some(Tile::new(0, 'a')));
        assert_eq!(board.selected[1][1], Some(Tile::new(1, 'b')));
        assert_eq!(board.selected[2][2], Some(Tile::new(2, 'c')));
        assert_eq!(board.selected[3][3], Some(Tile::new(3, 'd')));

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
