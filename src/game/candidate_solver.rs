use std::collections::{HashMap, HashSet};

use crate::{
    game::clue_constraint::create_clue_constraint,
    model::{
        Clue, ClueType, Deduction, DeductionKind, GameBoard, HorizontalClueType, Tile,
        TileAssertion, VerticalClueType,
    },
};
use log::trace;

use super::hidden_pair_finder::{find_hidden_pairs_in_row, find_naked_pairs_in_row};

fn is_known_deduction(board: &GameBoard, deduction: &Deduction) -> bool {
    let result = if deduction.tile_assertion.assertion {
        board.is_selected_in_column(&deduction.tile_assertion.tile, deduction.column)
    } else {
        board.has_negative_deduction(&deduction.tile_assertion.tile, deduction.column)
    };
    result
}

pub fn group_assertions_by_coordinates(
    possible_solutions: &Vec<Vec<(usize, TileAssertion)>>,
) -> HashMap<Coordinates, CellSolutionAssertion> {
    // For each coordinate in possible solutions,
    let mut solutions_by_coordinates: HashMap<Coordinates, CellSolutionAssertion> = HashMap::new();
    for possible_solution in possible_solutions.iter() {
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

            let entry = solutions_by_coordinates
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

    solutions_by_coordinates
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
            deductions.insert(Deduction::new(
                *column,
                TileAssertion {
                    tile: tile.clone(),
                    assertion: true,
                },
            ));
        } else {
            // Make negative deductions for impossible columns
            for col in 0..board_width {
                if !possible.contains(&col) {
                    deductions.insert(Deduction::new(
                        col,
                        TileAssertion {
                            tile: tile.clone(),
                            assertion: false,
                        },
                    ));
                }
            }
        }
    }

    deductions
        .into_iter()
        .filter(|deducation| !is_known_deduction(board, deducation))
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Coordinates {
    pub row: usize,
    pub column: usize,
}

impl std::fmt::Debug for Coordinates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{},{}}}", self.row, self.column)
    }
}

impl Coordinates {
    pub fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CellSolutionAssertion {
    pub positive_variants: HashSet<char>,
    pub negative_variants: HashSet<char>,
    pub positive_count: usize,
    pub negative_count: usize,
}

fn deduce_clue_with_candidate_finder(board: &GameBoard, clue: &Clue) -> Vec<Deduction> {
    let solution_candidate_finder = create_clue_constraint(clue);
    let board_width = board.solution.n_variants;
    let mut deductions = HashSet::new();
    let mut possible_solutions: Vec<Vec<(usize, TileAssertion)>> = Vec::new();

    trace!(
        target: "solver",
        "Deducing clue with handler: {:?}",
        solution_candidate_finder
    );

    for column in 0..board_width {
        let solutions = solution_candidate_finder.potential_solutions(board, column);
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
            deductions.insert(Deduction::new_with_kind(
                *column,
                tile_assertion.clone(),
                DeductionKind::LastRemaining,
            ));
        }
    } else {
        // For each coordinate in possible solutions,
        let solutions_by_coordinates = group_assertions_by_coordinates(&possible_solutions);

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
                                    deductions.insert(Deduction::new_with_kind(
                                        col,
                                        TileAssertion {
                                            tile: Tile::new(row, *variant),
                                            assertion: false,
                                        },
                                        DeductionKind::Converging,
                                    ));
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
                            deductions.insert(Deduction::new_with_kind(
                                col,
                                TileAssertion {
                                    tile: Tile::new(row, *variant),
                                    assertion: false,
                                },
                                DeductionKind::Converging,
                            ));
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
                                deductions.insert(Deduction::new_with_kind(
                                    col,
                                    TileAssertion {
                                        tile: Tile::new(row, *variant),
                                        assertion: false,
                                    },
                                    DeductionKind::Constraint,
                                ));
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
                        deductions.insert(Deduction::new_with_kind(
                            col,
                            TileAssertion {
                                tile: Tile::new(row, *variant),
                                assertion: false,
                            },
                            DeductionKind::Constraint,
                        ));
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

pub fn deduce_hidden_sets_in_row(board: &GameBoard, row: usize) -> Vec<Deduction> {
    let mut deductions = Vec::new();

    let mut hidden_sets = Vec::new();
    hidden_sets.extend(find_hidden_pairs_in_row(row, board));
    if hidden_sets.len() == 0 {
        hidden_sets.extend(find_naked_pairs_in_row(row, board));
    }
    if hidden_sets.len() == 0 {
        return deductions;
    }

    trace!(
        target: "solver",
        "Found {} hidden sets: {:?}",
        hidden_sets.len(),
        hidden_sets
    );

    // find the smallest one
    let smallest_hidden_set = hidden_sets.iter().min_by_key(|set| set.variants.len());
    if let Some(smallest_hidden_set) = smallest_hidden_set {
        trace!(
            target: "solver",
            "Smallest hidden set: {:?}",
            smallest_hidden_set
        );
        // add negative deductions for the variants that are not in the smallest hidden set
        let hidden_set_inverse = board
            .solution
            .variants
            .clone()
            .into_iter()
            .filter(|v| !smallest_hidden_set.variants.contains(v))
            .collect::<Vec<_>>();

        for column in 0..board.solution.n_variants {
            let col_in_set = smallest_hidden_set.columns.contains(&column);
            if col_in_set {
                // only the hidden set variants can go here
                // not in hidden set, but on board? get rid of it.
                for not_in_set_variant in hidden_set_inverse.iter() {
                    if board.is_candidate_available(row, column, *not_in_set_variant) {
                        deductions.push(Deduction::new_with_kind(
                            column,
                            TileAssertion {
                                tile: Tile::new(row, *not_in_set_variant),
                                assertion: false,
                            },
                            DeductionKind::HiddenSet,
                        ));
                    }
                }
            } else {
                // hidden set variants cannot go here
                for hidden_set_variant in smallest_hidden_set.variants.iter() {
                    // eliminate hidden variants from columns not part of set
                    if board.is_candidate_available(row, column, *hidden_set_variant) {
                        // remove it! you don't belong here
                        deductions.push(Deduction::new(
                            column,
                            TileAssertion {
                                tile: Tile::new(row, *hidden_set_variant),
                                assertion: false,
                            },
                        ));
                    }
                }
            }
        }
    }
    deductions
}

pub fn deduce_hidden_sets(board: &GameBoard) -> Vec<Deduction> {
    (0..board.solution.n_rows)
        .flat_map(|row| deduce_hidden_sets_in_row(board, row))
        .collect()
}

pub fn deduce_clue(board: &GameBoard, clue: &Clue) -> Vec<Deduction> {
    let tiles = clue.assertions.iter().map(|a| a.tile).collect::<Vec<_>>();
    match &clue.clue_type {
        ClueType::Horizontal(HorizontalClueType::ThreeAdjacent) => {
            deduce_clue_with_candidate_finder(board, &clue)
        }

        ClueType::Horizontal(HorizontalClueType::TwoAdjacent) => {
            return deduce_clue_with_candidate_finder(board, &clue);
        }

        ClueType::Horizontal(HorizontalClueType::TwoApartNotMiddle) => {
            deduce_clue_with_candidate_finder(board, &clue)
        }

        ClueType::Horizontal(HorizontalClueType::LeftOf) => {
            deduce_clue_with_candidate_finder(board, &clue)
        }

        ClueType::Horizontal(HorizontalClueType::NotAdjacent) => {
            deduce_clue_with_candidate_finder(board, &clue)
        }

        ClueType::Vertical(VerticalClueType::ThreeInColumn)
        | ClueType::Vertical(VerticalClueType::TwoInColumn) => {
            deduce_clue_with_candidate_finder(board, &clue)
        }

        ClueType::Vertical(VerticalClueType::OneMatchesEither) => {
            deduce_one_matches_either(board, &tiles)
        }

        ClueType::Vertical(VerticalClueType::NotInSameColumn) => {
            deduce_clue_with_candidate_finder(board, &clue)
        }

        ClueType::Vertical(VerticalClueType::TwoInColumnWithout) => {
            deduce_clue_with_candidate_finder(board, &clue)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvaluationStepResult {
    Nothing,
    HiddenSetsFound,
    DeductionsFound(Clue),
}

/// note - does not mutate, does not auto-solve, caller must call auto-solve after applying evaluation
pub fn perform_evaluation_step(board: &mut GameBoard, clues: &Vec<Clue>) -> EvaluationStepResult {
    // nothing to do
    if board.is_complete() {
        return EvaluationStepResult::Nothing;
    }

    // apply clues
    for clue in clues.iter() {
        let deductions = deduce_clue(board, clue);
        if deductions.len() > 0 {
            board.apply_deductions(&deductions);
            return EvaluationStepResult::DeductionsFound(clue.clone());
        }
    }

    // apply hidden sets
    let deductions = deduce_hidden_sets(board);
    if deductions.len() > 0 {
        board.apply_deductions(&deductions);
        return EvaluationStepResult::HiddenSetsFound;
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
    use crate::game::tests::create_test_solution;
    use crate::{
        model::{Clue, GameBoard, Tile},
        tests::UsingLogger,
    };

    #[test]
    fn test_deduce_three_adjacent_empty_board() {
        let input = "\
1|abcd|abcd|abcd|abcd|
-----------------
2|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));

        // Create a clue for 3 adjacent tiles in row 1
        let clue = Clue::three_adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'), Tile::new(0, 'c'));

        let deductions = deduce_clue(&board, &clue);
        trace!(
            target: "solver",
            "Deductions: {:?}",
            deductions
        );
        println!("Deductions: {:?}", deductions);

        // We expect to only deduce the edges as not possible.
        assert_eq!(deductions.len(), 4);
        assert!(deductions.contains(&Deduction::parse("0b not col 0 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("0b not col 3 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("0d not col 1 (Converging)")));
        assert!(deductions.contains(&Deduction::parse("0d not col 2 (Converging)")));
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_deduce_three_adjacent_partially_solved_board(_: &mut UsingLogger) {
        let input = "\
0|abcd|<B> |abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));

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
        assert!(deductions.contains(&Deduction::parse("0a not col 3 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("1a not col 1 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("1a not col 3 (Constraint)")));
    }

    #[test]
    fn test_deduce_three_adjacent_solvable_board() {
        let input = "\
0|<A> |abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));

        let clue = Clue::three_adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'), Tile::new(1, 'a'));

        println!("Board: {:?}", board);
        println!("Clue: {:?}", clue);

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("0b is col 1 (LastRemaining)")));
        assert!(deductions.contains(&Deduction::parse("1a is col 2 (LastRemaining)")));
    }

    #[test]
    fn test_deduce_two_adjacent_partially_solved_board() {
        let input = "\
0|abcd|<B> |abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));

        let clue = Clue::adjacent(Tile::new(0, 'a'), Tile::new(1, 'a'));

        println!("Board: {:?}", board);
        println!("Clue: {:?}", clue);

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("1a not col 0 (Constraint)")));
    }

    #[test]
    fn test_deduce_two_adjacent_solvable_board() {
        let input = "\
0|<A> |abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));

        let clue = Clue::adjacent(Tile::new(0, 'a'), Tile::new(1, 'a'));

        println!("Board: {:?}", board);
        println!("Clue: {:?}", clue);

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("1a is col 1 (LastRemaining)")));
    }

    #[test]
    fn test_deduce_two_apart_not_middle_unsolved() {
        let input = "\
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));

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

        let board = GameBoard::parse(input, create_test_solution(2, 4));

        let clue =
            Clue::two_apart_not_middle(Tile::new(0, 'a'), Tile::new(0, 'b'), Tile::new(1, 'a'));

        println!("Board: {:?}", board);
        println!("Clue: {:?}", clue);

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("0a is col 3 (LastRemaining)")));
        assert!(deductions.contains(&Deduction::parse("1a is col 1 (LastRemaining)")));
    }

    #[test]
    fn test_deduce_two_apart_not_middle_second_tile_partially_solvable() {
        let input = "\
0|<D> |abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));

        let clue =
            Clue::two_apart_not_middle(Tile::new(0, 'a'), Tile::new(0, 'b'), Tile::new(1, 'a'));

        println!("Board: {:?}", board);
        println!("Clue: {:?}", clue);

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("1a not col 2 (Constraint)")));
    }

    #[test]
    fn test_deduce_left_of_empty_board() {
        let input = "\
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));

        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(1, 'b'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("0a not col 3 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("1b not col 0 (Constraint)")));
    }

    #[test]
    fn test_deduce_left_of_with_selection() {
        let input = "\
0|abcd|<A> |abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));

        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(1, 'a'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1a not col 0 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("1a not col 1 (Constraint)")));
    }

    #[test]
    fn test_deduce_left_of_partially_solved() {
        let input = "\
0| bcd|abcd|abcd| bcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));

        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(1, 'a'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1a not col 0 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("1a not col 1 (Constraint)")));
    }

    #[test]
    fn test_deduce_not_adjacent_empty_board() {
        let input = "\
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));
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

        let board = GameBoard::parse(input, create_test_solution(2, 4));
        println!("Board: {:?}", board);

        let clue = Clue::not_adjacent(Tile::new(0, 'a'), Tile::new(1, 'a'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1a not col 0 (Converging)")));
        assert!(deductions.contains(&Deduction::parse("1a not col 2 (Converging)")));

        let clue = Clue::not_adjacent(Tile::new(1, 'a'), Tile::new(0, 'a'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1a not col 0 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("1a not col 2 (Constraint)")));
    }

    #[test]
    fn test_deduce_not_adjacent_solvable_board() {
        let input = "\
0|abcd|<B> |abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));
        println!("Board: {:?}", board);

        let clue = Clue::not_adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("0a is col 3 (LastRemaining)")));
    }

    #[test]
    fn test_deduce_all_in_column_empty_board() {
        let input = "\
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------
2|abcd|abcd|abcd|abcd|";

        let board = GameBoard::parse(input, create_test_solution(3, 4));

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

        let board = GameBoard::parse(input, create_test_solution(3, 4));
        println!("Board: {:?}", board);

        let clue = Clue::parse("|+0a,+1b,+2c|");

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1b is col 2 (LastRemaining)")));
        assert!(deductions.contains(&Deduction::parse("2c is col 2 (LastRemaining)")));
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

        let board = GameBoard::parse(input, create_test_solution(3, 4));
        println!("Board: {:?}", board);

        let clue = Clue::three_in_column(Tile::new(0, 'a'), Tile::new(1, 'a'), Tile::new(2, 'a'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 4);
        assert!(deductions.contains(&Deduction::parse("0a not col 0 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("1a not col 2 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("2a not col 0 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("2a not col 2 (Constraint)")));
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

        let board = GameBoard::parse(input, create_test_solution(3, 4));

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

        let board = GameBoard::parse(input, create_test_solution(3, 4));
        println!("Board: {:?}", board);

        let clue =
            Clue::two_in_column_without(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("0a not col 2 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("2c not col 2 (Constraint)")));
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

        let board = GameBoard::parse(input, create_test_solution(3, 4));

        let clue =
            Clue::two_in_column_without(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("2c is col 2 (LastRemaining)")));
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
        let board = GameBoard::parse(input, create_test_solution(3, 4));

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
        let board = GameBoard::parse(input, create_test_solution(3, 4));

        let clue =
            Clue::one_matches_either(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("2c is col 1 (LastRemaining)")));
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
        let board = GameBoard::parse(input, create_test_solution(3, 4));

        let clue =
            Clue::one_matches_either(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("2c not col 1")));
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
        let board = GameBoard::parse(input, create_test_solution(3, 4));

        let clue =
            Clue::one_matches_either(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("0a not col 0")));
        assert!(deductions.contains(&Deduction::parse("0a not col 2")));
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_deduce_hidden_pairs(_: &mut UsingLogger) {
        let input = "\
0|ab  |ab  |abcd|abcd|
----------------------
1|abcd|abcd|abcd|abcd|
----------------------
";
        let board = GameBoard::parse(input, create_test_solution(2, 4));
        println!("Board: {:?}", board);

        let deductions = deduce_hidden_sets(&board);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 4);
        assert!(deductions.contains(&Deduction::parse("0a not col 3 (HiddenSet)")));
        assert!(deductions.contains(&Deduction::parse("0b not col 3 (HiddenSet)")));
        assert!(deductions.contains(&Deduction::parse("0a not col 2 (HiddenSet)")));
        assert!(deductions.contains(&Deduction::parse("0b not col 2 (HiddenSet)")));
    }

    #[test]
    fn test_left_of_handler_empty_board() {
        let input = "\
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));
        println!("Board: {:?}", board);

        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(1, 'b'));

        let deductions = deduce_clue(&board, &clue);

        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("0a not col 3 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("1b not col 0 (Constraint)")));
    }

    #[test]
    fn test_left_of_handler_partially_solved() {
        let input = "\
0| bcd|abcd|abcd| bcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));
        println!("Board: {:?}", board);

        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(1, 'b'));
        let deductions = deduce_clue(&board, &clue);

        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1b not col 0 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("1b not col 1 (Constraint)")));
    }

    #[test]
    fn test_left_of_handler_solvable() {
        let input = "\
0|abcd|abcd|<A> |abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));
        println!("Board: {:?}", board);

        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(1, 'b'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("1b is col 3 (LastRemaining)")));
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_eliminate_invalid_solutions(_: &mut UsingLogger) {
        let input = "\
0|ab  |abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));
        println!("Board: {:?}", board);

        let clue = Clue::adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'));

        let deductions = deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 6);
        assert!(deductions.contains(&Deduction::parse("0c not col 1 (Converging)")));
        assert!(deductions.contains(&Deduction::parse("0d not col 1 (Converging)")));

        assert!(deductions.contains(&Deduction::parse("0a not col 2 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("0b not col 2 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("0a not col 3 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("0b not col 3 (Constraint)")));
    }
}
