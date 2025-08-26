use glib::timeout_add_local_once;
use gtk4::prelude::*;
use gtk4::Button;
use log::trace;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::destroyable::Destroyable;
use crate::events::EventHandler;
use crate::model::GameEngineEvent;

pub struct HistoryControlsUI {
    pub undo_button: Rc<Button>,
    pub redo_button: Rc<Button>,
}

impl Destroyable for HistoryControlsUI {
    fn destroy(&mut self) {
        // Subscriptions are handled centrally via EventHandler/subscribe_component
    }
}

impl HistoryControlsUI {
    pub fn new() -> Rc<RefCell<Self>> {
        // Create buttons first
        let undo_button = Rc::new(Button::from_icon_name("edit-undo-symbolic"));
        let redo_button = Rc::new(Button::from_icon_name("edit-redo-symbolic"));
        undo_button.set_tooltip_text(Some("Undo (Ctrl+Z)"));
        redo_button.set_tooltip_text(Some("Redo (Ctrl+Shift+Z)"));

        undo_button.set_action_name(Some("win.undo"));
        redo_button.set_action_name(Some("win.redo"));

        // Wire up undo button
        // Because we're connected to the action here, we don't need to bind another handler.
        // let game_engine_command_emitter_undo = game_engine_command_emitter.clone();
        // undo_button.connect_clicked(move |_| {
        //     game_engine_command_emitter_undo.emit(&GameActionEvent::Undo);
        // });
        // let game_engine_command_emitter_redo = game_engine_command_emitter.clone();
        // redo_button.connect_clicked(move |_| {
        //     game_engine_command_emitter_redo.emit(&GameActionEvent::Redo);
        // });

        let history_controls_ui = Rc::new(RefCell::new(Self {
            undo_button,
            redo_button,
        }));

        timeout_add_local_once(
            Duration::default(),
            Self::idle_add_handler(history_controls_ui.clone()),
        );

        // Subscriptions are handled centrally via `wire_event_observers`

        history_controls_ui
    }

    fn idle_add_handler(history_controls_ui: Rc<RefCell<Self>>) -> impl Fn() {
        let history_controls_ui = history_controls_ui.clone();

        move || {
            history_controls_ui.borrow().update_buttons(0, 0);
        }
    }

    fn update_buttons(&self, history_index: usize, history_length: usize) {
        trace!(
            target: "history_controls_ui",
            "update_buttons {:?} {:?}",
            history_index,
            history_length
        );
        self.undo_button.set_sensitive(history_index > 0);
        self.redo_button
            .set_sensitive(history_index + 1 < history_length);
    }
}

impl EventHandler<GameEngineEvent> for HistoryControlsUI {
    fn handle_event(&mut self, event: &GameEngineEvent) {
        match event {
            GameEngineEvent::GameBoardUpdated {
                history_index,
                history_length,
                ..
            } => self.update_buttons(*history_index, *history_length),
            _ => (),
        }
    }
}
