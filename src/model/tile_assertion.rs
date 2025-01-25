use super::Tile;

#[derive(Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Copy)]
pub struct TileAssertion {
    pub tile: Tile,
    pub assertion: bool, // true = positive assertion (tile exists), false = negative assertion (tile does not exist)
}

impl TileAssertion {
    pub fn to_string(&self) -> String {
        format!("{}{}", if self.assertion { "+" } else { "-" }, self.tile)
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
