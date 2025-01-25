use crate::model::tile::Tile;

#[derive(Clone, PartialEq, Eq)]
pub struct TileAssertion {
    pub tile: Tile,
    pub assertion: bool, // true = positive assertion (tile exists), false = negative assertion (tile does not exist)
}
