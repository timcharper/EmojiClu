// Base unit sizes
pub const CELL_SIZE: i32 = 64;
pub const SPACING_SMALL: i32 = 2;
pub const SPACING_LARGE: i32 = 10;

// Derived sizes
pub const FRAME_MARGIN: i32 = SPACING_SMALL;
pub const CELL_SPACING: i32 = SPACING_SMALL;

// Icon sizes
pub const SOLUTION_IMG_SIZE: i32 = 128;
pub const CANDIDATE_IMG_SIZE: i32 = SOLUTION_IMG_SIZE / 2;
pub const ICON_SIZE_SMALL: i32 = 20;

// Layout calculations
pub fn calc_clue_set_size(n_cells: i32) -> i32 {
    CELL_SIZE * n_cells + CELL_SPACING * (n_cells - 1) + FRAME_MARGIN * 2
}
