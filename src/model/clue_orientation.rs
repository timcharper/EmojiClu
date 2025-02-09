use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
pub enum ClueOrientation {
    Horizontal,
    Vertical,
}

impl ClueOrientation {
    pub fn to_string(&self) -> &str {
        match self {
            ClueOrientation::Horizontal => "Horizontal",
            ClueOrientation::Vertical => "Vertical",
        }
    }

    pub fn invert(&self) -> ClueOrientation {
        match self {
            ClueOrientation::Horizontal => ClueOrientation::Vertical,
            ClueOrientation::Vertical => ClueOrientation::Horizontal,
        }
    }
}
impl Display for ClueOrientation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
