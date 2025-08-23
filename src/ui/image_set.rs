use fixed::types::I8F8;
use gdk_pixbuf::{InterpType, Pixbuf};
use gtk4::gdk::Texture;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

use crate::model::Tile;

// TODO - use value from LayoutManager
const SOLUTION_IMG_SIZE: i32 = 128;
const CANDIDATE_IMG_SIZE: i32 = 64;

#[derive(Clone)]
pub struct OriginalIcons {
    icons: HashMap<(i32, i32), Rc<Pixbuf>>,
    negative_assertion: Rc<Pixbuf>,
    left_of: Rc<Pixbuf>,
    maybe_assertion_top: Rc<Pixbuf>,
    maybe_assertion_bottom: Rc<Pixbuf>,
    not_next_to_assertion_left: Rc<Pixbuf>,
    not_next_to_assertion_right: Rc<Pixbuf>,
}

pub struct ScaledIcons {
    solution_scale_icons: HashMap<(i32, i32), Rc<Texture>>,
    candidate_scale_icons: HashMap<(i32, i32), Rc<Texture>>,
    scaled_negative_assertion: Rc<Texture>,
    scaled_left_of: Rc<Texture>,
    scaled_maybe_assertion_top: Rc<Texture>,
    scaled_maybe_assertion_bottom: Rc<Texture>,
    scaled_not_next_to_assertion_left: Rc<Texture>,
    scaled_not_next_to_assertion_right: Rc<Texture>,
}

pub struct ImageSet {
    original_icons: OriginalIcons,
    scaled_icons: ScaledIcons,
}

impl ImageSet {
    pub fn new() -> Self {
        let mut original_icons: HashMap<(i32, i32), Rc<Pixbuf>> = HashMap::new();

        // Load all icon variants (8x8 grid of icons)
        for row in 0..8 {
            for col in 0..8 {
                let resource_path = format!("/org/emojiclu/assets/icons/{}/{}.png", row, col);
                let original_image = Pixbuf::from_resource(&resource_path)
                    .expect(&format!("Failed to load icon {} {}", row, col));
                original_icons.insert((row, col), Rc::new(original_image));
            }
        }

        // Load special icons
        let negative_assertion = Rc::new(
            Pixbuf::from_resource("/org/emojiclu/assets/icons/negative-assertion.png")
                .expect("Failed to load negative assertion icon"),
        );

        let not_next_to_assertion_left = Rc::new(
            Pixbuf::from_resource("/org/emojiclu/assets/icons/not-next-to-assertion-left.png")
                .expect("Failed to load not next to assertion left icon"),
        );

        let not_next_to_assertion_right = Rc::new(
            Pixbuf::from_resource("/org/emojiclu/assets/icons/not-next-to-assertion-right.png")
                .expect("Failed to load not next to assertion right icon"),
        );

        let left_of = Rc::new(
            Pixbuf::from_resource("/org/emojiclu/assets/icons/left-of.png")
                .expect("Failed to load left-of icon"),
        );

        let maybe_assertion_top = Rc::new(
            Pixbuf::from_resource("/org/emojiclu/assets/icons/maybe-assertion-top.png")
                .expect("Failed to load maybe assertion top icon"),
        );

        let maybe_assertion_bottom = Rc::new(
            Pixbuf::from_resource("/org/emojiclu/assets/icons/maybe-assertion-bottom.png")
                .expect("Failed to load maybe assertion bottom icon"),
        );

        let original_icons = OriginalIcons {
            icons: original_icons,
            negative_assertion,
            left_of,
            maybe_assertion_top,
            maybe_assertion_bottom,
            not_next_to_assertion_left,
            not_next_to_assertion_right,
        };

        let scaled_icons = ImageSet::rescale_icons(
            &original_icons,
            CANDIDATE_IMG_SIZE,
            SOLUTION_IMG_SIZE,
            I8F8::from_num(1),
        );

        Self {
            original_icons,
            scaled_icons,
        }
    }

    fn rescale_icons(
        original_icons: &OriginalIcons,
        unscaled_candidate_tile_size: i32,
        unscaled_solution_tile_size: i32,
        scale_factor: I8F8,
    ) -> ScaledIcons {
        let mut solution_scale_icons: HashMap<(i32, i32), Rc<Texture>> = HashMap::new();
        let mut candidate_scale_icons: HashMap<(i32, i32), Rc<Texture>> = HashMap::new();

        let scaled_candidate_tile_size =
            (unscaled_candidate_tile_size as f32 * scale_factor.to_num::<f32>()) as i32;
        let scaled_solution_tile_size =
            (unscaled_solution_tile_size as f32 * scale_factor.to_num::<f32>()) as i32;

        // Load all icon variants (8x8 grid of icons)
        for row in 0..8 {
            for col in 0..8 {
                let original_icon = original_icons.icons.get(&(row, col)).unwrap();
                let candidate_size = ImageSet::rescale_icon_from_pixbuf(
                    original_icon,
                    scaled_candidate_tile_size as u32,
                );
                let solution_size = ImageSet::rescale_icon_from_pixbuf(
                    original_icon,
                    scaled_solution_tile_size as u32,
                );
                candidate_scale_icons.insert((row, col), Rc::new(candidate_size));
                solution_scale_icons.insert((row, col), Rc::new(solution_size));
            }
        }

        // Load special icons
        let scaled_negative_assertion = ImageSet::rescale_icon_from_pixbuf(
            &original_icons.negative_assertion,
            scaled_candidate_tile_size as u32,
        );

        let scaled_left_of = ImageSet::rescale_icon_from_pixbuf(
            &original_icons.left_of,
            scaled_candidate_tile_size as u32,
        );

        let scaled_maybe_assertion_top = ImageSet::rescale_icon_from_pixbuf(
            &original_icons.maybe_assertion_top,
            scaled_solution_tile_size as u32,
        );

        let scaled_maybe_assertion_bottom = ImageSet::rescale_icon_from_pixbuf(
            &original_icons.maybe_assertion_bottom,
            scaled_solution_tile_size as u32,
        );

        let scaled_not_next_to_assertion_left = ImageSet::rescale_icon_from_pixbuf(
            &original_icons.not_next_to_assertion_left,
            scaled_solution_tile_size as u32,
        );

        let scaled_not_next_to_assertion_right = ImageSet::rescale_icon_from_pixbuf(
            &original_icons.not_next_to_assertion_right,
            scaled_solution_tile_size as u32,
        );

        let scaled_icons = ScaledIcons {
            solution_scale_icons,
            candidate_scale_icons,
            scaled_negative_assertion: Rc::new(scaled_negative_assertion),
            scaled_left_of: Rc::new(scaled_left_of),
            scaled_maybe_assertion_top: Rc::new(scaled_maybe_assertion_top),
            scaled_maybe_assertion_bottom: Rc::new(scaled_maybe_assertion_bottom),
            scaled_not_next_to_assertion_left: Rc::new(scaled_not_next_to_assertion_left),
            scaled_not_next_to_assertion_right: Rc::new(scaled_not_next_to_assertion_right),
        };

        scaled_icons
    }

    pub fn optimized_image_set(
        &self,
        candidate_tile_size: i32,
        solution_tile_size: i32,
        scale_factor: I8F8,
    ) -> ImageSet {
        let scaled_icons = ImageSet::rescale_icons(
            &self.original_icons,
            candidate_tile_size,
            solution_tile_size,
            scale_factor,
        );
        let image_set = ImageSet {
            original_icons: self.original_icons.clone(),
            scaled_icons,
        };

        image_set
    }

    fn rescale_icon_from_pixbuf(pixbuf: &Pixbuf, size: u32) -> Texture {
        let scaled_image = pixbuf
            .scale_simple(size as i32, size as i32, InterpType::Bilinear)
            .expect("Failed to scale icon");
        Texture::for_pixbuf(&scaled_image).into()
    }

    pub fn get_candidate_icon(&self, tile: &Tile) -> Option<Rc<Texture>> {
        self.scaled_icons
            .candidate_scale_icons
            .get(&(tile.row as i32, tile.variant as i32 - 'a' as i32))
            .cloned()
    }

    pub fn get_solution_icon(&self, tile: &Tile) -> Option<Rc<Texture>> {
        self.scaled_icons
            .solution_scale_icons
            .get(&(tile.row as i32, tile.variant as i32 - 'a' as i32))
            .cloned()
    }

    pub fn get_negative_assertion(&self) -> Rc<Texture> {
        Rc::clone(&self.scaled_icons.scaled_negative_assertion)
    }

    pub fn get_left_of(&self) -> Rc<Texture> {
        Rc::clone(&self.scaled_icons.scaled_left_of)
    }

    pub fn get_maybe_assertion_top(&self) -> Rc<Texture> {
        Rc::clone(&self.scaled_icons.scaled_maybe_assertion_top)
    }
    pub fn get_maybe_assertion_bottom(&self) -> Rc<Texture> {
        Rc::clone(&self.scaled_icons.scaled_maybe_assertion_bottom)
    }

    pub fn get_not_next_to_assertion_left(&self) -> Rc<Texture> {
        Rc::clone(&self.scaled_icons.scaled_not_next_to_assertion_left)
    }
    pub fn get_not_next_to_assertion_right(&self) -> Rc<Texture> {
        Rc::clone(&self.scaled_icons.scaled_not_next_to_assertion_right)
    }
}

impl Debug for ImageSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResourceSet")
    }
}
