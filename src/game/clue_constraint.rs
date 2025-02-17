use std::hash::Hash;

use log::trace;

use crate::model::{
    Clue, ClueType, Difficulty, GameBoard, HorizontalClueType, PartialSolution, Tile,
    TileAssertion, VerticalClueType,
};

/// Unary constraint trait: relates one tile.
pub trait UnaryConstraint: std::fmt::Debug {
    fn var(&self) -> Tile;
    fn valid(&self, value: usize) -> bool;
}

/// Binary constraint trait: relates two tiles.
pub trait BinaryConstraint: std::fmt::Debug {
    /// Returns the two tiles involved in the constraint.
    fn vars(&self) -> (Tile, Tile);
    /// Checks whether the constraint is satisfied for a pair of values.
    fn valid(&self, x: usize, y: usize) -> bool;
}

/// Ternary constraint trait: relates three tiles.
pub trait TernaryConstraint: std::fmt::Debug {
    /// Returns the three tiles involved in the constraint.
    fn vars(&self) -> Vec<Tile>;
    /// Checks whether the constraint is satisfied for the given assignment of values.
    fn valid(&self, values: &Vec<usize>) -> bool;
}

#[derive(Debug, Clone, Hash)]
pub struct NotInSameColumnConstraint {
    pub tile_a: Tile,
    pub tile_b: Tile,
}

impl BinaryConstraint for NotInSameColumnConstraint {
    fn vars(&self) -> (Tile, Tile) {
        (self.tile_a, self.tile_b)
    }

    fn valid(&self, col_a: usize, col_b: usize) -> bool {
        col_a != col_b
    }
}

#[derive(Debug, Clone, Hash)]
pub struct EdgeConstraint {
    pub tile: Tile,
    pub difficulty: Difficulty,
    pub allow_left: bool,
    pub allow_right: bool,
}

impl UnaryConstraint for EdgeConstraint {
    fn var(&self) -> Tile {
        self.tile
    }

    fn valid(&self, value: usize) -> bool {
        let ncols = self.difficulty.n_cols();
        if !self.allow_left && value <= 0 {
            return false;
        }
        if !self.allow_right && value >= ncols - 1 {
            return false;
        }
        true
    }
}

#[derive(Debug, Clone, Hash)]
pub struct InSameColumnConstraint {
    pub tile_a: Tile,
    pub tile_b: Tile,
}

impl BinaryConstraint for InSameColumnConstraint {
    fn vars(&self) -> (Tile, Tile) {
        (self.tile_a, self.tile_b)
    }

    fn valid(&self, col_a: usize, col_b: usize) -> bool {
        col_a == col_b
    }
}

#[derive(Debug, Clone, Hash)]
pub struct AdjacentConstraint {
    pub tile_a: Tile,
    pub tile_b: Tile,
    pub distance: usize,
}

impl BinaryConstraint for AdjacentConstraint {
    fn vars(&self) -> (Tile, Tile) {
        (self.tile_a, self.tile_b)
    }

    fn valid(&self, col_a: usize, col_b: usize) -> bool {
        (col_a as isize - col_b as isize).abs() == self.distance as isize
    }
}

#[derive(Debug, Clone, Hash)]
pub struct NotAdjacentConstraint {
    pub tile_a: Tile,
    pub tile_b: Tile,
}

impl BinaryConstraint for NotAdjacentConstraint {
    fn vars(&self) -> (Tile, Tile) {
        (self.tile_a, self.tile_b)
    }

    fn valid(&self, col_a: usize, col_b: usize) -> bool {
        (col_a as isize - col_b as isize).abs() != 1
    }
}

#[derive(Debug, Clone, Hash)]
pub struct TwoApartNotMiddleConstraint {
    pub tile_a: Tile,
    pub tile_not_b: Tile,
    pub tile_c: Tile,
}

impl TernaryConstraint for TwoApartNotMiddleConstraint {
    fn vars(&self) -> Vec<Tile> {
        vec![self.tile_a, self.tile_not_b, self.tile_c]
    }

    fn valid(&self, values: &Vec<usize>) -> bool {
        let a = values[0] as isize;
        let b = values[1] as isize;
        let c = values[2] as isize;

        // a and c should be 2 apart
        if (a - c).abs() != 2 {
            return false;
        }

        // b shouldn't be between a and c
        let middle_col = (a + c) / 2;
        if b == middle_col {
            return false;
        }

        true
    }
}

#[derive(Debug, Clone, Hash)]
pub struct LessThanConstraint {
    pub tile_a: Tile,
    pub tile_b: Tile,
}

impl BinaryConstraint for LessThanConstraint {
    fn vars(&self) -> (Tile, Tile) {
        (self.tile_a, self.tile_b)
    }

    fn valid(&self, col_a: usize, col_b: usize) -> bool {
        col_a < col_b
    }
}

#[derive(Debug, Clone, Hash)]
pub struct OneMatchesEitherConstraint {
    pub tile_a: Tile,
    pub tile_b: Tile,
    pub tile_c: Tile,
}

impl TernaryConstraint for OneMatchesEitherConstraint {
    fn vars(&self) -> Vec<Tile> {
        vec![self.tile_a, self.tile_b, self.tile_c]
    }

    fn valid(&self, values: &Vec<usize>) -> bool {
        let a = values[0];
        let b = values[1];
        let c = values[2];

        if b == c {
            return false;
        }

        a == b || a == c
    }
}

#[derive(Default, Debug)]
pub struct ConstraintSet {
    pub unary_constraints: Vec<Box<dyn UnaryConstraint>>,
    pub binary_constraints: Vec<Box<dyn BinaryConstraint>>,
    pub ternary_constraints: Vec<Box<dyn TernaryConstraint>>,
}

pub trait ClueConstraint: std::fmt::Debug {
    /// Returns all potential solutions for a given clue
    fn potential_solutions(
        &self,
        board: &GameBoard,
        column: usize,
    ) -> Vec<Vec<(usize, TileAssertion)>>;

    fn constraints(&self, difficulty: Difficulty) -> ConstraintSet;
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

impl ClueConstraint for LeftOfHandler {
    fn potential_solutions(
        &self,
        board: &GameBoard,
        column: usize,
    ) -> Vec<Vec<(usize, TileAssertion)>> {
        let max_column = board.solution.n_variants - 1;
        let mut solutions = Vec::new();

        // Skip if we're at the last column - can't have a right tile
        if column >= max_column {
            return solutions;
        }

        // Check if the left tile can go in this column
        if !board.is_candidate_available(self.left_tile.row, column, self.left_tile.variant) {
            return solutions;
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

    fn constraints(&self, difficulty: Difficulty) -> ConstraintSet {
        let mut constraints = ConstraintSet::default();
        constraints.unary_constraints.push(Box::new(EdgeConstraint {
            tile: self.left_tile,
            difficulty,
            allow_left: true,
            allow_right: false,
        }));
        constraints.unary_constraints.push(Box::new(EdgeConstraint {
            tile: self.right_tile,
            difficulty,
            allow_left: false,
            allow_right: true,
        }));

        constraints
            .binary_constraints
            .push(Box::new(LessThanConstraint {
                tile_a: self.left_tile,
                tile_b: self.right_tile,
            }));
        constraints
    }
}

impl ClueConstraint for NotAdjacentHandler {
    fn potential_solutions(
        &self,
        board: &GameBoard,
        column: usize,
    ) -> Vec<Vec<(usize, TileAssertion)>> {
        let max_column = board.solution.n_variants - 1;

        // can the positive tile go here and the negative assertion work both ways?
        if !board.is_candidate_available(self.positive_tile.row, column, self.positive_tile.variant)
        {
            // positive tile can't go here
            return Vec::new();
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

    fn constraints(&self, _difficulty: Difficulty) -> ConstraintSet {
        let mut constraints = ConstraintSet::default();
        constraints
            .binary_constraints
            .push(Box::new(NotAdjacentConstraint {
                tile_a: self.positive_tile,
                tile_b: self.negative_tile,
            }));
        constraints
    }
}

impl ClueConstraint for AdjacentHandler {
    fn potential_solutions(
        &self,
        board: &GameBoard,
        column: usize,
    ) -> Vec<Vec<(usize, TileAssertion)>> {
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

    fn constraints(&self, difficulty: Difficulty) -> ConstraintSet {
        let mut constraints = ConstraintSet::default();
        let assertions = &self.clue.assertions;

        if assertions.iter().all(|ta| ta.assertion) {
            // all positive
            for i in 0..assertions.len() {
                for j in i + 1..assertions.len() {
                    constraints
                        .binary_constraints
                        .push(Box::new(AdjacentConstraint {
                            tile_a: self.clue.assertions[i].tile,
                            tile_b: self.clue.assertions[j].tile,
                            distance: j - i,
                        }));
                }
            }
            if assertions.len() == 3 {
                constraints.unary_constraints.push(Box::new(EdgeConstraint {
                    tile: assertions[1].tile,
                    difficulty,
                    allow_left: false,
                    allow_right: false,
                }));
            }
        } else if assertions.len() == 3 {
            // two apart, not middle
            let middle_tile = assertions[1].tile;
            let left_tile = assertions[0].tile;
            let right_tile = assertions[2].tile;
            constraints
                .ternary_constraints
                .push(Box::new(TwoApartNotMiddleConstraint {
                    tile_a: left_tile,
                    tile_not_b: middle_tile,
                    tile_c: right_tile,
                }));
        }
        constraints
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

impl ClueConstraint for AllInColumnHandler {
    fn potential_solutions(
        &self,
        board: &GameBoard,
        column: usize,
    ) -> Vec<Vec<(usize, TileAssertion)>> {
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

    fn constraints(&self, _difficulty: Difficulty) -> ConstraintSet {
        let mut constraints = ConstraintSet::default();
        for i in 0..self.assertions.len() {
            for j in i + 1..self.assertions.len() {
                let assertion_a = &self.assertions[i];
                let assertion_b = &self.assertions[j];

                if assertion_a.is_positive() && assertion_b.is_positive() {
                    constraints
                        .binary_constraints
                        .push(Box::new(InSameColumnConstraint {
                            tile_a: assertion_a.tile,
                            tile_b: assertion_b.tile,
                        }));
                } else {
                    constraints
                        .binary_constraints
                        .push(Box::new(NotInSameColumnConstraint {
                            tile_a: assertion_a.tile,
                            tile_b: assertion_b.tile,
                        }));
                }
            }
        }
        constraints
    }
}

pub fn create_clue_constraint(clue: &Clue) -> Box<dyn ClueConstraint> {
    match &clue.clue_type {
        ClueType::Horizontal(h_type) => match h_type {
            HorizontalClueType::TwoAdjacent => Box::new(AdjacentHandler::new(clue)),
            HorizontalClueType::ThreeAdjacent => Box::new(AdjacentHandler::new(clue)),
            HorizontalClueType::TwoApartNotMiddle => Box::new(AdjacentHandler::new(clue)),
            HorizontalClueType::NotAdjacent => Box::new(NotAdjacentHandler::new(clue)),
            HorizontalClueType::LeftOf => Box::new(LeftOfHandler::new(clue)),
        },
        ClueType::Vertical(v_type) => match v_type {
            VerticalClueType::OneMatchesEither => Box::new(OneMatchesEitherHandler::new(clue)),
            _ => Box::new(AllInColumnHandler::new(clue)),
        },
    }
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
    // TODO - would be nice to tag filtered potential solution sets that were reduced due to some of them creating invalid possibilities, as this is a bit of an advanced deduction mechanism.
    let is_valid = board_clone.is_valid_possibility();
    trace!(
        target: "solver",
        "Board after partial solution: {:?}",
        board_clone
    );
    trace!(target: "solver", "Is valid? {}", is_valid);
    is_valid
}

#[derive(Clone, Debug)]
struct OneMatchesEitherHandler {
    assertions: Vec<TileAssertion>,
}

impl OneMatchesEitherHandler {
    fn new(clue: &Clue) -> Self {
        Self {
            assertions: clue.assertions.clone(),
        }
    }
}

impl ClueConstraint for OneMatchesEitherHandler {
    fn potential_solutions(
        &self,
        _board: &GameBoard,
        _column: usize,
    ) -> Vec<Vec<(usize, TileAssertion)>> {
        // this ones... tricky. We have a solver for it.
        todo!()
    }

    fn constraints(&self, _difficulty: Difficulty) -> ConstraintSet {
        let mut constraints = ConstraintSet::default();
        constraints
            .ternary_constraints
            .push(Box::new(OneMatchesEitherConstraint {
                tile_a: self.assertions[0].tile,
                tile_b: self.assertions[1].tile,
                tile_c: self.assertions[2].tile,
            }));
        constraints
    }
}
