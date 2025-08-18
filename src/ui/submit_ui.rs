use glib::Propagation;
use gtk4::gdk::Key;
use gtk4::{prelude::*, EventControllerKey, Label};
use gtk4::{ApplicationWindow, Button};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::destroyable::Destroyable;
use crate::events::EventEmitter;
use crate::events::EventObserver;
use crate::events::Unsubscriber;
use crate::game::stats_manager::StatsManager;
use crate::model::GameStateEvent;
use crate::model::{GameActionEvent, PuzzleCompletionState};
use crate::ui::stats_dialog::StatsDialog;
use fluent_i18n::t;

use super::audio_set::AudioSet;
use super::NotQuiteRightDialog;

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
        let submit_button = Rc::new(Button::with_label(&t!("submit")));
        submit_button.set_tooltip_text(Some(&t!("submit-puzzle-solution")));
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
                // Play game over sound using a MediaStream
                self.game_action_emitter
                    .emit(GameActionEvent::IncrementHintsUsed);
                let media = self.audio_set.random_lose_sound();
                media.play();

                NotQuiteRightDialog::new(&self.window, self.game_action_emitter.clone()).show();
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
            GameStateEvent::PuzzleCompleted(state) => {
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

        let content_area = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(20)
            .margin_bottom(20)
            .margin_top(20)
            .margin_start(20)
            .margin_end(20)
            .build();

        completion_dialog.is_active = true;
        let modal = gtk4::Window::builder()
            .transient_for(completion_dialog.window.as_ref())
            .modal(true)
            .child(&content_area)
            .build();

        let label = Label::builder()
            .label(&t!("submit-solution"))
            .css_classes(["completion-label"])
            .build();
        content_area.append(&label);

        let button_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(10)
            .halign(gtk4::Align::Center)
            .build();
        content_area.append(&button_box);

        let submit_button = Button::builder()
            .label(&t!("submit"))
            .css_classes(["completion-submit-button"])
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(10)
            .margin_end(10)
            .build();

        let undo_button = Button::builder()
            .label(&t!("go-back"))
            .css_classes(["completion-undo-button"])
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(10)
            .margin_end(10)
            .build();

        button_box.append(&undo_button);
        button_box.append(&submit_button);

        drop(completion_dialog);

        let accepted = Rc::new(Cell::new(false));
        submit_button.connect_clicked({
            let modal = modal.clone();
            let accepted = accepted.clone();
            move |_| {
                accepted.set(true);
                modal.close();
            }
        });

        undo_button.connect_clicked({
            let modal = modal.clone();
            let accepted = accepted.clone();
            move |_| {
                accepted.set(false);
                modal.close();
            }
        });

        let key_controller = EventControllerKey::new();
        key_controller.connect_key_pressed({
            let modal = modal.clone();
            move |_, key, _, _| {
                if key == Key::Escape {
                    modal.close();
                    return Propagation::Stop;
                }
                Propagation::Proceed
            }
        });

        modal.add_controller(key_controller);

        modal.connect_close_request(move |_| {
            if let Some(completion_dialog) = completion_dialog_weak.upgrade() {
                let mut completion_dialog = completion_dialog.borrow_mut();
                if accepted.get() {
                    (completion_dialog.on_submit)();
                } else {
                    (completion_dialog.on_undo)();
                }
                completion_dialog.is_active = false;
            }
            Propagation::Proceed
        });

        modal.present();
    }
}
