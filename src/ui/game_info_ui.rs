// handles stats and things about the game

use std::{cell::RefCell, rc::Rc, time::Duration};

use glib::{timeout_add_local, Continue, SourceId};
use gtk::{prelude::WidgetExt, Label};

use crate::model::TimerState;

pub struct GameInfoUI {
    hints_used: u32,
    timer_state: Rc<RefCell<TimerState>>,
    pub timer_label: Rc<Label>,
    pub hints_label: Rc<Label>,
    timer: RefCell<Option<SourceId>>,
}

impl GameInfoUI {
    pub fn new() -> Self {
        // Create timer label with monospace font
        let timer_label = Rc::new(Label::new(None));
        timer_label.set_css_classes(&["timer"]);
        // Create hints label
        let hints_label = Rc::new(Label::new(Some("0")));
        hints_label.set_css_classes(&["hints"]);

        // Set up timer update
        let timer_state = TimerState::default();
        GameInfoUI::update_timer_label(&timer_label, &timer_state);

        let timer_state = Rc::new(RefCell::new(timer_state));
        let mut game_info = Self {
            hints_used: 0,
            timer_state,
            timer_label,
            hints_label,
            timer: RefCell::new(None),
        };
        game_info.start_timer_label_handler();
        game_info
    }

    pub fn update_hints_used(&mut self, hints_used: u32) {
        println!("update_hints_used: {}", hints_used);
        self.hints_used = hints_used;
        self.hints_label.set_text(&format!("{}", hints_used));
    }

    pub fn update_timer_state(&mut self, new_timer_state: &TimerState) {
        let mut timer_state = self.timer_state.borrow_mut();
        *timer_state = new_timer_state.clone();
        GameInfoUI::update_timer_label(&self.timer_label, &timer_state);
        let is_paused = timer_state.paused_timestamp.is_some();
        drop(timer_state);
        if is_paused {
            self.pause_timer_label_handler();
        } else {
            self.start_timer_label_handler();
        }
    }

    fn pause_timer_label_handler(&mut self) {
        if let Some(timer) = self.timer.get_mut().take() {
            timer.remove();
        }
    }

    fn start_timer_label_handler(&mut self) {
        // time running? Do nothing.
        if self.timer.borrow().is_none() {
            let timer = timeout_add_local(
                Duration::from_secs(1),
                GameInfoUI::timer_update_handler(&self.timer_label, &self.timer_state),
            );
            self.timer.replace(Some(timer));
        }
    }

    fn update_timer_label(timer_label: &Label, timer_state: &TimerState) {
        let elapsed = timer_state.elapsed();
        let minutes = elapsed.as_secs() / 60;
        let seconds = elapsed.as_secs() % 60;
        timer_label.set_text(&format!("{:02}:{:02}", minutes, seconds));
    }

    fn timer_update_handler(
        timer_label: &Rc<Label>,
        timer_state: &Rc<RefCell<TimerState>>,
    ) -> impl Fn() -> Continue {
        let timer_label = Rc::clone(&timer_label);
        let timer_state = Rc::clone(&timer_state);
        move || {
            GameInfoUI::update_timer_label(timer_label.as_ref(), &timer_state.borrow());
            Continue(true)
        }
    }
}

impl Drop for GameInfoUI {
    fn drop(&mut self) {
        log::trace!(target: "game_info_ui", "Dropping GameInfoUI");

        if let Some(timer) = self.timer.get_mut().take() {
            timer.remove();
        }
    }
}
