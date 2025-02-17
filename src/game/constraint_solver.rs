use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use log::{trace, warn};

use crate::{
    game::candidate_solver::Coordinates,
    model::{Clue, Deduction, DeductionKind, GameBoard, Tile, TileAssertion},
};

use super::{
    clue_constraint::{BinaryConstraint, TernaryConstraint, UnaryConstraint},
    solver_helpers::get_domains_and_constraints,
};

// / A simple Tile type.
#[derive(Debug)]
/// A work item that the solver must process.
enum WorkItem {
    Unary {
        constraint: Rc<dyn UnaryConstraint>,
        tile: Tile,
    },
    Binary {
        constraint: Rc<dyn BinaryConstraint>,
        x: Tile,
        y: Tile,
    },
    Ternary(Rc<dyn TernaryConstraint>),
}

/// The solver state.
/// - `domains` maps each tile to its set of possible column values.
/// - `binary_constraints` and `ternary_constraints` store our constraints.
pub struct ConstraintSolver {
    domains: BTreeMap<Tile, BTreeSet<usize>>,
    binary_constraints: Vec<Rc<dyn BinaryConstraint>>,
    unary_constraints: Vec<Rc<dyn UnaryConstraint>>,
    ternary_constraints: Vec<Rc<dyn TernaryConstraint>>,
    worklist: Vec<WorkItem>,
}

impl ConstraintSolver {
    fn new(
        domains: BTreeMap<Tile, BTreeSet<usize>>,
        unary_constraints: Vec<Rc<dyn UnaryConstraint>>,
        binary_constraints: Vec<Rc<dyn BinaryConstraint>>,
        ternary_constraints: Vec<Rc<dyn TernaryConstraint>>,
    ) -> Self {
        let mut solver = Self {
            domains,
            binary_constraints,
            unary_constraints,
            ternary_constraints,
            worklist: Vec::new(),
        };
        solver.worklist = solver.initialize_worklist();
        solver
    }

    fn reduce_domains(&mut self) {
        trace!(target: "constraint_solver", "AC3 iteration");
        while let Some(item) = self.worklist.pop() {
            trace!(target: "constraint_solver", "Domains: {:?}", self.domains);
            trace!(target: "constraint_solver", "Processing item: {:?}", item);
            match item {
                WorkItem::Unary { constraint, tile } => {
                    self.process_unary_item(constraint.as_ref(), tile);
                }
                WorkItem::Binary { constraint, x, y } => {
                    self.process_binary_item(constraint.as_ref(), x, y);
                }
                WorkItem::Ternary(constraint) => {
                    self.process_ternary_item(constraint.as_ref());
                }
            }
        }
    }

    /// Builds the initial worklist with all binary (both directions) and ternary constraints.
    fn initialize_worklist(&self) -> Vec<WorkItem> {
        let mut worklist = Vec::new();
        // worklist is in reverse order
        for constraint in &self.ternary_constraints {
            worklist.push(WorkItem::Ternary(Rc::clone(constraint)));
        }
        for constraint in &self.binary_constraints {
            let (x, y) = constraint.vars();
            worklist.push(WorkItem::Binary {
                constraint: Rc::clone(constraint),
                x,
                y,
            });
            worklist.push(WorkItem::Binary {
                constraint: Rc::clone(constraint),
                x: y,
                y: x,
            });
        }
        for constraint in &self.unary_constraints {
            worklist.push(WorkItem::Unary {
                constraint: Rc::clone(constraint),
                tile: constraint.var(),
            });
        }
        worklist
    }

    /// Processes a single binary work item.
    /// For each value in x’s domain, checks for support in y’s domain.
    /// If a value is unsupported, it is removed and a reduction is returned.
    fn process_binary_item<'a>(
        &mut self,
        constraint: &dyn BinaryConstraint,
        eval_x: Tile,
        eval_y: Tile,
    ) {
        let (x, _y) = constraint.vars();
        let domain_x = self.domains.get(&eval_x).cloned().unwrap_or_default();
        for &vx in &domain_x {
            let domain_y = self.domains.get(&eval_y).unwrap();
            let has_support = domain_y.iter().any(|&vy| {
                if eval_x == x {
                    constraint.valid(vx, vy)
                } else {
                    constraint.valid(vy, vx)
                }
            });
            if !has_support {
                trace!(
                    target: "constraint_solver",
                    "Removing value {} from domain of tile {:?}",
                    vx,
                    eval_x
                );
                self.domains.get_mut(&eval_x).unwrap().remove(&vx);
                self.enqueue_related_constraints(eval_x);
            }
        }
    }

    /// Processes a single ternary work item.
    /// For each variable involved in the ternary constraint, and for each of its values,
    /// checks whether there exists a valid complete assignment.
    /// If not, the value is removed and a reduction is returned.
    fn process_ternary_item(&mut self, constraint: &dyn TernaryConstraint) {
        let vars = constraint.vars();
        if vars.len() != 3 {
            return;
        }
        for (i, &tile) in vars.iter().enumerate() {
            let domain_tile = self.domains.get(&tile).cloned().unwrap_or_default();
            for &val in &domain_tile {
                if !self.has_valid_assignment(constraint, i, val, &vars) {
                    trace!(
                        target: "constraint_solver",
                        "Removing value {} from domain of tile {:?}",
                        val,
                        tile
                    );
                    self.domains.get_mut(&tile).unwrap().remove(&val);
                    self.enqueue_related_constraints(tile);
                }
            }
        }
    }

    /// Checks whether, for a ternary constraint, a fixed value for one variable
    /// can be extended to a full valid assignment for all three variables.
    /// `fixed_index` is the position of the variable in the constraint's variable list,
    /// and `fixed_val` is the candidate value.
    fn has_valid_assignment(
        &self,
        constraint: &dyn TernaryConstraint,
        fixed_index: usize,
        fixed_val: usize,
        vars: &Vec<Tile>,
    ) -> bool {
        assert!(vars.len() == 3);
        // For a ternary constraint, assume exactly 3 variables.
        let mut assignment = vec![0; 3];
        assignment[fixed_index] = fixed_val;
        // Identify the other two variables.
        let other_indices: Vec<usize> = (0..3).filter(|&i| i != fixed_index).collect();
        let domain0 = self.domains.get(&vars[other_indices[0]]).unwrap();
        let domain1 = self.domains.get(&vars[other_indices[1]]).unwrap();
        // Iterate over the Cartesian product of the other two domains.
        for &val0 in domain0 {
            for &val1 in domain1 {
                if fixed_index == 0 {
                    assignment[1] = val0;
                    assignment[2] = val1;
                } else if fixed_index == 1 {
                    assignment[0] = val0;
                    assignment[2] = val1;
                } else {
                    assignment[0] = val0;
                    assignment[1] = val1;
                }
                if constraint.valid(&assignment) {
                    return true;
                }
            }
        }
        false
    }

    /// Enqueues all binary and ternary constraints that involve the given tile.
    fn enqueue_related_constraints(&mut self, tile: Tile) {
        for constraint in &self.binary_constraints {
            let (a, b) = constraint.vars();
            if a == tile || b == tile {
                self.worklist.push(WorkItem::Binary {
                    constraint: Rc::clone(constraint),
                    x: a,
                    y: b,
                });
                self.worklist.push(WorkItem::Binary {
                    constraint: Rc::clone(constraint),
                    x: b,
                    y: a,
                });
            }
        }
        for constraint in &self.ternary_constraints {
            if constraint.vars().contains(&tile) {
                self.worklist.push(WorkItem::Ternary(Rc::clone(constraint)));
            }
        }
    }

    fn process_unary_item(&mut self, as_ref: &dyn UnaryConstraint, tile: Tile) {
        let domain = self.domains.get(&tile).cloned().unwrap_or_default();
        for &val in &domain {
            if !as_ref.valid(val) {
                trace!(
                    target: "constraint_solver",
                    "Removing value {} from domain of tile {:?}",
                    val,
                    tile
                );
                self.domains.get_mut(&tile).unwrap().remove(&val);
                self.enqueue_related_constraints(tile);
            }
        }
    }

    pub fn deduce_clue(board: &GameBoard, clue: &Clue) -> Vec<Deduction> {
        let (domains, constraint_set) = get_domains_and_constraints(clue, board);
        let unary_constraints: Vec<Rc<dyn UnaryConstraint>> = constraint_set
            .unary_constraints
            .into_iter()
            .map(|c| c.into())
            .collect::<Vec<_>>();

        let binary_constraints: Vec<Rc<dyn BinaryConstraint>> = constraint_set
            .binary_constraints
            .into_iter()
            .map(|c| c.into())
            .collect::<Vec<_>>();
        let ternary_constraints: Vec<Rc<dyn TernaryConstraint>> = constraint_set
            .ternary_constraints
            .into_iter()
            .map(|c| c.into())
            .collect::<Vec<_>>();

        let mut solver = ConstraintSolver::new(
            domains.clone(),
            unary_constraints,
            binary_constraints,
            ternary_constraints,
        );
        trace!(target: "constraint_solver", "Domains before: {:?}", solver.domains);
        solver.reduce_domains();
        trace!(target: "constraint_solver", "Domains after: {:?}", solver.domains);
        let mut deductions = Vec::new();
        let solved_coordinates: BTreeMap<Coordinates, Tile> = solver
            .domains
            .iter()
            .filter(|(_, domain)| domain.len() == 1)
            .map(|(tile, domain)| {
                (
                    Coordinates::new(tile.row, *domain.iter().next().unwrap()),
                    tile.clone(),
                )
            })
            .collect();

        for (tile, domain) in domains.iter() {
            let domain_after = solver.domains.get(tile).unwrap();
            if domain.len() == domain_after.len() {
                continue;
            }
            if domain_after.len() == 0 {
                warn!("Domain for tile {:?} is empty", tile);
            } else if domain_after.len() == 1 {
                let val = domain_after.iter().next().unwrap();
                deductions.push(Deduction::new_with_kind(
                    *val,
                    TileAssertion {
                        tile: tile.clone(),
                        assertion: true,
                    },
                    DeductionKind::LastRemaining,
                ));
            } else {
                let removed_values: Vec<usize> = domain.difference(domain_after).cloned().collect();
                for val in removed_values {
                    let coordinates = Coordinates::new(tile.row, val);
                    if !solved_coordinates.contains_key(&coordinates) {
                        deductions.push(Deduction::new_with_kind(
                            val,
                            TileAssertion {
                                tile: tile.clone(),
                                assertion: false,
                            },
                            DeductionKind::Constraint,
                        ));
                    }
                }
            }
        }
        deductions
    }
}

#[cfg(test)]
mod tests {
    use test_context::test_context;

    use crate::{game::tests::create_test_solution, tests::UsingLogger};

    use super::*;

    #[test_context(UsingLogger)]
    #[test]
    fn test_three_adjacent(_: &mut UsingLogger) {
        let clue = Clue::three_adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'), Tile::new(0, 'c'));

        let input = "
        0|abcd|abcd|abcd|abcd|
        -----------------
        1|abcd|abcd|abcd|abcd|
        -----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));

        let deductions = ConstraintSolver::deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);

        //         let input = "
        // 0|<A> |abcd|abcd|abcd|
        // -----------------
        // 1|abcd|abcd|abcd|abcd|
        // -----------------";

        //         let mut board = GameBoard::parse(input, create_test_solution(2, 4));

        //         let deductions = ConstraintSolver::deduce_clue(&board, &clue);
        //         println!("Deductions: {:?}", deductions);
        //         board.apply_deductions(&deductions);
        //         println!("Board after deductions: {:?}", board);
    }

    #[test]
    fn test_two_apart_not_middle() {
        let input = "
0|<A> |abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));
        let clue =
            Clue::two_apart_not_middle(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(0, 'c'));

        let deductions = ConstraintSolver::deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2); // Adjust as needed
        assert!(deductions.contains(&Deduction::parse("0c is col 2 (LastRemaining)")));
        assert!(deductions.contains(&Deduction::parse("1b not col 1 (Constraint)")));
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_left_of(_: &mut UsingLogger) {
        let input = "
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));
        let clue = Clue::left_of(Tile::new(0, 'a'), Tile::new(0, 'b'));

        let deductions = ConstraintSolver::deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2); // Adjust as needed
        assert!(deductions.contains(&Deduction::parse("0a not col 3 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("0b not col 0 (Constraint)")));
    }

    #[test]
    fn test_two_adjacent() {
        let input = "
0|abcd|abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));
        let clue = Clue::adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'));

        let deductions = ConstraintSolver::deduce_clue(&board, &clue);
        assert_eq!(deductions.len(), 0); // Adjust as needed
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_not_adjacent(_: &mut UsingLogger) {
        let input = "
0|<A> |abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));
        let clue = Clue::not_adjacent(Tile::new(0, 'a'), Tile::new(0, 'b'));

        let deductions = ConstraintSolver::deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1); // Adjust as needed
        assert!(deductions.contains(&Deduction::parse("0b not col 1 (Constraint)")));
    }

    #[test]
    fn test_three_in_column() {
        let input = "
0|abcd|abcd|<C> |<D> |
-----------------
1|abcd|abcd|abcd|abcd|
-----------------
2|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(3, 4));
        let clue = Clue::three_in_column(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'c'));

        let deductions = ConstraintSolver::deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 4);
        assert!(deductions.contains(&Deduction::parse("1b not col 2 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("1b not col 3 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("2c not col 2 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("2c not col 3 (Constraint)")));
    }

    #[test]
    fn test_two_in_column() {
        let input = "
0|abcd|abcd|<C> |<D> |
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));
        let clue = Clue::two_in_column(Tile::new(0, 'a'), Tile::new(1, 'a'));

        let deductions = ConstraintSolver::deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1a not col 2 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("1a not col 3 (Constraint)")));
    }

    #[test]
    fn test_one_matches_either() {
        let input = "
0|abcd|abcd|abcd|abcd|
-----------------
1|<A> |abcd|abcd|abcd|
-----------------
2|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(3, 4));
        let clue =
            Clue::one_matches_either(Tile::new(0, 'a'), Tile::new(1, 'a'), Tile::new(2, 'b'));

        let deductions = ConstraintSolver::deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("2b not col 0 (Constraint)")));
    }

    #[test]
    fn test_not_in_same_column() {
        let input = "
0|<A> |abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(2, 4));
        let clue = Clue::two_not_in_same_column(Tile::new(0, 'a'), Tile::new(1, 'a'));

        let deductions = ConstraintSolver::deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 1);
        assert!(deductions.contains(&Deduction::parse("1a not col 0 (Constraint)")));
    }

    #[test]
    fn test_two_in_column_without() {
        let input = "
0|<A> |abcd|abcd|abcd|
-----------------
1|abcd|abcd|abcd|abcd|
-----------------
2|abcd|abcd|abcd|abcd|
-----------------";

        let board = GameBoard::parse(input, create_test_solution(3, 4));
        let clue =
            Clue::two_in_column_without(Tile::new(0, 'a'), Tile::new(1, 'b'), Tile::new(2, 'a'));

        let deductions = ConstraintSolver::deduce_clue(&board, &clue);
        println!("Deductions: {:?}", deductions);
        assert_eq!(deductions.len(), 2);
        assert!(deductions.contains(&Deduction::parse("1b not col 0 (Constraint)")));
        assert!(deductions.contains(&Deduction::parse("2a is col 0 (LastRemaining)")));
    }
}
