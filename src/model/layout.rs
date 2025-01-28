#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct Dimensions {
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct GridCellSizing {
    pub dimensions: Dimensions,
    pub solution_image: Dimensions,
    pub candidate_image: Dimensions,
    pub padding: i32, // do we really need this? we have marging around
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct GridSizing {
    pub column_spacing: i32,
    pub row_spacing: i32,
    pub outer_padding: i32, // style.css defines this
    pub cell: GridCellSizing,
    pub total_dimensions: Dimensions,
    pub n_variants: usize,
    pub n_rows: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct CluesSizing {
    pub clue_tile_size: Dimensions,
    pub horizontal_clue_panel_width: i32,
    pub vertical_clue_panel_height: i32,
    pub clue_annotation_size: Dimensions,
    pub horizontal_margin: i32,
    pub vertical_margin: i32,
    pub horizontal_clue_column_spacing: i32,
    pub vertical_clue_group_spacer: i32, // space between clues, not the tiles themselves
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct LayoutConfiguration {
    pub grid: GridSizing,
    pub clues: CluesSizing,
}
