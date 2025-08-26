// handles stats and things about the game

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    time::Duration,
};

use glib::{timeout_add_local, SourceId};
use gtk4::{prelude::*, Box, Label, Orientation};

use crate::{
    destroyable::Destroyable,
    events::EventHandler,
    model::{GameEngineEvent, TimerState},
};

pub struct GameInfoUI {
    hints_used: u32,
    timer_state: TimerState,
    pub timer_label: Label,
    pub hints_label: Label,
    timer: Option<SourceId>,
    pub game_box: Rc<Box>,
    pause_screen: Rc<Box>,
    self_weak: Option<Weak<RefCell<GameInfoUI>>>,
}

impl Destroyable for GameInfoUI {
    fn destroy(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
    }
}

impl GameInfoUI {
    pub fn new(pause_screen: Rc<Box>) -> Rc<RefCell<Self>> {
        // Create timer label with monospace font
        let timer_label = Label::new(None);
        timer_label.set_css_classes(&["timer"]);
        // Create hints label
        let hints_label = Label::new(Some("0"));
        hints_label.set_css_classes(&["hints"]);

        // Set up timer update
        let timer_state = TimerState::default();
        GameInfoUI::update_timer_label(&timer_label, &timer_state);

        // Create game area with puzzle and horizontal clues side by side
        let game_box = Rc::new(
            gtk4::Box::builder()
                .name("game-box")
                .orientation(Orientation::Horizontal)
                .spacing(10)
                .halign(gtk4::Align::Center)
                .hexpand(true)
                .margin_start(10)
                .margin_end(10)
                .build(),
        );
        let game_info = Rc::new(RefCell::new(Self {
            hints_used: 0,
            timer_state,
            timer_label,
            hints_label,
            timer: None,
            game_box,
            pause_screen,
            self_weak: None,
        }));

        // store a weak reference to self so timer handler can upgrade when needed
        game_info.borrow_mut().self_weak = Some(Rc::downgrade(&game_info));
        game_info.borrow_mut().start_timer_label_handler();

        game_info
    }

    fn handle_game_engine_event(&mut self, event: &GameEngineEvent) {
        match event {
            GameEngineEvent::TimerStateChanged(timer_state) => {
                self.update_timer_state(&timer_state);
            }
            GameEngineEvent::HintUsageChanged(hints_used) => {
                self.update_hints_used(*hints_used);
            }
            _ => {}
        }
    }

    pub fn update_hints_used(&mut self, hints_used: u32) {
        self.hints_used = hints_used;
        self.hints_label.set_text(&format!("{}", hints_used));
    }

    pub fn update_timer_state(&mut self, new_timer_state: &TimerState) {
        self.timer_state = new_timer_state.clone();
        GameInfoUI::update_timer_label(&self.timer_label, &self.timer_state);
        let is_paused = self.timer_state.paused_timestamp.is_some();
        if is_paused {
            // stop the timer update
            self.pause_timer_label_handler();

            // hide the game
            self.game_box.set_visible(false);
            // show the pause screen
            self.pause_screen.set_visible(true);
        } else {
            self.start_timer_label_handler();
            // show the game
            self.game_box.set_visible(true);
            // hide the pause screen
            self.pause_screen.set_visible(false);
        }
    }

    fn pause_timer_label_handler(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
    }

    fn start_timer_label_handler(&mut self) {
        // time running? Do nothing.
        if self.timer.is_none() {
            if let Some(self_weak) = &self.self_weak {
                let game_info_weak = self_weak.clone();
                let timer = timeout_add_local(Duration::from_secs(1), move || {
                    if let Some(game_info) = game_info_weak.upgrade() {
                        let game_info = game_info.borrow();
                        GameInfoUI::update_timer_label(
                            &game_info.timer_label,
                            &game_info.timer_state,
                        );
                        glib::ControlFlow::Continue
                    } else {
                        glib::ControlFlow::Break
                    }
                });
                self.timer = Some(timer);
            }
        }
    }

    fn update_timer_label(timer_label: &Label, timer_state: &TimerState) {
        let elapsed = timer_state.elapsed();
        let minutes = elapsed.as_secs() / 60;
        let seconds = elapsed.as_secs() % 60;
        timer_label.set_text(&format!("{:02}:{:02}", minutes, seconds));
    }
}

impl Drop for GameInfoUI {
    fn drop(&mut self) {
        log::trace!(target: "game_info_ui", "Dropping GameInfoUI");

        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
    }
}

impl EventHandler<GameEngineEvent> for GameInfoUI {
    fn handle_event(&mut self, event: &GameEngineEvent) {
        self.handle_game_engine_event(event);
    }
}
