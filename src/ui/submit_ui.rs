use gtk4::{prelude::*, Label};
use gtk4::{
    ApplicationWindow, Button, ButtonsType, Dialog, DialogFlags, MessageDialog, MessageType,
    ResponseType,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::destroyable::Destroyable;
use crate::events::EventEmitter;
use crate::events::EventObserver;
use crate::events::Unsubscriber;
use crate::game::stats_manager::StatsManager;
use crate::model::GameStateEvent;
use crate::model::{GameActionEvent, PuzzleCompletionState};
use crate::ui::stats_dialog::StatsDialog;

use super::audio_set::AudioSet;

pub struct SubmitUI {
    subscription_id: Option<Unsubscriber<GameStateEvent>>,
    stats_manager: Rc<RefCell<StatsManager>>,
    audio_set: Rc<AudioSet>,
    window: Rc<ApplicationWindow>,
    game_action_emitter: EventEmitter<GameActionEvent>,
    submit_dialog: Rc<RefCell<CompletionDialog>>,
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
        stats_manager: &Rc<RefCell<StatsManager>>,
        audio_set: &Rc<AudioSet>,
        window: &Rc<ApplicationWindow>,
    ) -> Rc<RefCell<Self>> {
        // Create submit button
        let submit_button = Rc::new(Button::with_label("Submit"));
        submit_button.set_tooltip_text(Some("Submit puzzle solution"));
        submit_button.set_action_name(Some("win.submit"));

        let submit_dialog: Rc<RefCell<CompletionDialog>>;

        submit_dialog = CompletionDialog::new(
            window,
            Box::new({
                let game_action_emitter = game_action_emitter.clone();
                move || {
                    game_action_emitter.emit(GameActionEvent::CompletePuzzle);
                }
            }),
            Box::new({
                let game_action_emitter = game_action_emitter.clone();
                move || {
                    game_action_emitter.emit(GameActionEvent::Undo);
                }
            }),
        );

        let submit_ui = Rc::new(RefCell::new(Self {
            subscription_id: None,
            stats_manager: Rc::clone(stats_manager),
            audio_set: Rc::clone(audio_set),
            window: Rc::clone(window),
            game_action_emitter: game_action_emitter,
            submit_dialog,
        }));

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
                let media = self.audio_set.random_win_sound();
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
                let media = self.audio_set.random_lose_sound();
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
                if *all_cells_filled {
                    CompletionDialog::show(submit_ui_moved.borrow().submit_dialog.clone());
                }
            }
            GameStateEvent::PuzzleSuccessfullyCompleted(state) => {
                submit_ui_moved.borrow().handle_game_completion(state);
            }
            _ => (),
        });
        submit_ui.borrow_mut().subscription_id = Some(subscription_id);
    }
}

struct CompletionDialog {
    window: Rc<ApplicationWindow>,
    is_active: bool,
    on_submit: Box<dyn Fn()>,
    on_undo: Box<dyn Fn()>,
    accepted: bool,
}

impl CompletionDialog {
    fn new(
        window: &Rc<ApplicationWindow>,
        on_submit: Box<dyn Fn()>,
        on_undo: Box<dyn Fn()>,
    ) -> Rc<RefCell<Self>> {
        let completion_dialog = Rc::new(RefCell::new(CompletionDialog {
            window: Rc::clone(window),
            is_active: false,
            accepted: false,
            on_submit,
            on_undo,
        }));

        completion_dialog
    }

    fn show(completion_dialog: Rc<RefCell<Self>>) {
        let completion_dialog_weak = Rc::downgrade(&completion_dialog);
        let mut completion_dialog = completion_dialog.borrow_mut();
        if completion_dialog.is_active {
            return;
        }
        completion_dialog.is_active = true;
        completion_dialog.accepted = false;
        let dialog = Dialog::builder()
            .transient_for(completion_dialog.window.as_ref())
            .modal(true)
            .build();

        let content_area = dialog.content_area();
        content_area.set_margin_bottom(20);
        content_area.set_margin_top(20);
        content_area.set_margin_start(20);
        content_area.set_margin_end(20);
        content_area.set_spacing(20);

        let label = Label::builder()
            .label("Submit Solution?")
            .css_classes(["completion-label"])
            .build();
        content_area.append(&label);

        let submit_button = Button::builder()
            .label("Submit")
            .css_classes(["completion-submit-button"])
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(10)
            .margin_end(10)
            .build();

        let undo_button = Button::builder()
            .label("Go Back")
            .css_classes(["completion-undo-button"])
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(10)
            .margin_end(10)
            .build();

        dialog.add_action_widget(&undo_button, ResponseType::Cancel);
        dialog.add_action_widget(&submit_button, ResponseType::Accept);
        drop(completion_dialog);

        dialog.connect_response(move |dialog, response| {
            if let Some(completion_dialog) = completion_dialog_weak.upgrade() {
                let mut completion_dialog = completion_dialog.borrow_mut();
                match response {
                    ResponseType::Accept => {
                        completion_dialog.accepted = true;
                        (completion_dialog.on_submit)();
                    }
                    ResponseType::Cancel => {}
                    _ => {
                        if !completion_dialog.accepted {
                            (completion_dialog.on_undo)();
                        }
                        completion_dialog.is_active = false;
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    }
}
