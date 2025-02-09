use super::Tile;

#[derive(
    Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Copy, serde::Serialize, serde::Deserialize,
)]
pub struct TileAssertion {
    pub tile: Tile,
    pub assertion: bool, // true = positive assertion (tile exists), false = negative assertion (tile does not exist)
}

impl TileAssertion {
    pub fn to_string(&self) -> String {
        format!("{}{}", if self.assertion { "+" } else { "-" }, self.tile)
    }

    /// Parse a tile assertion from a string of the form "+0a" or "-1b" or "?2c"
    /// The first character must be one of:
    /// - '+' for positive assertion
    /// - '-' for negative assertion
    /// - '?' for maybe assertion (used in one_matches_either)
    pub fn parse(s: &str) -> Self {
        let is_positive = match s.chars().next() {
            Some('+') => true,
            Some('-') => false,
            Some('?') => true, // maybe assertions are treated as positive
            _ => panic!("Invalid assertion prefix, must be +, -, or ?"),
        };

        let tile_str = &s[1..];
        let row = tile_str
            .chars()
            .next()
            .and_then(|c| c.to_digit(10))
            .map(|d| d as usize)
            .expect("Invalid row number");
        let variant = tile_str.chars().nth(1).expect("Missing variant character");

        Self {
            tile: Tile::new(row, variant),
            assertion: is_positive,
        }
    }

    pub(crate) fn is_positive(&self) -> bool {
        return self.assertion;
    }

    pub(crate) fn is_negative(&self) -> bool {
        return !self.assertion;
    }
}

impl std::fmt::Debug for TileAssertion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        // Test positive assertion
        let assertion = TileAssertion::parse("+0a");
        assert_eq!(assertion.tile.row, 0);
        assert_eq!(assertion.tile.variant, 'a');
        assert_eq!(assertion.assertion, true);

        // Test negative assertion
        let assertion = TileAssertion::parse("-1b");
        assert_eq!(assertion.tile.row, 1);
        assert_eq!(assertion.tile.variant, 'b');
        assert_eq!(assertion.assertion, false);

        // Test maybe assertion (used in one_matches_either)
        let assertion = TileAssertion::parse("?2c");
        assert_eq!(assertion.tile.row, 2);
        assert_eq!(assertion.tile.variant, 'c');
        assert_eq!(assertion.assertion, true);
    }

    #[test]
    #[should_panic(expected = "Invalid assertion prefix")]
    fn test_parse_invalid_prefix() {
        TileAssertion::parse("0a");
    }

    #[test]
    #[should_panic(expected = "Invalid row number")]
    fn test_parse_invalid_row() {
        TileAssertion::parse("+xa");
    }

    #[test]
    #[should_panic(expected = "Missing variant character")]
    fn test_parse_missing_variant() {
        TileAssertion::parse("+0");
    }
}
