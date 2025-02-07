use std::{cell::RefCell, rc::Rc};

use log::trace;

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventObserver, Unsubscriber},
    model::GlobalEvent,
};

use super::{audio_set::AudioSet, image_set::ImageSet};

pub struct ResourceManager {
    image_set: Rc<ImageSet>,
    audio_set: Rc<AudioSet>,
    global_event_subscription: Option<Unsubscriber<GlobalEvent>>,
    global_event_emitter: EventEmitter<GlobalEvent>,
}

impl Destroyable for ResourceManager {
    fn destroy(&mut self) {
        if let Some(subscription) = self.global_event_subscription.take() {
            subscription.unsubscribe();
        }
    }
}

impl ResourceManager {
    pub fn new(
        global_event_observer: EventObserver<GlobalEvent>,
        global_event_emitter: EventEmitter<GlobalEvent>,
    ) -> Rc<RefCell<Self>> {
        let image_set = Rc::new(ImageSet::new());
        let audio_set = Rc::new(AudioSet::new());
        let manager = Rc::new(RefCell::new(Self {
            image_set: image_set.clone(),
            audio_set: audio_set.clone(),
            global_event_subscription: None,
            global_event_emitter,
        }));

        // Set up event subscription
        {
            let manager_weak = Rc::downgrade(&manager);
            let subscription = global_event_observer.subscribe(move |event| {
                trace!(target: "resource_manager", "Received global event: {:?}", event);
                if let Some(manager) = manager_weak.upgrade() {
                    manager.borrow_mut().handle_global_event(event);
                }
            });
            manager.borrow_mut().global_event_subscription = Some(subscription);
        }

        manager
    }

    fn handle_global_event(&mut self, event: &GlobalEvent) {
        match event {
            GlobalEvent::OptimizeImages {
                candidate_tile_size,
                solution_tile_size,
                scale_factor,
            } => {
                trace!(target: "resource_manager", "Optimizing images");
                let new_image_set = self.image_set.optimized_image_set(
                    *candidate_tile_size,
                    *solution_tile_size,
                    *scale_factor,
                );
                self.image_set = Rc::new(new_image_set);
                trace!(target: "resource_manager", "Emitting images optimized event");
                self.global_event_emitter
                    .emit(GlobalEvent::ImagesOptimized(self.image_set.clone()));
            }
            _ => (),
        }
    }

    pub fn get_image_set(&self) -> Rc<ImageSet> {
        self.image_set.clone()
    }
    pub fn get_audio_set(&self) -> Rc<AudioSet> {
        self.audio_set.clone()
    }
}
