use std::{cell::RefCell, rc::Rc};

use gtk4::{gdk, prelude::*, ApplicationWindow, EventControllerKey, GestureClick, ScrolledWindow};

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventObserver, Unsubscriber},
    game::settings::Settings,
    model::{GameActionEvent, GlobalEvent},
};

pub struct GameControlsMonitor {
    window: Rc<ApplicationWindow>,
    scrolled_window: ScrolledWindow,
    key_controller: Option<EventControllerKey>,
    click_controller: Option<GestureClick>,
    game_action_emitter: EventEmitter<GameActionEvent>,
    settings: Settings,
    global_subscription: Option<Unsubscriber<GlobalEvent>>,
}

impl Destroyable for GameControlsMonitor {
    fn destroy(&mut self) {
        // Remove the key controller from the window
        if let Some(key_controller) = self.key_controller.take() {
            self.window.remove_controller(&key_controller);
        }
        // Remove the click controller from the window
        if let Some(click_controller) = self.click_controller.take() {
            self.scrolled_window.remove_controller(&click_controller);
        }
        // Clean up global subscription
        if let Some(subscription) = self.global_subscription.take() {
            subscription.unsubscribe();
        }
    }
}

impl GameControlsMonitor {
    pub fn new(
        window: Rc<ApplicationWindow>,
        scrolled_window: ScrolledWindow,
        game_action_emitter: EventEmitter<GameActionEvent>,
        global_event_observer: EventObserver<GlobalEvent>,
        settings: &Settings,
    ) -> Rc<RefCell<Self>> {
        let game_controls = Rc::new(RefCell::new(Self {
            window: window.clone(),
            scrolled_window,
            key_controller: None,
            click_controller: None,
            game_action_emitter,
            settings: settings.clone(),
            global_subscription: None,
        }));

        GameControlsMonitor::bind_key_press_handler(game_controls.clone());
        GameControlsMonitor::bind_click_handler(game_controls.clone());
        GameControlsMonitor::bind_global_observer(game_controls.clone(), global_event_observer);

        game_controls
    }

    fn bind_click_handler(game_controls: Rc<RefCell<Self>>) {
        let click_controller = GestureClick::new();
        click_controller.set_button(gdk::BUTTON_PRIMARY);

        let weak_game_controls = Rc::downgrade(&game_controls);

        click_controller.connect_pressed(move |gesture, _n_press, _x, _y| {
            if let Some(game_controls) = weak_game_controls.upgrade() {
                let game_controls = game_controls.borrow();
                game_controls
                    .game_action_emitter
                    .emit(GameActionEvent::ClueFocus(None));
                gesture.set_state(gtk4::EventSequenceState::Claimed);
            }
        });

        let mut game_controls = game_controls.borrow_mut();
        game_controls.click_controller = Some(click_controller.clone());
        game_controls
            .scrolled_window
            .add_controller(click_controller);
    }

    fn bind_global_observer(
        game_controls: Rc<RefCell<Self>>,
        global_event_observer: EventObserver<GlobalEvent>,
    ) {
        let subscription = {
            let game_controls = game_controls.clone();
            global_event_observer.subscribe(move |event| {
                game_controls.borrow_mut().handle_global_event(event);
            })
        };

        game_controls.borrow_mut().global_subscription = Some(subscription);
    }

    fn handle_global_event(&mut self, event: &GlobalEvent) {
        match event {
            GlobalEvent::SettingsChanged(settings) => {
                self.settings = settings.clone();
            }
            _ => (),
        }
    }

    fn bind_key_press_handler(game_controls: Rc<RefCell<Self>>) {
        let key_controller = EventControllerKey::new();

        let weak_game_controls = Rc::downgrade(&game_controls);

        // Connect key press handler
        {
            key_controller.connect_key_pressed(move |_controller, key, _keycode, state| {
                let val = key.to_lower();
                let handled = if let Some(game_controls) = weak_game_controls.upgrade() {
                    let game_controls = game_controls.borrow();
                    match val {
                        gdk::Key::a | gdk::Key::k => {
                            game_controls
                                .game_action_emitter
                                .emit(GameActionEvent::ClueFocusNext(-1));
                            true
                        }
                        gdk::Key::c => {
                            game_controls
                                .game_action_emitter
                                .emit(GameActionEvent::ClueToggleSelectedComplete);
                            true
                        }
                        gdk::Key::d | gdk::Key::j => {
                            // Skip if both Ctrl and Shift are pressed, so GTK debugger hot key still works
                            if state.contains(gdk::ModifierType::CONTROL_MASK)
                                && state.contains(gdk::ModifierType::SHIFT_MASK)
                            {
                                false
                            } else {
                                game_controls
                                    .game_action_emitter
                                    .emit(GameActionEvent::ClueFocusNext(1));
                                true
                            }
                        }
                        gdk::Key::Escape => {
                            game_controls
                                .game_action_emitter
                                .emit(GameActionEvent::ClueFocus(None));
                            true
                        }
                        _ => false,
                    }
                } else {
                    false
                };
                handled.into()
            });
        }

        let mut game_controls = game_controls.borrow_mut();
        game_controls.key_controller = Some(key_controller.clone());
        // Add controller to window
        game_controls.window.add_controller(key_controller);
    }
}
