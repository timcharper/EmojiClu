use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

use crate::destroyable::Destroyable;
use crate::events::EventHandler;
use crate::model::GameEngineEvent;
use crate::model::TimerState;
use fluent_i18n::t;

pub struct PauseScreenUI {
    pub pause_screen_box: GtkBox,
    pub pause_label: Label,
}

impl Destroyable for PauseScreenUI {
    fn destroy(&mut self) {
        // Subscriptions are handled centrally via EventHandler/subscribe_component
    }
}

impl PauseScreenUI {
    pub fn new() -> Rc<RefCell<Self>> {
        // Create pause label
        let pause_label = Label::builder()
            .name("pause-label")
            .label(&t!("paused"))
            .css_classes(["pause-label"])
            .visible(true)
            .hexpand(true)
            .vexpand(true)
            .build();

        // Create pause screen box
        let pause_screen_box = GtkBox::builder()
            .name("pause-screen")
            .orientation(Orientation::Vertical)
            .visible(false)
            .build();

        pause_screen_box.append(&pause_label);

        let pause_screen_ui = Rc::new(RefCell::new(Self {
            pause_screen_box,
            pause_label,
        }));

        pause_screen_ui
    }

    fn handle_timer_state_changed(&mut self, timer_state: &TimerState) {
        if timer_state.is_paused() {
            self.pause_screen_box.set_visible(true);
        } else {
            self.pause_screen_box.set_visible(false);
        }
    }
}

impl EventHandler<GameEngineEvent> for PauseScreenUI {
    fn handle_event(&mut self, event: &GameEngineEvent) {
        match event {
            GameEngineEvent::TimerStateChanged(timer_state) => {
                self.handle_timer_state_changed(timer_state);
            }
            _ => (),
        }
    }
}
