use gdk_pixbuf::{Colorspace, InterpType, Pixbuf};
use gtk4::MediaFile;
use rand::Rng;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

use crate::model::Tile;

// TODO - use value from LayoutManager
const SOLUTION_IMG_SIZE: i32 = 128;
pub struct ResourceSet {
    icons: HashMap<(i32, i32), Rc<Pixbuf>>,
    negative_assertion: Rc<Pixbuf>,
    left_of: Rc<Pixbuf>,
    maybe_assertion: Rc<Pixbuf>,
    lose_sounds: Vec<Rc<MediaFile>>,
    win_sounds: Vec<Rc<MediaFile>>,
}

impl ResourceSet {
    pub fn new() -> Self {
        let mut lose_sounds = Vec::new();
        for n in 1..=2 {
            let resource_path = format!("/org/mindhunt/assets/sounds/lose-{}.mp3", n);
            let media = MediaFile::for_resource(&resource_path);
            lose_sounds.push(Rc::new(media));
        }

        let mut win_sounds = Vec::new();
        for n in 1..=3 {
            let resource_path = format!("/org/mindhunt/assets/sounds/win-{}.mp3", n);
            let media = MediaFile::for_resource(&resource_path);
            win_sounds.push(Rc::new(media));
        }
        let empty_pixbuf = Rc::new(
            Pixbuf::new(Colorspace::Rgb, false, 8, 8, 8).expect("Failed to create empty pixbuf"),
        );

        let mut set = Self {
            icons: HashMap::new(),
            negative_assertion: empty_pixbuf.clone(),
            left_of: empty_pixbuf.clone(),
            maybe_assertion: empty_pixbuf.clone(),
            lose_sounds,
            win_sounds,
        };
        set.load_tile_icons();
        set
    }

    fn load_tile_icons(&mut self) {
        // Load all icon variants (8x8 grid of icons)
        for row in 0..8 {
            for col in 0..8 {
                let resource_path = format!("/org/mindhunt/assets/icons/{}/{}.png", row, col);
                let original_image = Pixbuf::from_resource(&resource_path)
                    .expect(&format!("Failed to load icon {} {}", row, col));
                let scaled_image = self.rescale_icon(&original_image);
                self.icons.insert((row, col), Rc::new(scaled_image));
            }
        }

        // Load special icons
        let negative_assertion =
            Pixbuf::from_resource("/org/mindhunt/assets/icons/negative-assertion.png")
                .expect("Failed to load negative assertion icon");
        let scaled_negative_assertion = self.rescale_icon(&negative_assertion);
        self.negative_assertion = Rc::new(scaled_negative_assertion);

        let left_of = Pixbuf::from_resource("/org/mindhunt/assets/icons/left-of.png")
            .expect("Failed to load left-of icon");
        let scaled_left_of = self.rescale_icon(&left_of);
        self.left_of = Rc::new(scaled_left_of);

        let maybe_assertion =
            Pixbuf::from_resource("/org/mindhunt/assets/icons/maybe-assertion.png")
                .expect("Failed to load maybe assertion icon");
        let scaled_maybe_assertion = self.rescale_icon(&maybe_assertion);
        self.maybe_assertion = Rc::new(scaled_maybe_assertion);
    }

    fn rescale_icon(&self, pixbuf: &Pixbuf) -> Pixbuf {
        let scaled_image =
            pixbuf.scale_simple(SOLUTION_IMG_SIZE, SOLUTION_IMG_SIZE, InterpType::Bilinear);
        scaled_image.expect("Failed to scale icon")
    }

    pub fn get_icon(&self, row: i32, col: i32) -> Option<Rc<Pixbuf>> {
        self.icons.get(&(row, col)).cloned()
    }

    pub fn get_tile_icon(&self, tile: &Tile) -> Option<Rc<Pixbuf>> {
        self.get_icon(tile.row as i32, tile.variant as i32 - 'a' as i32)
    }

    pub fn get_negative_assertion(&self) -> Rc<Pixbuf> {
        Rc::clone(&self.negative_assertion)
    }

    pub fn get_left_of(&self) -> Rc<Pixbuf> {
        Rc::clone(&self.left_of)
    }

    pub fn get_maybe_assertion(&self) -> Rc<Pixbuf> {
        Rc::clone(&self.maybe_assertion)
    }

    pub fn random_lose_sound(&self) -> Rc<MediaFile> {
        let index = rand::rng().random_range(0..self.lose_sounds.len());
        Rc::clone(&self.lose_sounds[index])
    }

    pub fn random_win_sound(&self) -> Rc<MediaFile> {
        let index = rand::rng().random_range(0..self.win_sounds.len());
        Rc::clone(&self.win_sounds[index])
    }
}

impl Debug for ResourceSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResourceSet")
    }
}
