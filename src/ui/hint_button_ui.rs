use glib::timeout_add_local_once;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Button};
use log::trace;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::destroyable::Destroyable;
use crate::events::EventEmitter;
use crate::game::game_engine::GameEngine;
use crate::model::GameEngineCommand;
use crate::ui::audio_set::AudioSet;
use crate::ui::NotQuiteRightDialog;
use fluent_i18n::t;

pub struct HintButtonUI {
    pub hint_button: Button,
}

impl Destroyable for HintButtonUI {
    fn destroy(&mut self) {
        // Subscriptions are handled centrally via subscribe_component (weak refs)
    }
}

impl HintButtonUI {
    pub fn new(
        game_engine_command_emitter: EventEmitter<GameEngineCommand>,
        game_state: &Rc<RefCell<GameEngine>>,
        audio_set: &Rc<AudioSet>,
        window: &Rc<ApplicationWindow>,
    ) -> Rc<RefCell<Self>> {
        // Create hint button
        let hint_button = Button::from_icon_name("view-reveal-symbolic");
        hint_button.set_tooltip_text(Some(&t!("show-hint")));

        // Connect the click handler
        Self::connect_click_handler(
            &hint_button,
            game_engine_command_emitter.clone(),
            game_state,
            audio_set,
            window,
        );

        let hint_button_ui = Rc::new(RefCell::new(Self { hint_button }));

        hint_button_ui
    }

    fn connect_click_handler(
        hint_button: &Button,
        game_engine_command_emitter: EventEmitter<GameEngineCommand>,
        game_state: &Rc<RefCell<GameEngine>>,
        audio_set: &Rc<AudioSet>,
        window: &Rc<ApplicationWindow>,
    ) {
        let game_state = Rc::clone(&game_state);
        let audio_set_hint = Rc::clone(&audio_set);
        let window = Rc::clone(&window);

        hint_button.connect_clicked(move |button| {
            let board_is_incorrect = game_state.borrow().current_board.is_incorrect();
            trace!(target: "hint_button_ui", "Handling hint button click");
            if board_is_incorrect {
                trace!(target: "hint_button_ui", "Board is incorrect, showing rewind dialog");
                let media = audio_set_hint.random_lose_sound();
                media.play();
                NotQuiteRightDialog::new(&window, game_engine_command_emitter.clone()).show();
            } else {
                trace!(target: "hint_button_ui", "Board is correct, showing hint");
                game_engine_command_emitter.emit(GameEngineCommand::ShowHint);
                button.set_sensitive(false);
                let button = button.clone();
                timeout_add_local_once(Duration::from_secs(4), move || {
                    trace!(target: "hint_button_ui", "Re-enabling hint button");
                    button.set_sensitive(true);
                });
            }
        });
    }
}
