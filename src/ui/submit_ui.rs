use glib::timeout_add_local_once;
use gtk4::glib::SignalHandlerId;
use gtk4::prelude::*;
use gtk4::{
    ApplicationWindow, Button, ButtonsType, DialogFlags, MessageDialog, MessageType, ResponseType,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::destroyable::Destroyable;
use crate::events::EventEmitter;
use crate::events::EventObserver;
use crate::events::Unsubscriber;
use crate::game::stats_manager::StatsManager;
use crate::model::GameStateEvent;
use crate::model::{GameActionEvent, PuzzleCompletionState};
use crate::ui::stats_dialog::StatsDialog;
use crate::ui::ResourceSet;

pub struct SubmitUI {
    pub submit_button: Rc<Button>,
    subscription_id: Option<Unsubscriber<GameStateEvent>>,
    stats_manager: Rc<RefCell<StatsManager>>,
    resources: Rc<ResourceSet>,
    submit_button_clicked_signal: Option<SignalHandlerId>,
    window: Rc<ApplicationWindow>,
    game_action_emitter: EventEmitter<GameActionEvent>,
}

impl Destroyable for SubmitUI {
    fn destroy(&mut self) {
        if let Some(subscription_id) = self.subscription_id.take() {
            subscription_id.unsubscribe();
        }
        if let Some(submit_button_clicked_signal) = self.submit_button_clicked_signal.take() {
            self.submit_button.disconnect(submit_button_clicked_signal);
        }
    }
}

impl SubmitUI {
    pub fn new(
        game_state_observer: EventObserver<GameStateEvent>,
        game_action_emitter: EventEmitter<GameActionEvent>,
        stats_manager: &Rc<RefCell<StatsManager>>,
        resources: &Rc<ResourceSet>,
        window: &Rc<ApplicationWindow>,
    ) -> Rc<RefCell<Self>> {
        // Create submit button
        let submit_button = Rc::new(Button::with_label("Submit"));
        submit_button.set_tooltip_text(Some("Submit puzzle solution"));
        submit_button.set_action_name(Some("win.submit"));

        let submit_button_clicked_signal: SignalHandlerId;

        {
            let game_action_emitter_submit = game_action_emitter.clone();
            submit_button_clicked_signal = submit_button.connect_clicked(move |_| {
                game_action_emitter_submit.emit(GameActionEvent::CompletePuzzle);
            });
        }

        let submit_ui = Rc::new(RefCell::new(Self {
            submit_button,
            subscription_id: None,
            stats_manager: Rc::clone(stats_manager),
            resources: Rc::clone(resources),
            submit_button_clicked_signal: Some(submit_button_clicked_signal),
            window: Rc::clone(window),
            game_action_emitter: game_action_emitter,
        }));

        // Initialize button state
        timeout_add_local_once(
            Duration::default(),
            Self::idle_add_handler(submit_ui.clone()),
        );

        // Connect observer
        SubmitUI::connect_observer(submit_ui.clone(), game_state_observer);

        submit_ui
    }

    fn handle_game_completion(&self, completion_state: &PuzzleCompletionState) {
        match completion_state {
            PuzzleCompletionState::Incomplete => {
                // just ignore
            }
            PuzzleCompletionState::Correct(stats) => {
                self.submit_button.remove_css_class("submit-ready"); // Stop blinking once clicked
                let media = self.resources.random_win_sound();
                media.play();

                let difficulty = stats.difficulty;

                if let Err(e) = self.stats_manager.borrow_mut().record_game(&stats) {
                    log::error!(target: "window", "Failed to record game stats: {}", e);
                }

                // Drop the mutable borrow before showing stats
                let game_action_emitter = self.game_action_emitter.clone();
                let stats_manager = self.stats_manager.as_ref().borrow_mut();
                StatsDialog::show(
                    &self.window,
                    difficulty,
                    &stats_manager,
                    Some(stats),
                    move || {
                        game_action_emitter.emit(GameActionEvent::NewGame(difficulty, None));
                    },
                );
            }
            PuzzleCompletionState::Incorrect => {
                let dialog = MessageDialog::new(
                    Some(self.window.as_ref()),
                    DialogFlags::MODAL,
                    MessageType::Info,
                    ButtonsType::OkCancel,
                    "Sorry, that's not quite right. Click OK to rewind to the last correct state.",
                );

                // Play game over sound using a MediaStream
                let media = self.resources.random_lose_sound();
                media.play();

                let game_action_emitter = self.game_action_emitter.clone();
                dialog.connect_response(move |dialog, response| {
                    if response == ResponseType::Ok {
                        game_action_emitter.emit(GameActionEvent::RewindLastGood);
                    }
                    dialog.close();
                });
                dialog.show();
            }
        }
    }

    fn connect_observer(
        submit_ui: Rc<RefCell<Self>>,
        game_state_observer: EventObserver<GameStateEvent>,
    ) {
        let submit_ui_moved = submit_ui.clone();
        let subscription_id = game_state_observer.subscribe(move |event| match event {
            GameStateEvent::PuzzleSubmissionReadyChanged(all_cells_filled) => {
                submit_ui_moved.borrow().update_button(*all_cells_filled)
            }
            GameStateEvent::PuzzleSuccessfullyCompleted(state) => {
                submit_ui_moved.borrow().handle_game_completion(state);
            }
            _ => (),
        });
        submit_ui.borrow_mut().subscription_id = Some(subscription_id);
    }

    fn idle_add_handler(submit_ui: Rc<RefCell<Self>>) -> impl Fn() {
        let submit_ui = submit_ui.clone();
        move || {
            submit_ui.borrow().update_button(false);
        }
    }

    fn update_button(&self, all_cells_filled: bool) {
        self.submit_button.set_sensitive(all_cells_filled);
        if all_cells_filled {
            self.submit_button.add_css_class("submit-ready");
        } else {
            self.submit_button.remove_css_class("submit-ready");
        }
    }
}
