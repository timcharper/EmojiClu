use std::{cell::RefCell, rc::Rc};

use gtk::prelude::*;
use gtk::{ApplicationWindow, Dialog, Entry};

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventObserver, Unsubscriber},
    model::{Difficulty, GameActionEvent, GameStateEvent},
};

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
        let dialog = Dialog::builder()
            .title("Game Seed")
            .transient_for(self.window.as_ref())
            .modal(true)
            .build();

        dialog.add_button("OK", gtk::ResponseType::Ok);
        dialog.add_button("Cancel", gtk::ResponseType::Cancel);

        let content_area = dialog.content_area();
        let entry = Entry::builder()
            .text(self.current_seed.map_or("".to_string(), |s| s.to_string()))
            .build();
        content_area.append(&entry);

        let game_action_emitter = self.game_action_emitter.clone();
        let current_seed = self.current_seed;
        let current_difficulty = self.current_difficulty;

        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Ok {
                if let Ok(new_seed) = entry.text().as_str().parse::<u64>() {
                    if Some(new_seed) != current_seed {
                        game_action_emitter
                            .emit(GameActionEvent::NewGame(current_difficulty, Some(new_seed)));
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    }
}
