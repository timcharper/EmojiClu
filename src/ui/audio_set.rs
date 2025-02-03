use gtk4::MediaFile;
use rand::Rng;
use std::fmt::Debug;
use std::rc::Rc;

// TODO - use value from LayoutManager
pub struct AudioSet {
    lose_sounds: Vec<Rc<MediaFile>>,
    win_sounds: Vec<Rc<MediaFile>>,
}

impl AudioSet {
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
        let set = Self {
            lose_sounds,
            win_sounds,
        };
        set
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

impl Debug for AudioSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AudioSet")
    }
}
