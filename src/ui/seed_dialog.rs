use std::cell::Cell;
use std::{cell::RefCell, rc::Rc};

use glib::Propagation;
use gtk4::gdk;
use gtk4::EventControllerKey;
use gtk4::{prelude::*, ApplicationWindow, Entry};

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventObserver, Unsubscriber},
    model::{Difficulty, GameActionEvent, GameStateEvent},
};
use fluent_i18n::t;

pub struct SeedDialog {
    window: Rc<ApplicationWindow>,
    game_action_emitter: EventEmitter<GameActionEvent>,
    subscription_id: Option<Unsubscriber<GameStateEvent>>,
    current_seed: Option<u64>,
    current_difficulty: Difficulty,
}

impl Destroyable for SeedDialog {
    fn destroy(&mut self) {
        if let Some(subscription_id) = self.subscription_id.take() {
            subscription_id.unsubscribe();
        }
    }
}

impl SeedDialog {
    pub fn new(
        window: &Rc<ApplicationWindow>,
        game_action_emitter: EventEmitter<GameActionEvent>,
        game_state_observer: EventObserver<GameStateEvent>,
    ) -> Rc<RefCell<Self>> {
        let dialog = Rc::new(RefCell::new(Self {
            window: window.clone(),
            game_action_emitter,
            subscription_id: None,
            current_seed: None,
            current_difficulty: Difficulty::Easy, // Default value, will be updated by observer
        }));

        // Connect observer to track current seed and difficulty
        let dialog_clone = dialog.clone();
        let subscription_id = game_state_observer.subscribe(move |event| {
            if let GameStateEvent::GridUpdate(board) = event {
                let mut dialog = dialog_clone.borrow_mut();
                dialog.current_seed = Some(board.solution.seed);
                dialog.current_difficulty = board.solution.difficulty;
            }
        });
        dialog.borrow_mut().subscription_id = Some(subscription_id);

        dialog
    }

    pub fn show_seed(&self) {
        let content_area = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(10)
            .margin_bottom(10)
            .margin_top(10)
            .margin_start(20)
            .margin_end(20)
            .build();

        let dialog = gtk4::Window::builder()
            .title(&t!("game-seed"))
            .transient_for(self.window.as_ref())
            .modal(true)
            .child(&content_area)
            .default_width(300)
            .build();

        let entry = Entry::builder()
            .text(self.current_seed.map_or("".to_string(), |s| s.to_string()))
            .build();
        content_area.append(&entry);

        let button_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .build();
        let ok_button = gtk4::Button::builder().label(&t!("ok")).build();
        let cancel_button = gtk4::Button::builder().label(&t!("cancel")).build();
        button_box.append(&cancel_button);
        button_box.append(&ok_button);
        button_box.set_halign(gtk4::Align::End);
        content_area.append(&button_box);

        cancel_button.connect_clicked({
            let dialog = dialog.clone();
            move |_| {
                dialog.close();
            }
        });

        let value_accepted = Rc::new(Cell::new(false));

        ok_button.connect_clicked({
            let dialog = dialog.clone();
            let value_accepted = value_accepted.clone();

            move |_| {
                value_accepted.set(true);
                dialog.close();
            }
        });

        entry.connect_activate({
            let dialog = dialog.clone();
            let value_accepted = value_accepted.clone();
            move |_| {
                value_accepted.set(true);
                dialog.close();
            }
        });

        let key_controller = EventControllerKey::new();
        key_controller.connect_key_pressed({
            let dialog = dialog.clone();
            move |_, keyval, _, _| {
                if keyval == gdk::Key::Escape {
                    dialog.close();
                    return Propagation::Stop;
                }
                Propagation::Proceed
            }
        });
        dialog.connect_close_request({
            let value_accepted = value_accepted.clone();
            let game_action_emitter = self.game_action_emitter.clone();
            let current_seed = self.current_seed;
            let current_difficulty = self.current_difficulty;
            move |_| {
                if value_accepted.take() {
                    if let Ok(new_seed) = entry.text().as_str().parse::<u64>() {
                        if Some(new_seed) != current_seed {
                            game_action_emitter
                                .emit(GameActionEvent::NewGame(current_difficulty, Some(new_seed)));
                        }
                    }
                }
                return Propagation::Proceed;
            }
        });
        dialog.add_controller(key_controller);
        dialog.present();
    }
}
