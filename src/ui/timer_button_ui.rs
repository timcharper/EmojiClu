use gtk::glib::Variant;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Button};
use std::cell::RefCell;
use std::rc::Rc;

use crate::events::EventEmitter;
use crate::model::GameActionEvent;

pub struct TimerButtonUI {
    pub button: Rc<Button>,
    _is_paused: Rc<RefCell<bool>>,
}

impl TimerButtonUI {
    pub fn new(
        window: &Rc<ApplicationWindow>,
        game_action_emitter: EventEmitter<GameActionEvent>,
    ) -> Self {
        let button = Rc::new(
            Button::builder()
                .label("⏸︎")
                .css_classes(["timer-control"])
                .build(),
        );
        let _is_paused = Rc::new(RefCell::new(false));
        button.set_tooltip_text(Some("Pause Game (Space)"));

        button.connect_clicked(TimerButtonUI::pause_resume_handler_button(
            &_is_paused,
            &button,
            game_action_emitter.clone(),
        ));

        // Add pause action
        let action_pause = gtk::gio::SimpleAction::new("pause", None);
        action_pause.connect_activate(TimerButtonUI::pause_resume_handler_action(
            &_is_paused,
            &button,
            game_action_emitter.clone(),
        ));
        window.add_action(&action_pause);

        Self { button, _is_paused }
    }

    fn pause_resume_logic(
        is_paused_ref: &Rc<RefCell<bool>>,
        button_ref: &Button,
        game_action_emitter: EventEmitter<GameActionEvent>,
    ) {
        let mut is_paused = is_paused_ref.borrow_mut();
        if !*is_paused {
            *is_paused = true;
            TimerButtonUI::update_button_state(&button_ref, true);
            game_action_emitter.emit(&GameActionEvent::Pause);
        } else {
            *is_paused = false;
            TimerButtonUI::update_button_state(&button_ref, false);
            game_action_emitter.emit(&GameActionEvent::Resume);
        }
    }

    fn pause_resume_handler_button<T>(
        is_paused_ref: &Rc<RefCell<bool>>,
        button_ref: &Rc<Button>,
        game_action_emitter: EventEmitter<GameActionEvent>,
    ) -> impl Fn(&T) {
        let button_ref = Rc::clone(&button_ref);
        let is_paused_ref = Rc::clone(&is_paused_ref);
        move |_| {
            TimerButtonUI::pause_resume_logic(
                &is_paused_ref,
                &button_ref,
                game_action_emitter.clone(),
            )
        }
    }

    fn pause_resume_handler_action<T>(
        is_paused_ref: &Rc<RefCell<bool>>,
        button_ref: &Rc<Button>,
        game_action_emitter: EventEmitter<GameActionEvent>,
    ) -> impl Fn(&T, Option<&Variant>) {
        let button_ref = Rc::clone(&button_ref);
        let is_paused_ref = Rc::clone(&is_paused_ref);
        move |_, _| {
            TimerButtonUI::pause_resume_logic(
                &is_paused_ref,
                &button_ref,
                game_action_emitter.clone(),
            )
        }
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
