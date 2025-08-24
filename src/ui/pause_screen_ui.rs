use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

use crate::destroyable::Destroyable;
use crate::events::{EventObserver, Unsubscriber};
use crate::model::GameStateEvent;
use fluent_i18n::t;

pub struct PauseScreenUI {
    pub pause_screen_box: GtkBox,
    pub pause_label: Label,
    subscription_id: Option<Unsubscriber<GameStateEvent>>,
}

impl Destroyable for PauseScreenUI {
    fn destroy(&mut self) {
        if let Some(subscription_id) = self.subscription_id.take() {
            subscription_id.unsubscribe();
        }
    }
}

impl PauseScreenUI {
    pub fn new(game_state_observer: EventObserver<GameStateEvent>) -> Rc<RefCell<Self>> {
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
            subscription_id: None,
        }));

        // Connect to game state observer for pause/unpause events
        Self::connect_observer(pause_screen_ui.clone(), game_state_observer);

        pause_screen_ui
    }

    fn connect_observer(
        pause_screen_ui: Rc<RefCell<Self>>,
        game_state_observer: EventObserver<GameStateEvent>,
    ) {
        let pause_screen_ui_moved = pause_screen_ui.clone();
        let subscription_id = game_state_observer.subscribe(move |event| match event {
            GameStateEvent::TimerStateChanged(timer_state) => {
                if timer_state.is_paused() {
                    pause_screen_ui_moved
                        .borrow()
                        .pause_screen_box
                        .set_visible(true);
                } else {
                    pause_screen_ui_moved
                        .borrow()
                        .pause_screen_box
                        .set_visible(false);
                }
            }
            _ => (),
        });
        pause_screen_ui.borrow_mut().subscription_id = Some(subscription_id);
    }
}
