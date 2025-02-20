use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::model::{Clue, Deduction, GameBoard, Tile, TileAssertion};

use super::{
    candidate_solver::Coordinates,
    clue_constraint::{create_clue_constraint, ConstraintSet, NotInSameColumnConstraint},
};

// When a set of negative deductions eliminate all but one remaining candidate, convert it to a positive deduction
pub fn simplify_deductions(
    board: &GameBoard,
    deductions: Vec<Deduction>,
    clue: &Clue,
) -> Vec<Deduction> {
    let mut deductions_by_row_and_col: HashMap<Coordinates, Vec<Deduction>> = HashMap::new();
    for deduction in deductions {
        deductions_by_row_and_col
            .entry(Coordinates::new(
                deduction.tile_assertion.tile.row,
                deduction.column,
            ))
            .or_insert_with(Vec::new)
            .push(deduction);
    }

    let mut new_deductions = Vec::new();

    for (coordinates, deductions) in deductions_by_row_and_col.into_iter() {
        let mut variants_at_cell: HashSet<char> = board
            .get_available_candidates_at_cell(coordinates.row, coordinates.column)
            .into_iter()
            .collect();
        let negative_deductions: HashSet<char> = deductions
            .iter()
            .filter(|deduction| !deduction.is_positive())
            .map(|deduction| deduction.tile_assertion.tile.variant)
            .collect();
        variants_at_cell.retain(|variant| !negative_deductions.contains(variant));

        if variants_at_cell.len() == 1 {
            let variant = variants_at_cell.into_iter().next().unwrap();
            new_deductions.push(Deduction::new(
                coordinates.column,
                TileAssertion {
                    tile: Tile::new(coordinates.row, variant),
                    assertion: true,
                },
            ));
        } else {
            new_deductions.extend(deductions);
        }
    }

    let clue_tiles: BTreeSet<Tile> = clue
        .assertions
        .iter()
        .map(|assertion| assertion.tile)
        .collect();

    // prefer tiles in the clue first;
    new_deductions.sort_by_key(|deduction| {
        if clue_tiles.contains(&deduction.tile_assertion.tile) {
            if deduction.is_positive() {
                // prioritize negative deductions at the front as these are easier to progressively understand
                1
            } else {
                0
            }
        } else {
            10
        }
    });
    new_deductions
}

pub fn get_domains_and_constraints(
    clue: &Clue,
    board: &GameBoard,
) -> (BTreeMap<Tile, BTreeSet<usize>>, ConstraintSet) {
    let mut domains: BTreeMap<Tile, BTreeSet<usize>> = BTreeMap::new();
    for assertion in clue.assertions.iter() {
        let possible_cols: BTreeSet<usize> =
            board.get_possible_cols_for_tile(assertion.tile).collect();
        domains.insert(assertion.tile, possible_cols);
    }
    let mut constraint_set = ConstraintSet::default();
    // let mut binary_constraints: Vec<Rc<dyn BinaryConstraint>> = Vec::new();
    // let mut ternary_constraints: Vec<Rc<dyn TernaryConstraint>> = Vec::new();

    let mut tiles: Vec<Tile> = domains.keys().cloned().collect();
    tiles.sort_by_key(|tile| domains[tile].len());

    // add binary constraints for board (tiles cannot occupy the same column)
    for i in 0..tiles.len() {
        for j in i + 1..tiles.len() {
            if tiles[i].row == tiles[j].row {
                constraint_set
                    .binary_constraints
                    .push(Box::new(NotInSameColumnConstraint {
                        tile_a: tiles[i],
                        tile_b: tiles[j],
                    }));
            }
        }
    }

    let clue_constraint_set = create_clue_constraint(clue).constraints(board.solution.difficulty);
    constraint_set
        .unary_constraints
        .extend(clue_constraint_set.unary_constraints);

    constraint_set
        .binary_constraints
        .extend(clue_constraint_set.binary_constraints);

    constraint_set
        .ternary_constraints
        .extend(clue_constraint_set.ternary_constraints);

    (domains, constraint_set)
}

#[cfg(test)]
mod tests {
    use test_context::test_context;

    use crate::{game::tests::create_test_solution, model::GameBoard, tests::UsingLogger};

    use super::*;

    #[test_context(UsingLogger)]
    #[test]
    fn test_simplify_deductions(_: &mut UsingLogger) {
        let board = GameBoard::parse(
            "
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------",
            create_test_solution(2, 4),
        );
        let deductions = vec![
            Deduction::new(
                0,
                TileAssertion {
                    tile: Tile::new(0, 'a'),
                    assertion: false,
                },
            ),
            Deduction::new(
                0,
                TileAssertion {
                    tile: Tile::new(0, 'b'),
                    assertion: false,
                },
            ),
            Deduction::new(
                0,
                TileAssertion {
                    tile: Tile::new(0, 'c'),
                    assertion: false,
                },
            ),
        ];
        let clue = Clue::parse("|+0a,+1a|");
        let simplified = simplify_deductions(&board, deductions, &clue);
        assert_eq!(simplified.len(), 1);
        assert_eq!(simplified[0].tile_assertion.tile, Tile::new(0, 'd'));
        assert!(simplified[0].is_positive());
    }
}
