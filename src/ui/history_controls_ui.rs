use glib::timeout_add_local_once;
use gtk::prelude::*;
use gtk::Button;
use log::trace;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::destroyable::Destroyable;
use crate::events::EventEmitter;
use crate::events::EventObserver;
use crate::events::Unsubscriber;
use crate::model::GameActionEvent;
use crate::model::GameStateEvent;

pub struct HistoryControlsUI {
    pub undo_button: Rc<Button>,
    pub redo_button: Rc<Button>,
    subscription_id: Option<Unsubscriber<GameStateEvent>>,
    game_state_observer: EventObserver<GameStateEvent>,
}

impl Destroyable for HistoryControlsUI {
    fn destroy(&mut self) {
        if let Some(subscription_id) = self.subscription_id.take() {
            subscription_id.unsubscribe();
        }
    }
}

impl HistoryControlsUI {
    pub fn new(
        game_state_observer: EventObserver<GameStateEvent>,
        game_action_emitter: EventEmitter<GameActionEvent>,
    ) -> Rc<RefCell<Self>> {
        // Create buttons first
        let undo_button = Rc::new(Button::from_icon_name("edit-undo-symbolic"));
        let redo_button = Rc::new(Button::from_icon_name("edit-redo-symbolic"));
        undo_button.set_tooltip_text(Some("Undo (Ctrl+Z)"));
        redo_button.set_tooltip_text(Some("Redo (Ctrl+Shift+Z)"));

        undo_button.set_action_name(Some("win.undo"));
        redo_button.set_action_name(Some("win.redo"));

        // Wire up undo button
        // Because we're connected to the action here, we don't need to bind another handler.
        // let game_action_emitter_undo = game_action_emitter.clone();
        // undo_button.connect_clicked(move |_| {
        //     game_action_emitter_undo.emit(&GameActionEvent::Undo);
        // });
        // let game_action_emitter_redo = game_action_emitter.clone();
        // redo_button.connect_clicked(move |_| {
        //     game_action_emitter_redo.emit(&GameActionEvent::Redo);
        // });

        let history_controls_ui = Rc::new(RefCell::new(Self {
            undo_button,
            redo_button,
            game_state_observer: game_state_observer.clone(),
            subscription_id: None,
        }));

        timeout_add_local_once(
            Duration::default(),
            Self::idle_add_handler(history_controls_ui.clone()),
        );

        HistoryControlsUI::connect_observer(history_controls_ui.clone(), game_state_observer);

        history_controls_ui
    }

    fn connect_observer(
        history_controls_ui: Rc<RefCell<Self>>,
        game_state_observer: EventObserver<GameStateEvent>,
    ) {
        let history_controls_ui_moved = history_controls_ui.clone();
        let subscription_id = game_state_observer.subscribe(move |event| match event {
            GameStateEvent::HistoryChanged {
                history_index,
                history_length,
            } => history_controls_ui_moved
                .borrow()
                .update_buttons(*history_index, *history_length),
            _ => (),
        });
        history_controls_ui.borrow_mut().subscription_id = Some(subscription_id);
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
