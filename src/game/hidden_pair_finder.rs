use crate::model::{GameBoard, Tile};

#[derive(Debug, PartialEq, Eq)]
pub struct SubsetResult {
    pub columns: Vec<usize>,
    pub variants: Vec<char>,
}

/*

By representing a row as either bitset of variant to columns, or columns to variants, we can use the same algorithm to find hidden sets or naked sets.

- naked set: a set of columns N long can only contain N variants
- naked pair: a set of variants N long can only be in N columns

A row can have a naked set but not a naked pair, and vice versa, so we need to pivot and look both ways.

To visualize a bitset (we reverse the bits, representing the first column as bit 7, for consistent left-to-right reading order)

6|  cdefg |abc efgh|  cdef h|  cdefgh|  cd fgh|  c efgh|   defgh|ab d fgh|

a 01000001
b 01000001
c 11111100
d 10111011
e 11110110
f 11111111
g 11101111
h 01111111


Algorith more-or-less goes as follows (s/hidden pair/naked set/ since these are just pivoted.)

    * start with numbers with bits <= half
    * find intersection that grows the bits minimally
    * expand set. bits = set size? hidden pair
    * surpass half? no hidden pair. Abandon path. Mark initial variant as not part of hidden pair and exclude from future search.
    * repeat
    * any columns with more bits than half rows cannot be a part of a hidden pair.
*/
fn find_isolated_bit_sets(bit_sets: &Vec<u8>, n_bits: usize) -> Vec<(Vec<usize>, u8)> {
    let max_set_size = n_bits as u32 / 2;
    let mut possible_bit_set_indices = Vec::new();
    for col in 0..n_bits {
        if bit_sets[col].count_ones() <= max_set_size {
            possible_bit_set_indices.push(col);
        }
    }

    possible_bit_set_indices
        .iter()
        .flat_map(|col| {
            let mut current_variant_set = bit_sets[*col];
            let mut column_set_members = vec![*col];

            // ignore cells with only one variant
            if current_variant_set.count_ones() < 2 {
                return None;
            }

            while current_variant_set.count_ones() <= max_set_size
                && current_variant_set.count_ones() > column_set_members.len() as u32
            {
                // find the variant that expands the column set the least
                let other_columns = possible_bit_set_indices
                    .iter()
                    .filter(|v| !column_set_members.contains(v) && v > &col)
                    .collect::<Vec<_>>();

                let minimal_expansion = other_columns
                    .iter()
                    .map(|other_col| {
                        let other_column_set = bit_sets[**other_col];
                        let intersection = current_variant_set | other_column_set;
                        (other_col, intersection, intersection.count_ones())
                    })
                    .min_by_key(|(_, _, bits_count)| *bits_count);

                if let Some((other_col, intersection, _)) = minimal_expansion {
                    current_variant_set = intersection;
                    column_set_members.push(**other_col);
                } else {
                    // ran out of candidates, this route is a bust
                    return None;
                }
            }

            if current_variant_set.count_ones() <= max_set_size
                && column_set_members.len() == current_variant_set.count_ones() as usize
            {
                return Some((column_set_members, current_variant_set));
            } else {
                return None;
            }
        })
        .collect()
}

pub fn find_naked_pairs_in_row(row: usize, board: &GameBoard) -> Vec<SubsetResult> {
    /*

    variant: a, column_set: 11111101
    variant: b, column_set: 00000010
    variant: c, column_set: 11111101
    variant: d, column_set: 01010100
    variant: e, column_set: 01010100
    variant: f, column_set: 11111101
    variant: g, column_set: 10101000
    variant: h, column_set: 11111000

         */

    // create bitmask for column positions for each variant

    fn variants_to_bit_set(variants: &[char]) -> u8 {
        variants.iter().fold(0, |acc, variant| {
            acc | (1 << (Tile::variant_to_usize(*variant)))
        })
    }

    fn bit_set_to_variants(bit_set: u8) -> Vec<char> {
        (0..=7)
            .filter(|i| bit_set & (1 << i) != 0)
            .map(|i| Tile::usize_to_variant(i))
            .collect()
    }

    let mut column_variant_bit_sets: Vec<u8> = vec![0; board.solution.n_variants];
    for col in 0..board.solution.n_variants {
        let variants = board
            .solution
            .variants
            .iter()
            .filter(|v| board.is_candidate_available(row, col, **v))
            .cloned()
            .collect::<Vec<_>>();
        column_variant_bit_sets[col] = variants_to_bit_set(&variants);
    }

    let max_set_size = board.solution.n_variants as u32 / 2;

    let mut possible_columns = Vec::new();
    for col in 0..board.solution.n_variants {
        if column_variant_bit_sets[col].count_ones() <= max_set_size {
            possible_columns.push(col);
        }
    }

    let result = find_isolated_bit_sets(&column_variant_bit_sets, board.solution.n_variants);
    result
        .into_iter()
        .map(|(columns, bit_set)| SubsetResult {
            columns,
            variants: bit_set_to_variants(bit_set),
        })
        .collect()
}

pub fn find_hidden_pairs_in_row(row: usize, board: &GameBoard) -> Vec<SubsetResult> {
    fn columns_to_bit_set(columns: &[usize]) -> u8 {
        columns.iter().fold(0, |acc, col| acc | (1 << col))
    }

    fn bit_set_to_columns(bit_set: u8, n_cols: usize) -> Vec<usize> {
        (0..n_cols).filter(|i| bit_set & (1 << i) != 0).collect()
    }

    // create bitmask for column positions for each variant
    let mut variant_column_sets: Vec<u8> = vec![0; board.solution.n_variants];
    for variant in board.solution.variants.iter() {
        let variant_index = Tile::variant_to_usize(*variant);

        let columns = (0..board.solution.n_variants)
            .filter(|col| board.is_candidate_available(row, *col, *variant))
            .collect::<Vec<_>>();

        let bit_set = columns_to_bit_set(&columns);
        variant_column_sets[variant_index] = bit_set;
    }

    let result = find_isolated_bit_sets(&variant_column_sets, board.solution.n_variants);
    result
        .into_iter()
        .map(|(variants, bit_set)| SubsetResult {
            variants: variants
                .iter()
                .map(|v| Tile::usize_to_variant(*v))
                .collect(),
            columns: bit_set_to_columns(bit_set, board.solution.n_variants),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use test_context::test_context;

    use super::*;
    use crate::game::tests::create_test_solution;
    use crate::game::tests::UsingLogger;

    #[test_context(UsingLogger)]
    #[test]
    fn test_deduce_obvious_sets(_: &mut UsingLogger) {
        // there are hidden pairs in row 2 (afh) and row 4 (adg).
        let input = "\
0|  c   g |ab d    |  c   g |ab d f h|<E>     | b d f h|ab d f  |a  d f h|
";

        let board = GameBoard::parse(input, create_test_solution(1, 8));
        println!("Board: {:?}", board);

        let naked_pairs = find_naked_pairs_in_row(0, &board);
        assert_eq!(naked_pairs.len(), 1);
        assert_eq!(naked_pairs[0].columns, vec![0, 2]);
        assert_eq!(naked_pairs[0].variants, vec!['c', 'g']);

        let hidden_pairs = find_hidden_pairs_in_row(0, &board);
        assert_eq!(hidden_pairs.len(), 1);
        assert_eq!(hidden_pairs[0].variants, vec!['c', 'g']);
        assert_eq!(hidden_pairs[0].columns, vec![0, 2]);
    }

    #[test_context(UsingLogger)]
    #[test]
    fn test_deduce_hidden_pairs_large_board(_: &mut UsingLogger) {
        // there are hidden pairs in row 2 (afh) and row 4 (adg).
        let input = "\
0|  c   g |ab d    |  c   g |ab d f h|<E>     | b d f h|ab d f  |a  d f h|
----------------------------------------------------------------
1|ab  efg | b  e g |ab  efgh| b  e g |ab  efgh| bc e g |<D>     | bc e g |
----------------------------------------------------------------
2|a c  fgh|a cdef h|a c  fgh|a cdef h|a c  fgh|a cdef  |<B>     |a c  f  |
----------------------------------------------------------------
3| b de gh| bcd  gh| bcde gh| bcd  gh| bcdefgh|abcd  gh| bcd fgh|ab d  g |
----------------------------------------------------------------
4|  cd  gh|a  d  g |  cde gh|a  d  g | bcde gh|a  d    | b d  g |<F>     |
----------------------------------------------------------------
5|ab   f h| b d fg |ab   f h|<C>     |ab  efgh| b d fg | b  ef  | b   fg |
----------------------------------------------------------------
6|abcd f  |ab def h|abcd f h|abcdef h|abcd f  |abcd fgh|abcd f h| bc  fgh|
----------------------------------------------------------------
7|  cdefg | bcd  gh| bcdefg | bcd  gh| bcdefg |<A>     | bcdefg | bcd fg |
";

        let board = GameBoard::parse(input, create_test_solution(8, 8));
        println!("Board: {:?}", board);

        /*
        === Naked pairs in row 0: [SubsetResult { columns: [0, 2], variants: ['c', 'g'] }]
        === Naked pairs in row 1: [SubsetResult { columns: [1, 3, 5, 7], variants: ['b', 'c', 'e', 'g'] }]
        === Naked pairs in row 2: []
        === Naked pairs in row 3: []
        === Naked pairs in row 4: [SubsetResult { columns: [1, 3, 5], variants: ['a', 'd', 'g'] }]
        === Naked pairs in row 5: []
        === Naked pairs in row 6: []
        === Naked pairs in row 7: []
        === Hidden pairs in row 0: [SubsetResult { columns: [0, 2], variants: ['c', 'g'] }]
        === Hidden pairs in row 1: [SubsetResult { columns: [0, 2, 4], variants: ['a', 'f', 'h'] }]
        === Hidden pairs in row 2: []
        === Hidden pairs in row 3: []
        === Hidden pairs in row 4: [SubsetResult { columns: [0, 2, 4, 6], variants: ['b', 'e', 'c', 'h'] }, SubsetResult { columns: [0, 2, 4], variants: ['c', 'e', 'h'] }]
        === Hidden pairs in row 5: []
        === Hidden pairs in row 6: []
        === Hidden pairs in row 7: []

                 */

        for row_without_hidden_sets in vec![2, 3, 5, 6, 7] {
            let naked_pairs = find_naked_pairs_in_row(row_without_hidden_sets, &board);
            assert!(
                naked_pairs.is_empty(),
                "Row {} has naked pairs: {:?}",
                row_without_hidden_sets,
                naked_pairs
            );

            let hidden_pairs = find_hidden_pairs_in_row(row_without_hidden_sets, &board);
            assert!(
                hidden_pairs.is_empty(),
                "Row {} has hidden pairs: {:?}",
                row_without_hidden_sets,
                hidden_pairs
            );
        }

        let row_0_naked_pairs = find_naked_pairs_in_row(0, &board);
        assert_eq!(
            row_0_naked_pairs[0],
            SubsetResult {
                columns: vec![0, 2],
                variants: vec!['c', 'g'],
            }
        );

        let row_0_hidden_pairs = find_hidden_pairs_in_row(0, &board);
        assert_eq!(row_0_hidden_pairs.len(), 1);
        assert_eq!(
            row_0_hidden_pairs[0],
            SubsetResult {
                columns: vec![0, 2],
                variants: vec!['c', 'g'],
            }
        );

        let row_1_naked_pairs = find_naked_pairs_in_row(1, &board);
        assert_eq!(row_1_naked_pairs.len(), 1);
        assert_eq!(
            row_1_naked_pairs[0],
            SubsetResult {
                columns: vec![1, 3, 5, 7],
                variants: vec!['b', 'c', 'e', 'g'],
            }
        );

        let row_1_hidden_pairs = find_hidden_pairs_in_row(1, &board);
        assert_eq!(row_1_hidden_pairs.len(), 1);
        assert_eq!(
            row_1_hidden_pairs[0],
            SubsetResult {
                variants: vec!['a', 'f', 'h'],
                columns: vec![0, 2, 4],
            }
        );

        let row_4_naked_pairs = find_naked_pairs_in_row(4, &board);
        assert_eq!(row_4_naked_pairs.len(), 1);
        assert_eq!(
            row_4_naked_pairs[0],
            SubsetResult {
                columns: vec![1, 3, 5],
                variants: vec!['a', 'd', 'g'],
            }
        );

        let row_4_hidden_pairs = find_hidden_pairs_in_row(4, &board);
        assert_eq!(row_4_hidden_pairs.len(), 2);
        assert_eq!(
            row_4_hidden_pairs[0],
            SubsetResult {
                variants: vec!['b', 'e', 'c', 'h'],
                columns: vec![0, 2, 4, 6],
            }
        );
        assert_eq!(
            row_4_hidden_pairs[1],
            SubsetResult {
                variants: vec!['c', 'e', 'h'],
                columns: vec![0, 2, 4],
            }
        );

        // for row in 0..board.solution.n_rows {
        //     let naked_pairs = find_naked_pairs_in_row_improved(row, &board);
        //     println!("=== Naked pairs in row {}: {:?}", row, naked_pairs);
        // }

        // for row in 0..board.solution.n_rows {
        //     let hidden_pairs = find_hidden_pairs_in_row_improved(row, &board);
        //     println!("=== Hidden pairs in row {}: {:?}", row, hidden_pairs);
        // }
    }
}
