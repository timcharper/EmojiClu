// handles stats and things about the game

use std::{cell::RefCell, rc::Rc, time::Duration};

use glib::{timeout_add_local, Continue, SourceId};
use gtk::{prelude::WidgetExt, Label, ScrolledWindow};

use crate::{
    destroyable::Destroyable,
    events::{EventObserver, Unsubscriber},
    model::{GameStateEvent, TimerState},
};

pub struct GameInfoUI {
    hints_used: u32,
    timer_state: TimerState,
    pub timer_label: Label,
    pub hints_label: Label,
    timer: Option<SourceId>,
    game_box: Rc<gtk::Box>,
    pause_screen: Rc<gtk::Box>,
    game_state_subscription: Option<Unsubscriber<GameStateEvent>>,
}

impl Destroyable for GameInfoUI {
    fn destroy(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
        if let Some(subscription) = self.game_state_subscription.take() {
            subscription.unsubscribe();
        }
    }
}

impl GameInfoUI {
    pub fn new(
        game_state_observer: EventObserver<GameStateEvent>,
        game_box: Rc<gtk::Box>,
        pause_screen: Rc<gtk::Box>,
    ) -> Rc<RefCell<Self>> {
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
            game_box,
            pause_screen,
            game_state_subscription: None,
        }));

        game_info
            .borrow_mut()
            .start_timer_label_handler(game_info.clone());
        GameInfoUI::bind_observer(Rc::clone(&game_info), game_state_observer);

        game_info
    }

    fn bind_observer(
        game_info: Rc<RefCell<Self>>,
        game_state_observer: EventObserver<GameStateEvent>,
    ) {
        let game_state_subscription = {
            let game_info = game_info.clone();
            game_state_observer.subscribe(move |event| {
                game_info
                    .borrow_mut()
                    .handle_game_state_event(game_info.clone(), event);
            })
        };

        game_info.borrow_mut().game_state_subscription = Some(game_state_subscription);
    }

    fn handle_game_state_event(&mut self, game_info: Rc<RefCell<Self>>, event: &GameStateEvent) {
        match event {
            GameStateEvent::TimerStateChanged(timer_state) => {
                self.update_timer_state(game_info.clone(), &timer_state);
            }
            GameStateEvent::HintUsageChanged(hints_used) => {
                self.update_hints_used(*hints_used);
            }
            _ => {}
        }
    }

    pub fn update_hints_used(&mut self, hints_used: u32) {
        println!("update_hints_used: {}", hints_used);
        self.hints_used = hints_used;
        self.hints_label.set_text(&format!("{}", hints_used));
    }

    pub fn update_timer_state(
        &mut self,
        game_info: Rc<RefCell<Self>>,
        new_timer_state: &TimerState,
    ) {
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
            self.start_timer_label_handler(game_info.clone());
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

    fn start_timer_label_handler(&mut self, game_info: Rc<RefCell<Self>>) {
        // time running? Do nothing.
        if self.timer.is_none() {
            let game_info_weak = Rc::downgrade(&game_info);
            let timer = timeout_add_local(Duration::from_secs(1), move || {
                if let Some(game_info) = game_info_weak.upgrade() {
                    let game_info = game_info.borrow();
                    GameInfoUI::update_timer_label(&game_info.timer_label, &game_info.timer_state);
                    Continue(true)
                } else {
                    Continue(false)
                }
            });
            self.timer = Some(timer);
        }
    }

    fn update_timer_label(timer_label: &Label, timer_state: &TimerState) {
        let elapsed = timer_state.elapsed();
        let minutes = elapsed.as_secs() / 60;
        let seconds = elapsed.as_secs() % 60;
        timer_label.set_text(&format!("{:02}:{:02}", minutes, seconds));
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
