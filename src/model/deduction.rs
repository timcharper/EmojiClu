#[cfg(test)]
use super::tile::Tile;

use crate::model::tile_assertion::TileAssertion;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Deduction {
    pub column: usize,
    pub tile_assertion: TileAssertion,
}

impl Deduction {
    #[cfg(test)]
    pub fn parse(input: &str) -> Self {
        let parts: Vec<&str> = input.split_whitespace().collect();

        // Need at least 4 parts: "1a", "not"/"is", "col", "0"
        if parts.len() != 4 || parts[2] != "col" {
            panic!("Invalid deduction: {}", input);
        }

        // Parse tile from first part (e.g. "1a")
        if parts[0].len() < 2 {
            panic!("Invalid deduction: {}", input);
        }
        let row = parts[0][0..1]
            .parse::<usize>()
            .expect(&format!("Invalid row in {}", input));
        let variant = parts[0]
            .chars()
            .nth(1)
            .expect(&format!("Invalid variant in {}", input));
        let tile = Tile::new(row, variant);

        // Parse column number
        let column = parts[3]
            .parse::<usize>()
            .expect(&format!("Invalid column in {}", input));

        match parts[1] {
            "not" => Deduction {
                tile_assertion: TileAssertion {
                    tile,
                    assertion: false,
                },
                column,
            },
            "is" => Deduction {
                tile_assertion: TileAssertion {
                    tile,
                    assertion: true,
                },
                column,
            },
            _ => panic!("Invalid deduction: {}", input),
        }
    }

    pub fn is_positive(&self) -> bool {
        self.tile_assertion.assertion
    }
}
