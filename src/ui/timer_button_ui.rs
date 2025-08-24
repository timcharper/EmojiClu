use gtk4::{prelude::*, ApplicationWindow, Button};
use std::cell::RefCell;
use std::rc::Rc;

use crate::destroyable::Destroyable;
use crate::events::EventEmitter;
use crate::model::GameEngineCommand;
use fluent_i18n::t;
use gio::SimpleAction;

pub struct TimerButtonUI {
    pub button: Button,
    is_paused: bool,
    game_engine_command_emitter: EventEmitter<GameEngineCommand>,
}

impl Destroyable for TimerButtonUI {
    fn destroy(&mut self) {}
}

impl TimerButtonUI {
    pub fn new(
        window: &Rc<ApplicationWindow>,
        game_engine_command_emitter: EventEmitter<GameEngineCommand>,
    ) -> Rc<RefCell<Self>> {
        let button = Button::builder()
            .label(&t!("timer-pause"))
            .css_classes(["timer-control"])
            .action_name("win.pause")
            .build();
        button.set_tooltip_text(Some(&t!("timer-pause-tooltip")));

        let timer_button_ui = Rc::new(RefCell::new(Self {
            button,
            is_paused: false,
            game_engine_command_emitter,
        }));

        let action_pause = SimpleAction::new("pause", None);

        {
            let timer_button_ui = timer_button_ui.clone();
            action_pause.connect_activate(move |_, _| {
                let mut timer_button_ui = timer_button_ui.borrow_mut();
                timer_button_ui.toggle_pause();
            });
        }
        window.add_action(&action_pause);

        timer_button_ui
    }

    fn toggle_pause(&mut self) {
        if self.is_paused {
            self.is_paused = false;
            self.game_engine_command_emitter.emit(GameEngineCommand::Resume);
        } else {
            self.is_paused = true;
            self.game_engine_command_emitter.emit(GameEngineCommand::Pause);
        }
        TimerButtonUI::update_button_state(&self.button, self.is_paused);
    }

    fn update_button_state(button: &Button, is_paused: bool) {
        if is_paused {
            button.set_label("▶");
            button.set_tooltip_text(Some("Resume Game (Space)"));
        } else {
            button.set_label("⏸︎");
            button.set_tooltip_text(Some("Pause Game (Space)"));
        }
    }
}
