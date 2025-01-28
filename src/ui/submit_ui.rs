use glib::timeout_add_local_once;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Button};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::destroyable::Destroyable;
use crate::events::EventEmitter;
use crate::events::EventObserver;
use crate::events::Unsubscriber;
use crate::game::game_state::GameState;
use crate::game::stats_manager::StatsManager;
use crate::model::GameActionEvent;
use crate::model::GameStateEvent;
use crate::ui::stats_dialog::StatsDialog;
use crate::ui::ResourceSet;

pub struct SubmitUI {
    pub submit_button: Rc<Button>,
    subscription_id: Option<Unsubscriber<GameStateEvent>>,
    game_state_observer: EventObserver<GameStateEvent>,
}

impl Destroyable for SubmitUI {
    fn destroy(&mut self) {
        if let Some(subscription_id) = self.subscription_id.take() {
            subscription_id.unsubscribe();
        }
    }
}

impl SubmitUI {
    pub fn new(
        game_state_observer: EventObserver<GameStateEvent>,
        game_action_emitter: EventEmitter<GameActionEvent>,
        game_state: &Rc<RefCell<GameState>>,
        stats_manager: &Rc<RefCell<StatsManager>>,
        resources: &Rc<ResourceSet>,
    ) -> Rc<RefCell<Self>> {
        // Create submit button
        let submit_button = Rc::new(Button::with_label("Submit"));
        submit_button.set_tooltip_text(Some("Submit puzzle solution"));
        submit_button.set_action_name(Some("win.submit"));

        // Wire up submit button with handler
        let game_action_emitter_submit = game_action_emitter.clone();
        let game_state_submit = Rc::clone(game_state);
        let stats_manager_submit = Rc::clone(stats_manager);
        let resources_submit = Rc::clone(resources);
        submit_button.connect_clicked(move |button| {
            let state = game_state_submit.try_borrow().ok().and_then(|gs| {
                stats_manager_submit
                    .try_borrow_mut()
                    .ok()
                    .and_then(|sm| Some((gs, sm)))
            });
            if let Some((state, mut stats_manager)) = state {
                if state.current_board.is_complete() && !state.current_board.is_incorrect() {
                    button.remove_css_class("submit-ready"); // Stop blinking once clicked
                    let media = resources_submit.random_win_sound();
                    media.play();

                    // Record completion and show stats
                    let stats = state.get_game_stats();
                    let difficulty = state.current_board.solution.difficulty;

                    if let Err(e) = stats_manager.record_game(&stats) {
                        log::error!(target: "window", "Failed to record game stats: {}", e);
                    }

                    if let Some(window) = button
                        .root()
                        .and_then(|r| r.downcast::<ApplicationWindow>().ok())
                    {
                        // Drop the mutable borrow before showing stats
                        let game_action_emitter = game_action_emitter_submit.clone();
                        StatsDialog::show(&window, &state, &stats_manager, Some(stats), move || {
                            game_action_emitter.emit(&GameActionEvent::NewGame(difficulty));
                        });
                    }
                } else {
                    let dialog = gtk::MessageDialog::new(
                        button
                            .root()
                            .and_then(|r| r.downcast::<gtk::Window>().ok())
                            .as_ref(),
                        gtk::DialogFlags::MODAL,
                        gtk::MessageType::Info,
                        gtk::ButtonsType::OkCancel,
                        "Sorry, that's not quite right. Click OK to rewind to the last correct state.",
                    );

                    // Play game over sound using a MediaStream
                    let media = resources_submit.random_lose_sound();
                    media.play();

                    let game_action_emitter = game_action_emitter_submit.clone();
                    dialog.connect_response(move |dialog, response| {
                        if response == gtk::ResponseType::Ok {
                            game_action_emitter.emit(&GameActionEvent::RewindLastGood);
                        }
                        dialog.close();
                    });
                    dialog.show();
                }
            }
        });

        let submit_ui = Rc::new(RefCell::new(Self {
            submit_button,
            game_state_observer: game_state_observer.clone(),
            subscription_id: None,
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

    fn connect_observer(
        submit_ui: Rc<RefCell<Self>>,
        game_state_observer: EventObserver<GameStateEvent>,
    ) {
        let submit_ui_moved = submit_ui.clone();
        let subscription_id = game_state_observer.subscribe(move |event| match event {
            GameStateEvent::PuzzleCompletionStateChanged(all_cells_filled) => {
                submit_ui_moved.borrow().update_button(*all_cells_filled)
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
