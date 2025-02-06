use std::collections::{HashMap, HashSet};

use crate::model::{
    CandidateState, Clue, ClueType, Deduction, GameBoard, HorizontalClueType, PartialSolution,
    Tile, TileAssertion, VerticalClueType,
};
use log::trace;

fn is_known_deduction(board: &GameBoard, deduction: &Deduction) -> bool {
    let result = if deduction.is_positive {
        board.is_selected_in_column(&deduction.tile, deduction.column)
    } else {
        board.has_negative_deduction(&deduction.tile, deduction.column)
    };
    result
}

fn is_partial_solution_valid(board: &GameBoard, solution: &PartialSolution) -> bool {
    let is_valid = solution.iter().all(|(column, tile_assertion)| {
        if tile_assertion.assertion {
            // positive assertion
            if board.has_negative_deduction(&tile_assertion.tile, *column) {
                // tile can't go here? solution can't go here.
                return false;
            }
        } else {
            // negative assertion
            if board.is_selected_in_column(&tile_assertion.tile, *column) {
                // tile selected here? solution can't go here
                return false;
            }
        }
        true
    });
    if !is_valid {
        return false;
    }

    trace!(
        target: "solver",
        "Checking if partial solution creates invalid board: {:?}",
        solution
    );

    // now, clone the game board, apply the partial solution, see if it results in a valid state
    let mut board_clone = board.clone();
    board_clone.apply_partial_solution(solution);
    let is_valid = board_clone.is_valid_possibility();
    trace!(
        target: "solver",
        "Board after partial solution: {:?}",
        board_clone
    );
    trace!(target: "solver", "Is valid? {}", is_valid);
    is_valid
}

fn synthesize_deductions(
    board: &GameBoard,
    possible_columns: &Vec<(Tile, HashSet<usize>)>,
) -> Vec<Deduction> {
    let mut deductions = HashSet::new();
    let board_width = board.solution.n_variants;

    trace!(
        target: "solver",
        "Possible columns for each tile: {:?}",
        possible_columns
    );
    for (tile, possible) in possible_columns {
        // Skip if no valid positions or if all positions are possible
        if possible.is_empty() || possible.len() == board_width as usize {
            continue;
        }

        // If only one position is possible, make a positive deduction
        if possible.len() == 1 {
            let column = possible.iter().next().unwrap();
            deductions.insert(Deduction {
                tile: tile.clone(),
                column: *column,
                is_positive: true,
            });
        } else {
            // Make negative deductions for impossible columns
            for col in 0..board_width {
                if !possible.contains(&col) {
                    deductions.insert(Deduction {
                        tile: tile.clone(),
                        column: col,
                        is_positive: false,
                    });
                }
            }
        }
    }

    deductions
        .into_iter()
        .filter(|deducation| !is_known_deduction(board, deducation))
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coordinates {
    pub row: usize,
    pub column: usize,
}

impl std::fmt::Debug for Coordinates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{},{}}}", self.row, self.column)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct CellSolutionAssertion {
    pub positive_variants: HashSet<char>,
    pub negative_variants: HashSet<char>,
    pub positive_count: usize,
    pub negative_count: usize,
}

trait ClueHandler: std::fmt::Debug {
    fn handle(&self, board: &GameBoard, column: usize) -> Vec<Vec<(usize, TileAssertion)>>;
}

#[derive(Clone, Debug)]
struct AdjacentHandler {
    clue: Clue,
}

impl AdjacentHandler {
    fn new(clue: &Clue) -> Self {
        Self { clue: clue.clone() }
    }
}

#[derive(Clone, Debug)]
struct NotAdjacentHandler {
    positive_tile: Tile,
    negative_tile: Tile,
}

impl NotAdjacentHandler {
    fn new(clue: &Clue) -> Self {
        assert_eq!(
            clue.assertions.len(),
            2,
            "Clue assertions must have exactly 2 elements"
        );
        let o_positive_assertion = &clue.assertions.iter().find(|ta| ta.assertion);
        let o_negative_assertion = &clue.assertions.iter().find(|ta| !ta.assertion);
        if let (Some(positive_assertion), Some(negative_assertion)) =
            (o_positive_assertion, o_negative_assertion)
        {
            Self {
                positive_tile: positive_assertion.tile,
                negative_tile: negative_assertion.tile,
            }
        } else {
            panic!("Clue assertions must have exactly 2 elements, one positive and one negative");
        }
    }
}

#[derive(Clone, Debug)]
struct LeftOfHandler {
    left_tile: Tile,
    right_tile: Tile,
}

impl LeftOfHandler {
    fn new(clue: &Clue) -> Self {
        assert_eq!(
            clue.assertions.len(),
            2,
            "Clue assertions must have exactly 2 elements"
        );
        let left_tile = clue.assertions[0].tile;
        let right_tile = clue.assertions[1].tile;
        Self {
            left_tile,
            right_tile,
        }
    }
}

impl ClueHandler for LeftOfHandler {
    fn handle(&self, board: &GameBoard, column: usize) -> Vec<Vec<(usize, TileAssertion)>> {
        let max_column = board.solution.n_variants - 1;
        let mut solutions = Vec::new();

        // Skip if we're at the last column - can't have a right tile
        if column >= max_column {
            return solutions;
        }

        // Check if the left tile can go in this column
        if let Some(candidate) =
            board.get_candidate(self.left_tile.row, column, self.left_tile.variant)
        {
            if candidate.state == CandidateState::Eliminated {
                return solutions;
            }
        }

        // For each possible right column (all columns after this one)
        for right_col in (column + 1)..=max_column {
            // Create a solution with the left tile in current column and right tile in right_col
            solutions.push(vec![
                (
                    column,
                    TileAssertion {
                        tile: self.left_tile,
                        assertion: true,
                    },
                ),
                (
                    right_col,
                    TileAssertion {
                        tile: self.right_tile,
                        assertion: true,
                    },
                ),
            ]);
        }

        solutions.retain(|solution| is_partial_solution_valid(board, solution));
        solutions
    }
}

impl ClueHandler for NotAdjacentHandler {
    fn handle(&self, board: &GameBoard, column: usize) -> Vec<Vec<(usize, TileAssertion)>> {
        let max_column = board.solution.n_variants - 1;

        // can the positive tile go here and the negative assertion work both ways?
        if let Some(candidate) =
            board.get_candidate(self.positive_tile.row, column, self.positive_tile.variant)
        {
            if candidate.state == CandidateState::Eliminated {
                // positive tile can't go here
                return Vec::new();
            }
        }

        let mut solutions = Vec::new();

        if column + 1 <= max_column {
            solutions.push(vec![
                (
                    column,
                    TileAssertion {
                        tile: self.positive_tile.clone(),
                        assertion: true,
                    },
                ),
                (
                    column + 1,
                    TileAssertion {
                        tile: self.negative_tile.clone(),
                        assertion: false,
                    },
                ),
            ]);
        }

        if column > 0 {
            solutions.push(vec![
                (
                    column - 1,
                    TileAssertion {
                        tile: self.negative_tile.clone(),
                        assertion: false,
                    },
                ),
                (
                    column,
                    TileAssertion {
                        tile: self.positive_tile.clone(),
                        assertion: true,
                    },
                ),
            ]);
        }

        let all_solutions_are_valid = solutions
            .iter()
            .all(|solution| is_partial_solution_valid(board, solution));

        trace!(
            target: "solver",
            "Found potential solutions: {:?}; all are valid? {}",
            solutions,
            all_solutions_are_valid
        );
        if all_solutions_are_valid {
            return solutions;
        } else {
            return Vec::new();
        }
    }
}

impl ClueHandler for AdjacentHandler {
    fn handle(&self, board: &GameBoard, column: usize) -> Vec<Vec<(usize, TileAssertion)>> {
        let max_column = board.solution.n_variants - 1;
        let clue_size = self.clue.assertions.len();
        if (column + clue_size - 1) > max_column {
            trace!(
                target: "solver",
                "Clue out of bounds for column {}",
                column
            );
            // clue out of bounds
            return Vec::new();
        }
        let mut solutions = Vec::new();

        // forward solution
        // do we go out of bounds?
        // append the clue here, we'll filter it later
        let mut forward_solution = Vec::new();
        let mut reverse_solution = Vec::new();
        for i in 0..clue_size {
            let forward_assertion = &self.clue.assertions[i];
            let reverse_assertion = &self.clue.assertions[clue_size - 1 - i];
            forward_solution.push((column + i, forward_assertion.clone()));
            reverse_solution.push((column + i, reverse_assertion.clone()));
        }
        solutions.push(forward_solution);
        solutions.push(reverse_solution);

        trace!(
            target: "solver",
            "Potential solutions for column {}: {:?}",
            column,
            solutions
        );

        // check solutions
        solutions.retain(|solution| is_partial_solution_valid(board, solution));
        solutions
    }
}

#[derive(Clone, Debug)]
struct AllInColumnHandler {
    assertions: Vec<TileAssertion>,
}

impl AllInColumnHandler {
    fn new(clue: &Clue) -> Self {
        Self {
            assertions: clue.assertions.clone(),
        }
    }
}

impl ClueHandler for AllInColumnHandler {
    fn handle(&self, board: &GameBoard, column: usize) -> Vec<Vec<(usize, TileAssertion)>> {
        let solution = self
            .assertions
            .iter()
            .map(|ta| (column, ta.clone()))
            .collect::<Vec<(usize, TileAssertion)>>();

        if is_partial_solution_valid(board, &solution) {
            vec![solution]
        } else {
            Vec::new()
        }
    }
}

fn create_handler(clue: &Clue) -> Box<dyn ClueHandler> {
    match &clue.clue_type {
        ClueType::Horizontal(h_type) => match h_type {
            HorizontalClueType::TwoAdjacent => Box::new(AdjacentHandler::new(clue)),
            HorizontalClueType::ThreeAdjacent => Box::new(AdjacentHandler::new(clue)),
            HorizontalClueType::TwoApartNotMiddle => Box::new(AdjacentHandler::new(clue)),
            HorizontalClueType::NotAdjacent => Box::new(NotAdjacentHandler::new(clue)),
            HorizontalClueType::LeftOf => Box::new(LeftOfHandler::new(clue)),
        },
        ClueType::Vertical(v_type) => match v_type {
            VerticalClueType::OneMatchesEither => panic!("Cannot handle clue type {:?}", v_type),
            _ => Box::new(AllInColumnHandler::new(clue)),
        },
    }
}

fn deduce_clue_with_handler(board: &GameBoard, clue: &Clue) -> Vec<Deduction> {
    let handler = create_handler(clue);
    let board_width = board.solution.n_variants;
    let mut deductions = HashSet::new();
    let mut possible_solutions: Vec<Vec<(usize, TileAssertion)>> = Vec::new();

    trace!(
        target: "solver",
        "Deducing clue with handler: {:?}",
        handler
    );

    for column in 0..board_width {
        let solutions = handler.handle(board, column);
        trace!(
            target: "solver",
            "Found {:?} solutions for column {}",
            solutions,
            column
        );
        possible_solutions.extend(solutions);
    }

    trace!(
        target: "solver",
        "Total possible solutions: {}",
        possible_solutions.len()
    );
    if possible_solutions.len() == 1 {
        // this is the solution
        let solution = &possible_solutions[0];
        for (column, tile_assertion) in solution {
            if tile_assertion.assertion {
                deductions.insert(Deduction {
                    tile: tile_assertion.tile.clone(),
                    column: *column,
                    is_positive: true,
                });
            } else {
                deductions.insert(Deduction {
                    tile: tile_assertion.tile.clone(),
                    column: *column,
                    is_positive: false,
                });
            }
        }
    } else {
        // For each coordinate in possible solutions,
        let mut solutions_by_coordinates: HashMap<Coordinates, CellSolutionAssertion> =
            HashMap::new();
        for possible_solution in &possible_solutions {
            for (column, tile_assertion) in possible_solution {
                let coord = Coordinates {
                    row: tile_assertion.tile.row,
                    column: *column,
                };

                let positive_variant_set = if tile_assertion.assertion {
                    HashSet::from([tile_assertion.tile.variant])
                } else {
                    HashSet::new()
                };

                let negative_variant_set = if tile_assertion.assertion {
                    HashSet::new()
                } else {
                    HashSet::from([tile_assertion.tile.variant])
                };

                let entry =
                    solutions_by_coordinates
                        .entry(coord)
                        .or_insert(CellSolutionAssertion {
                            positive_variants: positive_variant_set.clone(),
                            negative_variants: negative_variant_set.clone(),
                            positive_count: 0,
                            negative_count: 0,
                        });
                // union
                entry.positive_variants.extend(&positive_variant_set);
                // intersect
                entry
                    .negative_variants
                    .retain(|v| negative_variant_set.contains(v));

                if tile_assertion.assertion {
                    entry.positive_count += 1;
                } else {
                    entry.negative_count += 1;
                }
            }
        }

        // solution is considered "anchored" if the number of cells with positive assertions == the number of positive assertions
        let positive_assertion_count = clue.assertions.iter().filter(|ta| ta.assertion).count();
        let cells_with_positive_assertions = solutions_by_coordinates
            .iter()
            .filter(|(_, cell_solution_assertion)| cell_solution_assertion.positive_count > 0)
            .count();

        let anchored = cells_with_positive_assertions == positive_assertion_count;

        trace!(
            target: "solver",
            "Solutions by coordinates: {:?}; anchored: {}",
            solutions_by_coordinates,
            anchored
        );

        for row in 0..board.solution.n_rows {
            let mut row_positive_assertion_varants = HashSet::new();
            for assertion in clue
                .assertions
                .iter()
                .filter(|ta| ta.assertion && ta.tile.row == row)
            {
                row_positive_assertion_varants.insert(assertion.tile.variant);
            }
            if row_positive_assertion_varants.len() == 0 && !anchored {
                // clues do not affect this row
                trace!(
                    target: "solver",
                    "Row {} has no positive assertions, and not anchored",
                    row
                );
                continue;
            }

            trace!(
                target: "solver",
                "Processing row {} with positive assertions: {:?}",
                row,
                row_positive_assertion_varants
            );

            for col in 0..board_width {
                if let Some(cell_solution_assertion) =
                    solutions_by_coordinates.get(&Coordinates { row, column: col })
                {
                    if cell_solution_assertion.positive_count == possible_solutions.len() {
                        trace!(
                            target: "solver",
                            "Cell ({}, {}) appears in all solutions; assertions {:?}",
                            row,
                            col,
                            cell_solution_assertion
                        );
                        // this should always be true
                        if cell_solution_assertion.positive_variants.len() > 0 {
                            // all possible solutions point to this square, all other variants cannot be here
                            for variant in board.solution.variants.iter() {
                                if !cell_solution_assertion.positive_variants.contains(variant) {
                                    deductions.insert(Deduction {
                                        tile: Tile::new(row, *variant),
                                        column: col,
                                        is_positive: false,
                                    });
                                }
                            }
                        }

                        continue;
                    } else if anchored {
                        trace!(
                            target: "solver",
                            "Processing negative assertions since it is anchored",
                        );
                        // all negative solutions point to this square, negative variants can't be here.
                        for variant in cell_solution_assertion.negative_variants.iter() {
                            deductions.insert(Deduction {
                                tile: Tile::new(row, *variant),
                                column: col,
                                is_positive: false,
                            });
                        }
                    } else {
                        trace!(
                            target: "solver",
                            "Cell ({}, {}) appears in some but not all solutions",
                            row,
                            col
                        );

                        for variant in board.solution.variants.iter() {
                            if !cell_solution_assertion.positive_variants.contains(variant)
                                && row_positive_assertion_varants.contains(variant)
                            {
                                deductions.insert(Deduction {
                                    tile: Tile::new(row, *variant),
                                    column: col,
                                    is_positive: false,
                                });
                            }
                        }
                        continue;
                    }
                } else {
                    trace!(
                        target: "solver",
                        "Cell ({}, {}) appears in no solutions",
                        row,
                        col
                    );
                    // deduct all positive assertion from this row, they can't possibly be here
                    row_positive_assertion_varants.iter().for_each(|variant| {
                        deductions.insert(Deduction {
                            tile: Tile::new(row, *variant),
                            column: col,
                            is_positive: false,
                        });
                    });
                }
            }
        }
    }

    let filtered_deductions: Vec<Deduction> = deductions
        .into_iter()
        .filter(|deduction| !is_known_deduction(board, deduction))
        .collect();
    trace!(
        target: "solver",
        "Found {} deductions",
        filtered_deductions.len()
    );
    filtered_deductions
}

fn deduce_one_matches_either(board: &GameBoard, tiles: &[Tile]) -> Vec<Deduction> {
    let board_width = board.solution.n_variants;
    let mut possible_columns = vec![
        (tiles[0], HashSet::new()),
        (tiles[1], HashSet::new()),
        (tiles[2], HashSet::new()),
    ];

    let t0 = tiles[0].clone();
    let t0_selected_col = (0..board_width).find(|col| board.is_selected_in_column(&t0, *col));

    let t1_eliminated_from_t0_selected_col = t0_selected_col
        .map(|col| board.has_negative_deduction(&tiles[1], col))
        .unwrap_or(false);
    let t2_eliminated_from_t0_selected_col = t0_selected_col
        .map(|col| board.has_negative_deduction(&tiles[2], col))
        .unwrap_or(false);

    if t1_eliminated_from_t0_selected_col {
        return deduce_clue(board, &Clue::two_in_column(tiles[0], tiles[2]));
    }

    if t2_eliminated_from_t0_selected_col {
        return deduce_clue(board, &Clue::two_in_column(tiles[0], tiles[1]));
    }

    // fallback behavior when junction not known
    for col in 0..board_width {
        // only add t0 if either t1 or t2 are available in this column
        if !board.has_negative_deduction(&tiles[1], col)
            || !board.has_negative_deduction(&tiles[2], col)
        {
            possible_columns[0].1.insert(col);
        }

        // add t1 if t2 is not selected in this column
        if !board.is_selected_in_column(&tiles[2], col) {
            possible_columns[1].1.insert(col);
        }

        // add t2 if t1 is not selected in this column
        if !board.is_selected_in_column(&tiles[1], col) {
            possible_columns[2].1.insert(col);
        }
    }

    synthesize_deductions(board, &possible_columns)
}

fn group_variants_by_columns<K>(
    possible_variant_columns: &HashMap<K, HashSet<usize>>,
) -> Vec<(&HashSet<usize>, Vec<K>)>
where
    K: Copy + Eq,
{
    let mut grouped_by_columns: Vec<(&HashSet<usize>, Vec<K>)> = Vec::new();
    for (variant, columns) in possible_variant_columns {
        // Try to find existing group with matching columns
        let mut found = false;
        for (existing_columns, variants) in &mut grouped_by_columns {
            if **existing_columns == *columns {
                variants.push(*variant);
                found = true;
                break;
            }
        }

        // If no matching group found, create new one
        if !found {
            grouped_by_columns.push((columns, vec![*variant]));
        }
    }
    grouped_by_columns
}

pub fn deduce_hidden_pairs(board: &GameBoard) -> Vec<Deduction> {
    let mut deductions = Vec::new();

    for row in 0..board.solution.n_rows {
        // Build the map of variant -> possible columns
        let mut possible_variant_columns = HashMap::new();
        for col in 0..board.solution.n_variants {
            for variant in board.solution.variants.iter() {
                if let Some(candidate) = board.get_candidate(row, col, *variant) {
                    if candidate.state == CandidateState::Available {
                        possible_variant_columns
                            .entry(candidate.tile.variant)
                            .or_insert(HashSet::new())
                            .insert(col);
                    }
                }
            }
        }

        // Group variants by their possible columns
        let grouped_by_columns = group_variants_by_columns(&possible_variant_columns);

        // Filter to only consider sets of appropriate size and check for hidden sets
        for (columns, variants) in grouped_by_columns {
            if columns.len() <= board.solution.n_variants / 2 && variants.len() == columns.len() {
                // Found a hidden set! Add negative deductions for other variants in these columns
                for col in columns {
                    for variant in board.solution.variants.iter() {
                        if !variants.contains(variant) {
                            deductions.push(Deduction {
                                tile: Tile::new(row, *variant),
                                column: *col,
                                is_positive: false,
                            });
                        }
                    }
                }
            }
        }
    }

    deductions
        .into_iter()
        .filter(|deduction| !is_known_deduction(board, deduction))
        .collect()
}

pub fn deduce_clue(board: &GameBoard, clue: &Clue) -> Vec<Deduction> {
    let tiles = clue.assertions.iter().map(|a| a.tile).collect::<Vec<_>>();
    match &clue.clue_type {
        ClueType::Horizontal(HorizontalClueType::ThreeAdjacent) => {
            deduce_clue_with_handler(board, &clue)
        }

        ClueType::Horizontal(HorizontalClueType::TwoAdjacent) => {
            return deduce_clue_with_handler(board, &clue);
        }

        ClueType::Horizontal(HorizontalClueType::TwoApartNotMiddle) => {
            deduce_clue_with_handler(board, &clue)
        }

        ClueType::Horizontal(HorizontalClueType::LeftOf) => deduce_clue_with_handler(board, &clue),

        ClueType::Horizontal(HorizontalClueType::NotAdjacent) => {
            deduce_clue_with_handler(board, &clue)
        }

        ClueType::Vertical(VerticalClueType::ThreeInColumn)
        | ClueType::Vertical(VerticalClueType::TwoInColumn) => {
            deduce_clue_with_handler(board, &clue)
        }

        ClueType::Vertical(VerticalClueType::OneMatchesEither) => {
            deduce_one_matches_either(board, &tiles)
        }

        ClueType::Vertical(VerticalClueType::NotInSameColumn) => {
            deduce_clue_with_handler(board, &clue)
        }

        ClueType::Vertical(VerticalClueType::TwoInColumnWithout) => {
            deduce_clue_with_handler(board, &clue)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvaluationStepResult {
    Nothing,
    HiddenPairsFound,
    DeductionsFound(Clue),
}

pub fn perform_evaluation_step(board: &mut GameBoard, clues: &Vec<Clue>) -> EvaluationStepResult {
    // nothing to do
    if board.is_complete() {
        return EvaluationStepResult::Nothing;
    }

    let deductions = deduce_hidden_pairs(board);
    if deductions.len() > 0 {
        board.apply_deductions(&deductions);
        return EvaluationStepResult::HiddenPairsFound;
    }

    // reapply existing clues
    for clue in clues.iter() {
        let deductions = deduce_clue(board, clue);
        if deductions.len() > 0 {
            board.apply_deductions(&deductions);
            return EvaluationStepResult::DeductionsFound(clue.clone());
        }
    }
    trace!(
        target: "solver",
        "No deductions found. board: {:?}",
        board
    );
    for clue in clues.iter() {
        trace!(target: "solver", "Clue: {:?}", clue);
    }
    EvaluationStepResult::Nothing
}

#[cfg(test)]
mod tests {
    use test_context::test_context;

    use super::*;
    use crate::{
        game::tests::UsingLogger,
        model::{Clue, Difficulty, GameBoard, Solution, Tile, MAX_GRID_SIZE},
    };
    use std::rc::Rc;

    fn create_test_solution(n_rows: usize) -> Rc<Solution> {
        let mut grid = [[Tile::new(0, '0'); MAX_GRID_SIZE]; MAX_GRID_SIZE];
        // Fill first 4x4 of grid with test data
        for row in 0..3 {
            for col in 0..4 {
                grid[row][col] = Tile::new(row, (b'a' + col as u8) as char);
            }
        }

        Rc::new(Solution {
            variants: vec!['a', 'b', 'c', 'd'],
            grid,
            n_rows,
            n_variants: 4,
            variants_range: 'a'..='d',
            difficulty: Difficulty::Easy,
            seed: 0,
        })
    }

    #[test]
    fn test_deduce_three_adjacent_empty_board() {
        let input = "\
1|abcd|abcd|abcd|abcd|
-----------------
2|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));

        // Create a clue for 3 adjacent tiles in row 1
        let clue = Clue::three_adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'), Tile::new(0, 'c'));

        let deductions = deduce_clue(&board, &clue);
        trace!(
            target: "solver",
            "Deductions: {:?}",
            deductions
        );

        // We expect to only deduce the edges as not possible.
        assert_eq!(deductions.len(), 4);
        assert!(deductions.contains(&Deduction::parse("0b not col 0").unwrap()));
        assert!(deductions.contains(&Deduction::parse("0b not col 3").unwrap()));
        assert!(deductions.contains(&Deduction::parse("0d not col 1").unwrap()));
        assert!(deductions.contains(&Deduction::parse("0d not col 2").unwrap()));
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_deduce_three_adjacent_partially_solved_board(_: &mut UsingLogger) {
        let input = "\
0|abcd|<B> |abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));

        let clue = Clue::three_adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'), Tile::new(1, 'a'));

        trace!(
            target: "solver",
            "Board: {:?}",
            board
        );
        trace!(target: "solver", "Clue: {:?}", clue);

        let deductions = deduce_clue(&board, &clue);
        trace!(
            target: "solver",
            "Deductions: {:?}",
            deductions
        );
        assert_eq!(deductions.len(), 3);
        assert!(deductions.contains(&Deduction::parse("0a not col 3").unwrap()));
        assert!(deductions.contains(&Deduction::parse("1a not col 1").unwrap()));
        assert!(deductions.contains(&Deduction::parse("1a not col 3").unwrap()));
    }

    #[test]
    fn test_deduce_three_adjacent_solvable_board() {
        let input = "\
0|<A> |abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));

        let clue = Clue::three_adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'), Tile::new(1, 'a'));

        println!("Board: {:?}", board);
        println!("Clue: {:?}", clue);

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("0b is col 1").unwrap()));
        assert!(deductions.contains(&Deduction::parse("1a is col 2").unwrap()));
    }

    #[test]
    fn test_deduce_two_adjacent_partially_solved_board() {
        let input = "\
0|abcd|<B> |abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));

        let clue = Clue::adjacent(Tile::new(0, 'a'), Tile::new(1, 'a'));

        println!("Board: {:?}", board);
        println!("Clue: {:?}", clue);

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("1a not col 0").unwrap()));
    }

    #[test]
    fn test_deduce_two_adjacent_solvable_board() {
        let input = "\
0|<A> |abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));

        let clue = Clue::adjacent(Tile::new(0, 'a'), Tile::new(1, 'a'));

        println!("Board: {:?}", board);
        println!("Clue: {:?}", clue);

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("1a is col 1").unwrap()));
    }

    #[test]
    fn test_deduce_two_apart_not_middle_unsolved() {
        let input = "\
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));

        let clue =
            Clue::two_apart_not_middle(Tile::new(0, 'a'), Tile::new(0, 'b'), Tile::new(1, 'a'));

        println!("Board: {:?}", board);
        println!("Clue: {:?}", clue);

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 0);
    }

    #[test]
    fn test_deduce_two_apart_not_middle_solvable() {
        let input = "\
0|abcd| <B>|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));

        let clue =
            Clue::two_apart_not_middle(Tile::new(0, 'a'), Tile::new(0, 'b'), Tile::new(1, 'a'));

        println!("Board: {:?}", board);
        println!("Clue: {:?}", clue);

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("0a is col 3").unwrap()));
        assert!(deductions.contains(&Deduction::parse("1a is col 1").unwrap()));
    }

    #[test]
    fn test_deduce_two_apart_not_middle_second_tile_partially_solvable() {
        let input = "\
0|<D> |abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));

        let clue =
            Clue::two_apart_not_middle(Tile::new(0, 'a'), Tile::new(0, 'b'), Tile::new(1, 'a'));

        println!("Board: {:?}", board);
        println!("Clue: {:?}", clue);

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("1a not col 2").unwrap()));
    }

    #[test]
    fn test_deduce_left_of_empty_board() {
        let input = "\
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));

        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(1, 'b'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("0a not col 3").unwrap()));
        assert!(deductions.contains(&Deduction::parse("1b not col 0").unwrap()));
    }

    #[test]
    fn test_deduce_left_of_with_selection() {
        let input = "\
0|abcd|<A> |abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));

        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(1, 'a'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1a not col 0").unwrap()));
        assert!(deductions.contains(&Deduction::parse("1a not col 1").unwrap()));
    }

    #[test]
    fn test_deduce_left_of_partially_solved() {
        let input = "\
0| bcd|abcd|abcd| bcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));

        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(1, 'a'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1a not col 0").unwrap()));
        assert!(deductions.contains(&Deduction::parse("1a not col 1").unwrap()));
    }

    #[test]
    fn test_deduce_not_adjacent_empty_board() {
        let input = "\
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));
        println!("Board: {:?}", board);

        let clue = Clue::not_adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 0); // No deductions possible on empty board
    }

    #[test]
    fn test_deduce_not_adjacent_partially_solvable_board() {
        let input = "\
0|abcd|<A> |abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));
        println!("Board: {:?}", board);

        let clue = Clue::not_adjacent(Tile::new(0, 'a'), Tile::new(1, 'a'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1a not col 0").unwrap()));
        assert!(deductions.contains(&Deduction::parse("1a not col 2").unwrap()));

        let clue = Clue::not_adjacent(Tile::new(1, 'a'), Tile::new(0, 'a'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1a not col 0").unwrap()));
        assert!(deductions.contains(&Deduction::parse("1a not col 2").unwrap()));
    }

    #[test]
    fn test_deduce_not_adjacent_solvable_board() {
        let input = "\
0|abcd|<B> |abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));
        println!("Board: {:?}", board);

        let clue = Clue::not_adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("0a is col 3").unwrap()));
    }

    #[test]
    fn test_deduce_all_in_column_empty_board() {
        let input = "\
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------
2|abcd|abcd|abcd|abcd|";

        let board = GameBoard::parse(input, create_test_solution(3));

        let clue = Clue::three_in_column(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 0); // No deductions possible on empty board
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_deduce_all_in_column_solvable(_: &mut UsingLogger) {
        let input = "\
0|abcd|abcd|<A> |abcd|
----------------------
1|<A> |abcd|abcd|abcd|
----------------------
2|abcd|abcd|abcd|abcd|
----------------------
";

        let board = GameBoard::parse(input, create_test_solution(3));
        println!("Board: {:?}", board);

        let clue = Clue::parse_vertical("|+0a,+1b,+2c|");

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1b is col 2").unwrap()));
        assert!(deductions.contains(&Deduction::parse("2c is col 2").unwrap()));
    }

    #[test]
    fn test_deduce_all_in_column_with_multiple_eliminations() {
        let input = "\
0|abcd|abcd|<B> |abcd|
----------------------
1|<B> |abcd|abcd|abcd|
----------------------
2|abcd|abcd|abcd|abcd|
----------------------
";

        let board = GameBoard::parse(input, create_test_solution(3));
        println!("Board: {:?}", board);

        let clue = Clue::three_in_column(Tile::new(0, 'a'), Tile::new(1, 'a'), Tile::new(2, 'a'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 4);
        assert!(deductions.contains(&Deduction::parse("0a not col 0").unwrap()));
        assert!(deductions.contains(&Deduction::parse("1a not col 2").unwrap()));
        assert!(deductions.contains(&Deduction::parse("2a not col 0").unwrap()));
        assert!(deductions.contains(&Deduction::parse("2a not col 2").unwrap()));
    }

    #[test]
    fn test_deduce_two_in_column_without_empty_board() {
        let input = "\
0|abcd|abcd|abcd|abcd|
----------------------
1|abcd|abcd|abcd|abcd|
----------------------
2|abcd|abcd|abcd|abcd|
----------------------
";

        let board = GameBoard::parse(input, create_test_solution(3));

        let clue =
            Clue::two_in_column_without(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        assert_eq!(deductions.len(), 0); // No deductions possible on empty board
    }

    #[test]
    fn test_deduce_two_in_column_without_middle_selected() {
        let input = "\
0|abcd|abcd|abcd|abcd|
----------------------
1|abcd|abcd|<B> |abcd|
----------------------
2|abcd|abcd|abcd|abcd|
----------------------
";

        let board = GameBoard::parse(input, create_test_solution(3));
        println!("Board: {:?}", board);

        let clue =
            Clue::two_in_column_without(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("0a not col 2").unwrap()));
        assert!(deductions.contains(&Deduction::parse("2c not col 2").unwrap()));
    }

    #[test]
    fn test_deduce_two_in_column_solvable() {
        let input = "\
0|abcd|abcd|<A> |abcd|
----------------------
1|<B> |abcd|abcd|abcd|
----------------------
2|abcd|abcd|abcd|abcd|
----------------------
";

        let board = GameBoard::parse(input, create_test_solution(3));

        let clue =
            Clue::two_in_column_without(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("2c is col 2").unwrap()));
    }

    #[test]
    fn test_deduce_one_matches_either_empty_board() {
        let input = "\
0|abcd|abcd|abcd|abcd|
----------------------
1|abcd|abcd|abcd|abcd|
----------------------
2|abcd|abcd|abcd|abcd|
----------------------
";
        let board = GameBoard::parse(input, create_test_solution(3));

        let clue =
            Clue::one_matches_either(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        assert_eq!(deductions.len(), 0); // No deductions possible on empty board
    }

    #[test]
    fn test_deduce_one_matches_either_first_selected_second_eliminated() {
        let input = "\
0|abcd|<A> |abcd|abcd|
----------------------
1|abcd|a cd|abcd|abcd|
----------------------
2|abcd|abcd|abcd|abcd|
----------------------
";
        let board = GameBoard::parse(input, create_test_solution(3));

        let clue =
            Clue::one_matches_either(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("2c is col 1").unwrap()));
    }

    #[test]
    fn test_deduce_one_matches_either_first_and_second_selected() {
        let input = "\
0|abcd|<A> |abcd|abcd|
----------------------
1|abcd|<B> |abcd|abcd|
----------------------
2|abcd|abcd|abcd|abcd|
----------------------
";
        let board = GameBoard::parse(input, create_test_solution(3));

        let clue =
            Clue::one_matches_either(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("2c not col 1").unwrap()));
    }

    #[test]
    fn test_deduce_one_matches_either_second_and_third_known() {
        let input = "\
0|abcd|abcd|abcd|abcd|
----------------------
1|abcd|<B> |abcd|abcd|
----------------------
2|abcd|abcd|abcd|<C> |
----------------------
";
        let board = GameBoard::parse(input, create_test_solution(3));

        let clue =
            Clue::one_matches_either(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("0a not col 0").unwrap()));
        assert!(deductions.contains(&Deduction::parse("0a not col 2").unwrap()));
    }

    #[test]
    fn test_deduce_hidden_pairs() {
        let input = "\
0|ab  |ab  |abcd|abcd|
----------------------
1|abcd|abcd|abcd|abcd|
----------------------
";
        let board = GameBoard::parse(input, create_test_solution(2));
        println!("Board: {:?}", board);

        let deductions = deduce_hidden_pairs(&board);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 4);
        assert!(deductions.contains(&Deduction::parse("0a not col 3").unwrap()));
        assert!(deductions.contains(&Deduction::parse("0b not col 3").unwrap()));
        assert!(deductions.contains(&Deduction::parse("0a not col 2").unwrap()));
        assert!(deductions.contains(&Deduction::parse("0b not col 2").unwrap()));
    }

    #[test]
    fn test_left_of_handler_empty_board() {
        let input = "\
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));
        println!("Board: {:?}", board);

        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(1, 'b'));

        let deductions = deduce_clue(&board, &clue);

        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("0a not col 3").unwrap()));
        assert!(deductions.contains(&Deduction::parse("1b not col 0").unwrap()));
    }

    #[test]
    fn test_left_of_handler_partially_solved() {
        let input = "\
0| bcd|abcd|abcd| bcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));
        println!("Board: {:?}", board);

        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(1, 'b'));
        let deductions = deduce_clue(&board, &clue);

        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1b not col 0").unwrap()));
        assert!(deductions.contains(&Deduction::parse("1b not col 1").unwrap()));
    }

    #[test]
    fn test_left_of_handler_solvable() {
        let input = "\
0|abcd|abcd|<A> |abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));
        println!("Board: {:?}", board);

        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(1, 'b'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("1b is col 3").unwrap()));
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_eliminate_invalid_solutions(_: &mut UsingLogger) {
        let input = "\
0|ab  |abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2));
        println!("Board: {:?}", board);

        let clue = Clue::adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 6);
        assert!(deductions.contains(&Deduction::parse("0c not col 1").unwrap()));
        assert!(deductions.contains(&Deduction::parse("0d not col 1").unwrap()));
        assert!(deductions.contains(&Deduction::parse("0a not col 2").unwrap()));
        assert!(deductions.contains(&Deduction::parse("0b not col 2").unwrap()));
        assert!(deductions.contains(&Deduction::parse("0a not col 3").unwrap()));
        assert!(deductions.contains(&Deduction::parse("0b not col 3").unwrap()));
    }
}
