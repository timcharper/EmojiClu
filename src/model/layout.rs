#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct Dimensions {
    pub width: i32,
    pub height: i32,
}

impl Dimensions {
    pub fn scale_by(&self, scale: f32) -> Dimensions {
        Dimensions {
            width: (self.width as f32 * scale) as i32,
            height: (self.height as f32 * scale) as i32,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct GridCellSizing {
    pub dimensions: Dimensions,
    pub solution_image: Dimensions,
    pub candidate_image: Dimensions,
    pub candidate_spacing: i32,
    pub candidate_rows: i32,
    pub candidate_columns: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct GridSizing {
    pub column_spacing: i32,
    pub row_spacing: i32,
    pub outer_margin: i32,
    pub cell: GridCellSizing,
    pub total_dimensions: Dimensions,
    pub n_variants: i32,
    pub n_rows: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct HorizontalCluePanelSizing {
    pub total_clues_dimensions: Dimensions, // total dimensions of the entire panel
    pub row_spacing: i32,
    pub column_spacing: i32,
    pub left_margin: i32,
    pub clue_dimensions: Dimensions, // dimensions of each clue
    pub n_rows: i32,
    pub n_columns: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct VerticalCluePanelSizing {
    pub total_clues_height: i32,
    pub margin_top: i32,
    pub column_spacing: i32,
    pub group_spacing: i32, // space between clues, not the tiles themselves
    pub clue_dimensions: Dimensions, // dimensions of each clue
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct CluesSizing {
    pub clue_tile_size: Dimensions,
    pub horizontal_clue_panel: HorizontalCluePanelSizing,
    pub vertical_clue_panel: VerticalCluePanelSizing,
    pub clue_annotation_size: Dimensions,
    pub clue_padding: i32, // padding between clue and grid cell
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct LayoutConfiguration {
    pub grid: GridSizing,
    pub clues: CluesSizing,
}
