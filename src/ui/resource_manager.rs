use std::{cell::RefCell, rc::Rc};

use fixed::types::I8F8;
use log::trace;

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventHandler},
    model::LayoutManagerEvent,
};

use super::{audio_set::AudioSet, image_set::ImageSet};

pub struct ResourceManager {
    image_set: Rc<ImageSet>,
    audio_set: Rc<AudioSet>,
    layout_manager_event_emitter: EventEmitter<LayoutManagerEvent>,
}

impl Destroyable for ResourceManager {
    fn destroy(&mut self) {
        // Subscription cleanup handled automatically by subscribe_component
    }
}

impl EventHandler<LayoutManagerEvent> for ResourceManager {
    fn handle_event(&mut self, event: &LayoutManagerEvent) {
        match event {
            LayoutManagerEvent::OptimizeImages {
                candidate_tile_size,
                solution_tile_size,
                scale_factor,
            } => {
                self.optimize_images(*candidate_tile_size, *solution_tile_size, *scale_factor);
            }
            _ => (),
        }
    }
}

impl ResourceManager {
    pub fn new(
        layout_manager_event_emitter: EventEmitter<LayoutManagerEvent>,
    ) -> Rc<RefCell<Self>> {
        let image_set = Rc::new(ImageSet::new());
        let audio_set = Rc::new(AudioSet::new());
        let manager = Rc::new(RefCell::new(Self {
            image_set: image_set.clone(),
            audio_set: audio_set.clone(),
            layout_manager_event_emitter,
        }));

        manager
    }

    pub fn get_image_set(&self) -> Rc<ImageSet> {
        self.image_set.clone()
    }
    pub fn get_audio_set(&self) -> Rc<AudioSet> {
        self.audio_set.clone()
    }

    fn optimize_images(
        &mut self,
        candidate_tile_size: i32,
        solution_tile_size: i32,
        scale_factor: I8F8,
    ) {
        trace!(target: "resource_manager", "Optimizing images");
        let new_image_set = self.image_set.optimized_image_set(
            candidate_tile_size,
            solution_tile_size,
            scale_factor,
        );
        self.image_set = Rc::new(new_image_set);
        trace!(target: "resource_manager", "Emitting images optimized event");
        self.layout_manager_event_emitter
            .emit(LayoutManagerEvent::ImagesOptimized(self.image_set.clone()));
    }
}
