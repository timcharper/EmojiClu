#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tile {
    pub row: usize,    // 0-5 (zero-based row index)
    pub variant: char, // 'a'-'f'
}

impl Tile {
    pub fn new(row: usize, variant: char) -> Self {
        Self { row, variant }
    }

    #[cfg(test)]
    /// Parse a tile from a string of the form "0a" or "1b" etc.
    pub fn parse(s: &str) -> Self {
        let row = s.chars().next().unwrap().to_digit(10).unwrap() as usize;
        let variant = s.chars().nth(1).unwrap();
        Self { row, variant }
    }
}

impl std::fmt::Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.row, self.variant)
    }
}

impl std::fmt::Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.row, self.variant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let tile = Tile::parse("0a");
        assert_eq!(tile.row, 0);
        assert_eq!(tile.variant, 'a');

        let tile = Tile::parse("1b");
        assert_eq!(tile.row, 1);
        assert_eq!(tile.variant, 'b');

        let tile = Tile::parse("5f");
        assert_eq!(tile.row, 5);
        assert_eq!(tile.variant, 'f');
    }
}
