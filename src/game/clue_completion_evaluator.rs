use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use log::trace;

use crate::model::{Clue, GameBoard, Tile};

use super::clue_constraint::{
    create_clue_constraint, BinaryConstraint, NotInSameColumnConstraint, TernaryConstraint,
};

/// A simple Tile type.
// #[derive(Debug)]
// /// A work item that the solver must process.
// enum WorkItem {
//     Binary {
//         constraint: Rc<dyn BinaryConstraint>,
//         x: Tile,
//         y: Tile,
//     },
//     Ternary(Rc<dyn TernaryConstraint>),
// }

// /// The solver state.
// /// - `domains` maps each tile to its set of possible column values.
// /// - `binary_constraints` and `ternary_constraints` store our constraints.
// struct Solver {
//     domains: HashMap<Tile, HashSet<usize>>,
//     binary_constraints: Vec<Rc<dyn BinaryConstraint>>,
//     ternary_constraints: Vec<Rc<dyn TernaryConstraint>>,
//     worklist: Vec<WorkItem>,
// }

// impl Solver {
//     pub fn new(
//         domains: HashMap<Tile, HashSet<usize>>,
//         binary_constraints: Vec<Rc<dyn BinaryConstraint>>,
//         ternary_constraints: Vec<Rc<dyn TernaryConstraint>>,
//     ) -> Self {
//         let mut solver = Self {
//             domains,
//             binary_constraints,
//             ternary_constraints,
//             worklist: Vec::new(),
//         };
//         solver.worklist = solver.initialize_worklist();
//         solver
//     }

//     /// Performs a single iteration of constraint propagation.
//     /// Returns Some((tile, removed_value)) if a reduction was made;
//     /// otherwise, returns None.
//     pub fn ac3_iteration(&mut self) -> Option<Deduction> {
//         while let Some(item) = self.worklist.pop() {
//             match item {
//                 WorkItem::Binary { constraint, x, y } => {
//                     if let Some((tile, col)) = self.process_binary_item(constraint.as_ref(), x, y) {
//                         return Some(Deduction {
//                             column: col,
//                             tile_assertion: TileAssertion {
//                                 tile,
//                                 assertion: false,
//                             },
//                         });
//                     }
//                 }
//                 WorkItem::Ternary(constraint) => {
//                     if let Some((tile, col)) = self.process_ternary_item(constraint.as_ref()) {
//                         return Some(Deduction {
//                             column: col,
//                             tile_assertion: TileAssertion {
//                                 tile,
//                                 assertion: false,
//                             },
//                         });
//                     }
//                 }
//             }
//         }
//         None
//     }

//     /// Builds the initial worklist with all binary (both directions) and ternary constraints.
//     fn initialize_worklist(&self) -> Vec<WorkItem> {
//         let mut worklist = Vec::new();
//         for constraint in &self.binary_constraints {
//             let (x, y) = constraint.vars();
//             worklist.push(WorkItem::Binary {
//                 constraint: Rc::clone(constraint),
//                 x,
//                 y,
//             });
//             worklist.push(WorkItem::Binary {
//                 constraint: Rc::clone(constraint),
//                 x: y,
//                 y: x,
//             });
//         }
//         for constraint in &self.ternary_constraints {
//             worklist.push(WorkItem::Ternary(Rc::clone(constraint)));
//         }
//         worklist
//     }

//     /// Processes a single binary work item.
//     /// For each value in x’s domain, checks for support in y’s domain.
//     /// If a value is unsupported, it is removed and a reduction is returned.
//     fn process_binary_item<'a>(
//         &mut self,
//         constraint: &dyn BinaryConstraint,
//         x: Tile,
//         y: Tile,
//     ) -> Option<(Tile, usize)> {
//         let domain_x = self.domains.get(&x).cloned().unwrap_or_default();
//         for &vx in &domain_x {
//             let domain_y = self.domains.get(&y).unwrap();
//             let has_support = domain_y.iter().any(|&vy| constraint.valid(vx, vy));
//             if !has_support {
//                 self.domains.get_mut(&x).unwrap().remove(&vx);
//                 self.enqueue_related_constraints(x);
//                 return Some((x, vx));
//             }
//         }
//         None
//     }

//     /// Processes a single ternary work item.
//     /// For each variable involved in the ternary constraint, and for each of its values,
//     /// checks whether there exists a valid complete assignment.
//     /// If not, the value is removed and a reduction is returned.
//     fn process_ternary_item(
//         &mut self,
//         constraint: &dyn TernaryConstraint,
//     ) -> Option<(Tile, usize)> {
//         let vars = constraint.vars();
//         if vars.len() != 3 {
//             return None;
//         }
//         for (i, &tile) in vars.iter().enumerate() {
//             let domain_tile = self.domains.get(&tile).cloned().unwrap_or_default();
//             for &val in &domain_tile {
//                 if !self.has_valid_assignment(constraint, i, val, &vars) {
//                     self.domains.get_mut(&tile).unwrap().remove(&val);
//                     self.enqueue_related_constraints(tile);
//                     return Some((tile, val));
//                 }
//             }
//         }
//         None
//     }

//     /// Checks whether, for a ternary constraint, a fixed value for one variable
//     /// can be extended to a full valid assignment for all three variables.
//     /// `fixed_index` is the position of the variable in the constraint's variable list,
//     /// and `fixed_val` is the candidate value.
//     fn has_valid_assignment(
//         &self,
//         constraint: &dyn TernaryConstraint,
//         fixed_index: usize,
//         fixed_val: usize,
//         vars: &Vec<Tile>,
//     ) -> bool {
//         // For a ternary constraint, assume exactly 3 variables.
//         let mut assignment = vec![0; 3];
//         assignment[fixed_index] = fixed_val;
//         // Identify the other two variables.
//         let other_indices: Vec<usize> = (0..3).filter(|&i| i != fixed_index).collect();
//         let domain0 = self.domains.get(&vars[other_indices[0]]).unwrap();
//         let domain1 = self.domains.get(&vars[other_indices[1]]).unwrap();
//         // Iterate over the Cartesian product of the other two domains.
//         for &val0 in domain0 {
//             for &val1 in domain1 {
//                 if fixed_index == 0 {
//                     assignment[1] = val0;
//                     assignment[2] = val1;
//                 } else if fixed_index == 1 {
//                     assignment[0] = val0;
//                     assignment[2] = val1;
//                 } else {
//                     assignment[0] = val0;
//                     assignment[1] = val1;
//                 }
//                 if constraint.valid(&assignment) {
//                     return true;
//                 }
//             }
//         }
//         false
//     }

//     /// Enqueues all binary and ternary constraints that involve the given tile.
//     fn enqueue_related_constraints(&mut self, tile: Tile) {
//         for constraint in &self.binary_constraints {
//             let (a, b) = constraint.vars();
//             if a == tile || b == tile {
//                 self.worklist.push(WorkItem::Binary {
//                     constraint: Rc::clone(constraint),
//                     x: a,
//                     y: b,
//                 });
//                 self.worklist.push(WorkItem::Binary {
//                     constraint: Rc::clone(constraint),
//                     x: b,
//                     y: a,
//                 });
//             }
//         }
//         for constraint in &self.ternary_constraints {
//             if constraint.vars().contains(&tile) {
//                 self.worklist.push(WorkItem::Ternary(Rc::clone(constraint)));
//             }
//         }
//     }
// }

fn get_domains_and_constraints(
    clue: &Clue,
    board: &GameBoard,
) -> (
    HashMap<Tile, HashSet<usize>>,
    Vec<Rc<dyn BinaryConstraint>>,
    Vec<Rc<dyn TernaryConstraint>>,
) {
    let mut domains: HashMap<Tile, HashSet<usize>> = HashMap::new();
    for assertion in clue.assertions.iter() {
        let possible_cols: HashSet<usize> =
            board.get_possible_cols_for_tile(assertion.tile).collect();
        domains.insert(assertion.tile, possible_cols);
    }
    let mut binary_constraints: Vec<Rc<dyn BinaryConstraint>> = Vec::new();
    let mut ternary_constraints: Vec<Rc<dyn TernaryConstraint>> = Vec::new();

    let mut tiles: Vec<Tile> = domains.keys().cloned().collect();
    tiles.sort_by_key(|tile| domains[tile].len());

    // add binary constraints for board (tiles cannot occupy the same column)
    for i in 0..tiles.len() {
        for j in i + 1..tiles.len() {
            if tiles[i].row == tiles[j].row {
                binary_constraints.push(Rc::new(NotInSameColumnConstraint {
                    tile_a: tiles[i],
                    tile_b: tiles[j],
                }));
            }
        }
    }

    let clue_constraint_set = create_clue_constraint(clue).constraints();
    for binary_constraint in clue_constraint_set.binary_constraints {
        binary_constraints.push(binary_constraint.into());
    }
    for ternary_constraint in clue_constraint_set.ternary_constraints {
        ternary_constraints.push(ternary_constraint.into());
    }

    (domains, binary_constraints, ternary_constraints)
}

fn possible_violations_for_domain(
    prior_domains: &HashMap<Tile, usize>,
    domains: &HashMap<Tile, HashSet<usize>>,
    binary_constraints: &Vec<Rc<dyn BinaryConstraint>>,
    ternary_constraints: &Vec<Rc<dyn TernaryConstraint>>,
    variables: &Vec<Tile>,
) -> bool {
    // are we at a leaf?
    if variables.len() == 0 {
        trace!(
            target: "clue_completion_evaluator",
            "Checking constraints for prior_domains: {:?}",
            prior_domains
        );
        // check all constraints for prior_domains
        for constraint in binary_constraints {
            let (a, b) = constraint.vars();
            if prior_domains.contains_key(&a) && prior_domains.contains_key(&b) {
                let a_domain = prior_domains[&a];
                let b_domain = prior_domains[&b];
                if !constraint.valid(a_domain, b_domain) {
                    // violation found
                    return true;
                }
            }
        }
        // check ternary constraints
        for constraint in ternary_constraints {
            let vars = constraint.vars();
            let values: Vec<usize> = vars
                .iter()
                .flat_map(|v| prior_domains.get(v).cloned())
                .collect();
            if vars.len() == values.len() {
                if !constraint.valid(&values) {
                    return true;
                }
            }
        }
    } else {
        // continue to branch
        let variable = variables[0];
        let rest = variables[1..].to_vec();
        for value in domains[&variable].iter() {
            trace!(
                target: "clue_completion_evaluator",
                "Checking value: {:?} for variable: {:?}",
                value,
                variable
            );
            let mut new_domains = prior_domains.clone();
            new_domains.insert(variable, *value);
            let col_chosen_for_row = prior_domains
                .iter()
                .filter(|(tile, _)| tile.row == variable.row)
                .any(|(_, col)| col == value);

            if col_chosen_for_row {
                trace!(
                    target: "clue_completion_evaluator",
                    "Implicitly filtering value: {:?} for variable: {:?}",
                    value,
                    variable
                );
                // implicitly filter these as auto-solver eliminates multiple tiles from occupying the same row & column
                continue;
            }

            if possible_violations_for_domain(
                &new_domains,
                domains,
                binary_constraints,
                ternary_constraints,
                &rest,
            ) {
                return true;
            }
        }
    }
    false
}

pub fn is_clue_completed(clue: &Clue, board: &GameBoard) -> bool {
    let (domains, binary_constraints, ternary_constraints) =
        get_domains_and_constraints(clue, board);

    trace!(
        target: "clue_completion_evaluator",
        "Domains: {:?}",
        domains
    );

    trace!(
        target: "clue_completion_evaluator",
        "Binary constraints: {:?}",
        binary_constraints
    );
    trace!(
        target: "clue_completion_evaluator",
        "Ternary constraints: {:?}",
        ternary_constraints
    );

    let mut variables: Vec<Tile> = domains.keys().cloned().collect();
    variables.sort_by_key(|tile| domains[tile].len());

    let clue_has_violation = possible_violations_for_domain(
        &HashMap::new(),
        &domains,
        &binary_constraints,
        &ternary_constraints,
        &variables,
    );

    !clue_has_violation
}

// pub fn is_clue_completed(clue: &Clue, board: &GameBoard) -> bool {
//     let (domains, binary_constraints, ternary_constraints) =
//         get_domains_and_constraints(clue, board);
//     let mut solver = Solver::new(domains, binary_constraints, ternary_constraints);
//     solver.ac3_iteration().is_none()
// }

// use log::trace;

// use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

// use crate::game::solver::{
//     create_handler, group_assertions_by_coordinates, CellSolutionAssertion, Coordinates,
// };
// use crate::model::{Clue, Deduction, GameBoard, Tile, TileAssertion};

// impl

// /// returns a reduction when it finds a value for which no other arc is satisfied
// fn arc_iteration(
//     variables: &HashMap<Tile, HashSet<usize>>,
//     arc_work_list: Vec<Box<dyn BinaryConstraint>>,
//     // mut deductions: Vec<Deduction>,
//     // generation: usize,
//     // control: Tile,
//     // remaining: Vec<Tile>,
//     // limit: usize,
// ): Option<(Tile, usize)> {
//     // pick one column at a time
//     for col in tile_possible_cols[&control].iter() {
//         let hypothetical_

//     }
// }

// pub fn is_clue_completed(clue: &Clue, init_board: &GameBoard, order: usize) -> bool {
//     let mut board = init_board.clone();
//     let clue_handler = create_handler(clue);

//     let mut tile_possible_cols: HashMap<Tile, HashSet<usize>> = HashMap::new();
//     for assertion in clue.assertions.iter() {
//         let possible_cols: HashSet<usize> =
//             board.get_possible_cols_for_tile(assertion.tile).collect();
//         tile_possible_cols.insert(assertion.tile, possible_cols);
//     }

//     let mut eval_order: Vec<Tile> = tile_possible_cols.keys().cloned().collect();
//     eval_order.sort_by_key(|tile| tile_possible_cols[tile].len());

//     for tile in eval_order {
//         let possible_cols = tile_possible_cols[&tile];
//         for col in possible_cols {}
//     }
// }

// fn is_solution_encoded_in_board(solution: &Vec<(usize, TileAssertion)>, board: &GameBoard) -> bool {
//     for (column, assertion) in solution {
//         if !board.is_known_deduction(*column, *assertion) {
//             return false;
//         }
//     }
//     true
// }

/// Returns true if clue has no further evidence to offer, using arc consistency
/// @param clue The clue to evaluate
/// @param init_board The initial game board state
/// @param order The nth-order of deduction
/// @returns True if the clue is completed and has no more deductions to make
// pub fn is_clue_completed(clue: &Clue, init_board: &GameBoard, order: usize) -> bool {
//     let mut board = init_board.clone();
//     // I think if I get all negative deductions (implicit by selection in other cell), subtract out potential positive assertions for a cell, and check to see that all negative deductions are made, this should be a good surrogate for clue completion
//     // todo - learn more about arc consistency
//     board.auto_solve_all();
//     let clue_handler = create_handler(clue);
//     let mut all_solutions: Vec<Vec<(usize, crate::model::TileAssertion)>> = Vec::new();
//     for column in 0..board.solution.n_variants {
//         let solutions = clue_handler.potential_solutions(&board, column);
//         all_solutions.extend(solutions);
//     }
//     if all_solutions.len() == 0 {
//         return true;
//     } else if all_solutions.len() == 1 && is_solution_encoded_in_board(&all_solutions[0], &board) {
//         return true;
//     } else if order == 1 && all_solutions.len() <= 2 {
//         trace!(
//             target: "clue_completion_evaluator",
//             "All solutions for clue {:?} in order {}: {:?}",
//             clue,
//             order,
//             all_solutions
//         );
//         let non_known_assertions: Vec<(usize, TileAssertion)> = all_solutions
//             .iter()
//             .flatten()
//             .filter(|(column, assertion)| !board.is_known_deduction(*column, *assertion))
//             .cloned()
//             .collect();

//         let solutions_by_coordinates: HashMap<Coordinates, CellSolutionAssertion> =
//             group_assertions_by_coordinates(&all_solutions);
//         for row in 0..board.solution.n_rows {
//             let all_positive_assertions_in_row: BTreeSet<char> = (0..board.solution.n_variants)
//                 .flat_map(|col| {
//                     let coordinates = Coordinates { row, column: col };
//                     solutions_by_coordinates.get(&coordinates)
//                 })
//                 .flat_map(|cs| cs.positive_variants.iter())
//                 .cloned()
//                 .collect();

//             // .collect();
//             for col in 0..board.solution.n_variants {
//                 let coordinates = Coordinates { row, column: col };
//                 let cell_solution_assertion = solutions_by_coordinates
//                     .get(&coordinates)
//                     .cloned()
//                     .unwrap_or_default();
//                 trace!(
//                     target: "clue_completion_evaluator",
//                     "Check cell {:?}: {:?}",
//                     coordinates,
//                     cell_solution_assertion
//                 );
//                 if cell_solution_assertion.positive_count > 0 {
//                     for variant in board.solution.variants.iter() {
//                         let variant_should_be_here =
//                             cell_solution_assertion.positive_variants.contains(variant);
//                         if variant_should_be_here {
//                             trace!(
//                                 target: "clue_completion_evaluator",
//                                 "Variant {} should be here",
//                                 variant
//                             );
//                             if !board.is_candidate_available(row, col, *variant) {
//                                 return false;
//                             }
//                         } else {
//                             trace!(
//                                 target: "clue_completion_evaluator",
//                                 "Variant {} should not be here",
//                                 variant
//                             );
//                             // not supposed to be here, but is?
//                             if board.is_candidate_available(row, col, *variant) {
//                                 return false;
//                             }
//                         }
//                     }
//                 } else {
//                     for variant in all_positive_assertions_in_row.iter() {
//                         // if variant is not in the cell solution assertion, it should not be available in this cell
//                         if board.is_candidate_available(row, col, *variant) {
//                             return false;
//                         }
//                     }
//                 }
//             }
//         }

//         // iterate through all non-known assertions; if 2nd order non-known deductions exist, consider clue incomplete
//         for (column, assertion) in non_known_assertions.iter() {
//             let mut hypothetical_board = board.clone();
//             hypothetical_board.apply_deduction(&Deduction {
//                 column: *column,
//                 tile_assertion: TileAssertion {
//                     tile: assertion.tile,
//                     assertion: assertion.assertion,
//                 },
//             });
//             println!("Hypothetical board: {:?}", hypothetical_board);
//             if !is_clue_completed(clue, &hypothetical_board, order + 1) {
//                 return false;
//             }
//         }
//         true
//     } else {
//         return false;
//     }
// }

#[cfg(test)]
mod tests {
    use test_context::test_context;

    use crate::game::tests::create_test_solution;
    use crate::model::{Clue, Deduction, GameBoard};
    use crate::tests::UsingLogger;

    use super::*;

    #[test_context(UsingLogger)]
    #[test]
    fn test_is_clue_completed_two_adjacent(_: &mut UsingLogger) {
        let input = "\
0|abcd|<B> |abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let mut board = GameBoard::parse(input, create_test_solution(2, 4));
        println!("Board: {:?}", board);

        let clue = Clue::parse("<+0a,+0b>");
        assert!(
            !is_clue_completed(&clue, &board),
            "Clue should not be completed"
        );

        board.apply_deduction(&Deduction::parse("0a not col 3"));
        println!("Board after deduction: {:?}", board);
        assert!(
            is_clue_completed(&clue, &board),
            "Clue should be completed now that 0a is not in col 3"
        );
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_is_clue_completed_two_not_adjacent(_: &mut UsingLogger) {
        let input = "\
0|abcd|<B> |abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let mut board = GameBoard::parse(input, create_test_solution(2, 4));

        let clue = Clue::parse("<+0b,-0a>");
        assert!(
            !is_clue_completed(&clue, &board),
            "Clue should not be completed"
        );

        board.apply_deduction(&Deduction::parse("0a not col 0"));
        assert!(
            !is_clue_completed(&clue, &board),
            "Clue should not be completed now that 0a is not in col 0"
        );

        board.apply_deduction(&Deduction::parse("0a not col 2"));
        assert!(
            is_clue_completed(&clue, &board),
            "Clue should be completed now that 0a is not in col 0 and 0b is not in col 2"
        );
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_is_clue_completed_three_adjacent_same_row(_: &mut UsingLogger) {
        let input = "\
0|abcd|<B> |abcd|abcd|
----------------------
1|a c | b d|a c | b d|
----------------------
";

        let mut board = GameBoard::parse(input, create_test_solution(2, 4));

        let clue = Clue::parse("<+1a,+0b,+1c>");
        assert!(is_clue_completed(&clue, &board), "Clue should be completed");

        board.show_candidate(3, Tile::new(1, 'c'));

        println!("Board after showing 1c in col 3: {:?}", board);
        assert!(
            !is_clue_completed(&clue, &board),
            "Clue should not be completed now that 1c is in col 3"
        );
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_is_clue_completed_left_of(_: &mut UsingLogger) {
        let clue = Clue::parse("<0a...1c>");
        let input = "\
0|abcd|abcd|abcd|abcd|
----------------------
1|abcd|abcd|abcd|abcd|
----------------------
";

        let mut board = GameBoard::parse(input, create_test_solution(2, 4));

        assert!(
            !is_clue_completed(&clue, &board),
            "Clue should not be completed"
        );

        // clue is fully encoded in board; no 1c can be in the same column as 0a
        let input = "\
0|abcd|abcd| bcd| bcd|
----------------------
1|ab d|ab d|abcd|abcd|
----------------------
";

        let mut board = GameBoard::parse(input, create_test_solution(2, 4));
        assert!(is_clue_completed(&clue, &board), "Clue should be completed");
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_is_clue_completed_two_apart_but_not_middle(_: &mut UsingLogger) {
        let clue = Clue::parse("<+0a,-1b,+0c>");
        let input = "\
0|abcd|abcd|abcd|abcd|
----------------------
1|abcd|abcd|abcd|abcd|
----------------------
";

        let board = GameBoard::parse(input, create_test_solution(2, 4));

        assert!(
            !is_clue_completed(&clue, &board),
            "Clue should not be completed"
        );

        // edges fully encoded, but center still not "punched out"
        let input = "\
0|<A> |abcd|<C> | bcd|
----------------------
1|abcd|abcd|abcd|abcd|
----------------------
";

        let mut board = GameBoard::parse(input, create_test_solution(2, 4));
        assert!(
            !is_clue_completed(&clue, &board),
            "Clue should not be completed"
        );

        // punch out middle
        board.apply_deduction(&Deduction::parse("1b not col 1"));
        assert!(is_clue_completed(&clue, &board), "Clue should be completed");
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_is_clue_completed_all_in_column(_: &mut UsingLogger) {
        let clue = Clue::parse("|+0a,+1a,+2a|");
        let input = "\
0|<A> |abcd|abcd|abcd|
----------------------
1|<A> |abcd|abcd|abcd|
----------------------
2|abcd|abcd| bcd| bcd|
----------------------
";

        let mut board = GameBoard::parse(input, create_test_solution(3, 4));

        assert!(
            !is_clue_completed(&clue, &board),
            "Clue should not be completed"
        );

        // complete that last square

        board.apply_deduction(&Deduction::parse("2a not col 1"));
        assert!(is_clue_completed(&clue, &board), "Clue should be completed");
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_is_clue_completed_all_in_column_with_negative_deduction(_: &mut UsingLogger) {
        let clue = Clue::parse("|+0a,+1a,-2a|");
        let input = "\
0|<A> |abcd|abcd|abcd|
----------------------
1|<A> |abcd|abcd|abcd|
----------------------
2|abcd|abcd|abcd|abcd|
----------------------
";

        let mut board = GameBoard::parse(input, create_test_solution(3, 4));

        assert!(
            !is_clue_completed(&clue, &board),
            "Clue should not be completed"
        );

        // complete that last square

        board.apply_deduction(&Deduction::parse("2a not col 0"));
        assert!(is_clue_completed(&clue, &board), "Clue should be completed");
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_is_clue_completed_one_matches_either(_: &mut UsingLogger) {
        let clue = Clue::parse("|+0a,?1a,?2a|");
        let input = "\
0|abcd|abcd|abcd|abcd|
----------------------
1|<A> |abcd|abcd|abcd|
----------------------
2|abcd|<A> |abcd|abcd|
----------------------
";

        let mut board = GameBoard::parse(input, create_test_solution(3, 4));

        assert!(
            !is_clue_completed(&clue, &board),
            "Clue should not be completed"
        );

        // complete that last square

        board.apply_deduction(&Deduction::parse("0a not col 2"));
        board.apply_deduction(&Deduction::parse("0a not col 3"));

        println!("Board after deductions: {:?}", board);

        assert!(is_clue_completed(&clue, &board), "Clue should be completed");
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_is_clue_completed_one_matches_either_known_first_tile(_: &mut UsingLogger) {
        let clue = Clue::parse("|+0a,?1a,?2a|");
        let input = "\
0|<A> |abcd|abcd|abcd|
----------------------
1|<A> |abcd|abcd|abcd|
----------------------
2|abcd|abcd|abcd|abcd|
----------------------
";

        let mut board = GameBoard::parse(input, create_test_solution(3, 4));

        assert!(
            !is_clue_completed(&clue, &board),
            "Clue should not be completed"
        );

        // complete that last square

        board.apply_deduction(&Deduction::parse("2a not col 0"));

        println!("Board after deductions: {:?}", board);

        assert!(is_clue_completed(&clue, &board), "Clue should be completed");
    }
}
