use fixed::types::I8F8;

use crate::ui::ImageSet;
use std::rc::Rc;

use super::LayoutConfiguration;

/// Events that are not specific to any one component of the game.
#[derive(Debug)]
pub enum LayoutManagerEvent {
    DimensionsChanged(Rc<ImageSet>),
    LayoutChanged(LayoutConfiguration),
    OptimizeImages {
        candidate_tile_size: i32,
        solution_tile_size: i32,
        scale_factor: I8F8,
    },
    ImagesOptimized(Rc<ImageSet>),
}
