use std::{cell::Cell, cell::RefCell, rc::Rc, time::Instant};

use gtk4::{gdk, prelude::*, ApplicationWindow, EventControllerKey, GestureClick, ScrolledWindow};

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventObserver, Unsubscriber},
    game::settings::Settings,
    model::{Clickable, GameActionEvent, GlobalEvent, InputEvent},
};

pub struct TopLevelInputEventMonitor {
    window: Rc<ApplicationWindow>,
    scrolled_window: ScrolledWindow,
    key_controller: Option<EventControllerKey>,
    click_controller: Option<GestureClick>,
    game_action_emitter: EventEmitter<GameActionEvent>,
    input_event_emitter: EventEmitter<InputEvent>,
    settings: Settings,
    global_subscription: Option<Unsubscriber<GlobalEvent>>,
}

impl Destroyable for TopLevelInputEventMonitor {
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

impl TopLevelInputEventMonitor {
    pub fn new(
        window: Rc<ApplicationWindow>,
        scrolled_window: ScrolledWindow,
        game_action_emitter: EventEmitter<GameActionEvent>,
        input_event_emitter: EventEmitter<InputEvent>,
        global_event_observer: EventObserver<GlobalEvent>,
        settings: &Settings,
    ) -> Rc<RefCell<Self>> {
        let game_controls = Rc::new(RefCell::new(Self {
            window: window.clone(),
            scrolled_window,
            key_controller: None,
            click_controller: None,
            game_action_emitter,
            input_event_emitter,
            settings: settings.clone(),
            global_subscription: None,
        }));

        TopLevelInputEventMonitor::bind_key_press_handler(game_controls.clone());
        TopLevelInputEventMonitor::bind_click_handler(game_controls.clone());
        TopLevelInputEventMonitor::bind_global_observer(
            game_controls.clone(),
            global_event_observer,
        );

        game_controls
    }

    fn bind_click_handler(game_controls: Rc<RefCell<Self>>) {
        let click_controller = GestureClick::new();
        click_controller.set_button(gdk::BUTTON_PRIMARY);

        let weak_game_controls = Rc::downgrade(&game_controls);
        let press_start_time = Rc::new(Cell::new(None));

        // Track press start time and emit LeftClick on press
        {
            let press_start_for_press = Rc::clone(&press_start_time);
            let weak_game_controls_press = weak_game_controls.clone();
            click_controller.connect_pressed(move |gesture, _n_press, _x, _y| {
                if let Some(game_controls) = weak_game_controls_press.upgrade() {
                    let game_controls = game_controls.borrow();
                    press_start_for_press.set(Some(Instant::now()));

                    // Emit LeftClick immediately on press
                    game_controls
                        .input_event_emitter
                        .emit(InputEvent::LeftClick(Clickable::Surface));
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                }
            });
        }

        // Emit LeftClickUp on release with duration
        {
            let press_start_for_release = Rc::clone(&press_start_time);
            let weak_game_controls_release = weak_game_controls.clone();
            click_controller.connect_released(move |gesture, _n_press, _x, _y| {
                if let Some(game_controls) = weak_game_controls_release.upgrade() {
                    let game_controls = game_controls.borrow();
                    let duration = press_start_for_release
                        .get()
                        .map(|start| start.elapsed())
                        .unwrap_or_default();

                    game_controls
                        .input_event_emitter
                        .emit(InputEvent::TouchEvent(Clickable::Surface, duration));
                    gesture.set_state(gtk4::EventSequenceState::Claimed);
                }
                press_start_for_release.set(None);
            });
        }

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

        key_controller.connect_key_pressed(move |_controller, key, _keycode, state| {
            let val = key.to_lower();

            // Skip if both Ctrl and Shift are pressed for 'd' key (GTK debugger hotkey)
            if val == gdk::Key::d
                && state.contains(gdk::ModifierType::CONTROL_MASK)
                && state.contains(gdk::ModifierType::SHIFT_MASK)
            {
                return false.into();
            }

            if let Some(game_controls) = weak_game_controls.upgrade() {
                let game_controls = game_controls.borrow();
                game_controls
                    .input_event_emitter
                    .emit(InputEvent::KeyPressed(val));
                true.into()
            } else {
                false.into()
            }
        });

        let mut game_controls = game_controls.borrow_mut();
        game_controls.key_controller = Some(key_controller.clone());
        game_controls.window.add_controller(key_controller);
    }
}
