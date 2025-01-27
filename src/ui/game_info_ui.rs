// handles stats and things about the game

use std::{cell::RefCell, rc::Rc, time::Duration};

use glib::{timeout_add_local, Continue, SourceId};
use gtk::{prelude::WidgetExt, Label};

use crate::{
    destroyable::Destroyable,
    events::EventObserver,
    model::{GameStateEvent, TimerState},
};

pub struct GameInfoUI {
    hints_used: u32,
    timer_state: TimerState,
    pub timer_label: Label,
    pub hints_label: Label,
    timer: Option<SourceId>,
}

impl Destroyable for GameInfoUI {
    fn destroy(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
    }
}

impl GameInfoUI {
    pub fn new(game_state_observer: EventObserver<GameStateEvent>) -> Rc<RefCell<Self>> {
        // Create timer label with monospace font
        let timer_label = Label::new(None);
        timer_label.set_css_classes(&["timer"]);
        // Create hints label
        let hints_label = Label::new(Some("0"));
        hints_label.set_css_classes(&["hints"]);

        // Set up timer update
        let timer_state = TimerState::default();
        GameInfoUI::update_timer_label(&timer_label, &timer_state);

        let game_info = Rc::new(RefCell::new(Self {
            hints_used: 0,
            timer_state,
            timer_label,
            hints_label,
            timer: None,
        }));

        // Set up timer handler
        game_info
            .borrow_mut()
            .start_timer_label_handler(Rc::clone(&game_info));

        let game_info_handler = game_info.clone();
        game_state_observer.subscribe(move |event| match event {
            GameStateEvent::TimerStateChanged(timer_state) => {
                game_info_handler
                    .borrow_mut()
                    .update_timer_state(&timer_state);
            }
            GameStateEvent::HintUsageChanged(hints_used) => {
                game_info_handler
                    .borrow_mut()
                    .update_hints_used(*hints_used);
            }
            _ => {}
        });

        game_info
    }

    pub fn update_hints_used(&mut self, hints_used: u32) {
        println!("update_hints_used: {}", hints_used);
        self.hints_used = hints_used;
        self.hints_label.set_text(&format!("{}", hints_used));
    }

    pub fn update_timer_state(&mut self, new_timer_state: &TimerState) {
        self.timer_state = new_timer_state.clone();
        GameInfoUI::update_timer_label(&self.timer_label, &self.timer_state);
        let is_paused = self.timer_state.paused_timestamp.is_some();
        if is_paused {
            self.pause_timer_label_handler();
        } else {
            let game_info = unsafe { self.get_self_rc() };
            self.start_timer_label_handler(game_info);
        }
    }

    fn pause_timer_label_handler(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
    }

    fn start_timer_label_handler(&mut self, game_info: Rc<RefCell<Self>>) {
        // time running? Do nothing.
        if self.timer.is_none() {
            let timer = timeout_add_local(
                Duration::from_secs(1),
                GameInfoUI::timer_update_handler(game_info),
            );
            self.timer = Some(timer);
        }
    }

    fn update_timer_label(timer_label: &Label, timer_state: &TimerState) {
        let elapsed = timer_state.elapsed();
        let minutes = elapsed.as_secs() / 60;
        let seconds = elapsed.as_secs() % 60;
        timer_label.set_text(&format!("{:02}:{:02}", minutes, seconds));
    }

    fn timer_update_handler(game_info: Rc<RefCell<Self>>) -> impl Fn() -> Continue {
        move || {
            let game_info = game_info.borrow();
            GameInfoUI::update_timer_label(&game_info.timer_label, &game_info.timer_state);
            Continue(true)
        }
    }

    // SAFETY: This is only safe to call from methods that are called on a GameInfoUI that is stored in an Rc<RefCell<>>
    unsafe fn get_self_rc(&self) -> Rc<RefCell<Self>> {
        let ptr = self as *const Self;
        let offset = ptr.offset(-1);
        Rc::from_raw(offset as *const RefCell<Self>)
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
