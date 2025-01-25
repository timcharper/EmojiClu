use super::Tile;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Deduction {
    pub tile: Tile,
    pub column: usize,
    pub is_positive: bool,
}

impl Deduction {
    #[cfg(test)]
    pub fn parse(input: &str) -> Option<Self> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        // Need at least 4 parts: "1a", "not"/"is", "col", "0"
        if parts.len() != 4 || parts[2] != "col" {
            return None;
        }

        // Parse tile from first part (e.g. "1a")
        if parts[0].len() < 2 {
            return None;
        }
        let row = parts[0][0..1].parse::<usize>().ok()?;
        let variant = parts[0].chars().nth(1)?;
        let tile = Tile::new(row, variant);

        // Parse column number
        let column = parts[3].parse::<usize>().ok()?;

        match parts[1] {
            "not" => Some(Deduction {
                tile,
                column,
                is_positive: false,
            }),
            "is" => Some(Deduction {
                tile,
                column,
                is_positive: true,
            }),
            _ => None,
        }
    }
}
