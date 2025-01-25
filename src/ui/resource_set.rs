use gtk::gdk_pixbuf::Pixbuf;
use gtk::MediaFile;
use rand::Rng;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

use crate::model::Tile;

use super::layout::SOLUTION_IMG_SIZE;
pub struct ResourceSet {
    icons: HashMap<(i32, i32), Rc<Pixbuf>>,
    negative_assertion: Rc<Pixbuf>,
    triple_dot: Rc<Pixbuf>,
    maybe_assertion: Rc<Pixbuf>,
    lose_sounds: Vec<Rc<MediaFile>>,
    win_sounds: Vec<Rc<MediaFile>>,
}

impl ResourceSet {
    pub fn new() -> Self {
        let mut icons = HashMap::new();

        // Load all icon variants (8x8 grid of icons)
        for row in 0..8 {
            for col in 0..8 {
                let resource_path = format!("/org/gwatson/assets/icons/{}/{}.png", row, col);
                let original_image = Pixbuf::from_resource(&resource_path);
                let scaled_image = original_image.ok().and_then(|pixbuf| {
                    pixbuf.scale_simple(
                        SOLUTION_IMG_SIZE,
                        SOLUTION_IMG_SIZE,
                        gtk::gdk_pixbuf::InterpType::Bilinear,
                    )
                });
                if let Some(pixbuf) = scaled_image {
                    icons.insert((row, col), Rc::new(pixbuf));
                }
            }
        }

        // Load special icons
        let negative_assertion = Rc::new(
            Pixbuf::from_resource("/org/gwatson/assets/icons/negative-assertion.png")
                .expect("Failed to load negative assertion icon"),
        );

        let triple_dot = Rc::new(
            Pixbuf::from_resource("/org/gwatson/assets/icons/triple-dot.png")
                .expect("Failed to load triple dot icon"),
        );

        let maybe_assertion = Rc::new(
            Pixbuf::from_resource("/org/gwatson/assets/icons/maybe-assertion.png")
                .expect("Failed to load maybe assertion icon"),
        );

        let mut lose_sounds = Vec::new();
        for n in 1..=2 {
            let resource_path = format!("/org/gwatson/assets/sounds/lose-{}.mp3", n);
            let media = MediaFile::for_resource(&resource_path);
            lose_sounds.push(Rc::new(media));
        }

        let mut win_sounds = Vec::new();
        for n in 1..=3 {
            let resource_path = format!("/org/gwatson/assets/sounds/win-{}.mp3", n);
            let media = MediaFile::for_resource(&resource_path);
            win_sounds.push(Rc::new(media));
        }

        Self {
            icons,
            negative_assertion,
            triple_dot,
            maybe_assertion,
            lose_sounds,
            win_sounds,
        }
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

    pub fn get_triple_dot(&self) -> Rc<Pixbuf> {
        Rc::clone(&self.triple_dot)
    }

    pub fn get_maybe_assertion(&self) -> Rc<Pixbuf> {
        Rc::clone(&self.maybe_assertion)
    }

    pub fn random_lose_sound(&self) -> Rc<MediaFile> {
        let index = rand::thread_rng().gen_range(0..self.lose_sounds.len());
        Rc::clone(&self.lose_sounds[index])
    }

    pub fn random_win_sound(&self) -> Rc<MediaFile> {
        let index = rand::thread_rng().gen_range(0..self.win_sounds.len());
        Rc::clone(&self.win_sounds[index])
    }
}

impl Debug for ResourceSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResourceSet")
    }
}
