use crate::model::ClueOrientation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
pub struct ClueAddress {
    pub orientation: ClueOrientation,
    pub index: usize,
}
