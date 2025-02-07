use std::{cell::RefCell, rc::Rc};

use gtk4::gdk;

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventObserver, Unsubscriber},
    game::settings::Settings,
    model::{Clickable, GameActionEvent, GlobalEvent, InputEvent, LONG_PRESS_DURATION},
};

pub struct InputTranslator {
    game_action_emitter: EventEmitter<GameActionEvent>,
    settings: Settings,
    input_subscription: Option<Unsubscriber<InputEvent>>,
    global_subscription: Option<Unsubscriber<GlobalEvent>>,
}

impl Destroyable for InputTranslator {
    fn destroy(&mut self) {
        // Clean up subscriptions
        if let Some(subscription) = self.input_subscription.take() {
            subscription.unsubscribe();
        }
        if let Some(subscription) = self.global_subscription.take() {
            subscription.unsubscribe();
        }
    }
}

impl InputTranslator {
    pub fn new(
        game_action_emitter: EventEmitter<GameActionEvent>,
        input_event_observer: EventObserver<InputEvent>,
        global_event_observer: EventObserver<GlobalEvent>,
        settings: &Settings,
    ) -> Rc<RefCell<Self>> {
        let input_translator = Rc::new(RefCell::new(Self {
            game_action_emitter,
            settings: settings.clone(),
            input_subscription: None,
            global_subscription: None,
        }));

        InputTranslator::bind_input_observer(input_translator.clone(), input_event_observer);
        InputTranslator::bind_global_observer(input_translator.clone(), global_event_observer);

        input_translator
    }

    fn bind_input_observer(
        input_translator: Rc<RefCell<Self>>,
        input_event_observer: EventObserver<InputEvent>,
    ) {
        let subscription = {
            let input_translator = input_translator.clone();
            input_event_observer.subscribe(move |event| {
                input_translator.borrow().handle_input_event(event);
            })
        };

        input_translator.borrow_mut().input_subscription = Some(subscription);
    }

    fn bind_global_observer(
        input_translator: Rc<RefCell<Self>>,
        global_event_observer: EventObserver<GlobalEvent>,
    ) {
        let subscription = {
            let input_translator = input_translator.clone();
            global_event_observer.subscribe(move |event| {
                input_translator.borrow_mut().handle_global_event(event);
            })
        };

        input_translator.borrow_mut().global_subscription = Some(subscription);
    }

    fn handle_input_event(&self, event: &InputEvent) {
        match event {
            InputEvent::LeftClick(clickable) => {
                // In touch mode, we wait for LeftClickUp to determine the action
                if !self.settings.touch_screen_controls {
                    self.handle_left_click(clickable);
                }
            }
            InputEvent::RightClick(clickable) => {
                // Ignore right clicks in touch mode
                if !self.settings.touch_screen_controls {
                    self.handle_right_click(clickable);
                }
            }
            InputEvent::TouchEvent(clickable, duration) => {
                if self.settings.touch_screen_controls {
                    self.handle_touch_click(clickable, *duration);
                }
            }
            InputEvent::KeyPressed(key) => self.handle_key_press(*key),
        }
    }

    fn handle_touch_click(&self, clickable: &Clickable, duration: std::time::Duration) {
        let is_long_press = duration >= LONG_PRESS_DURATION;
        match clickable {
            Clickable::CandidateCellTile(data) => {
                // Long press = left click, short press = right click
                if is_long_press {
                    self.game_action_emitter.emit(GameActionEvent::CellClick(
                        data.row,
                        data.col,
                        Some(data.variant),
                    ));
                } else {
                    self.game_action_emitter
                        .emit(GameActionEvent::CellRightClick(
                            data.row,
                            data.col,
                            Some(data.variant),
                        ));
                }
            }
            Clickable::SolutionTile(data) => {
                // Long press = left click, short press = right click
                if is_long_press {
                    self.game_action_emitter
                        .emit(GameActionEvent::CellClick(data.row, data.col, None));
                } else {
                    self.game_action_emitter
                        .emit(GameActionEvent::CellRightClick(data.row, data.col, None));
                }
            }
            Clickable::Clue(data) => {
                // Long press = left click (focus), short press = right click (toggle complete)
                if is_long_press {
                    self.game_action_emitter
                        .emit(GameActionEvent::ClueFocus(Some((
                            data.orientation,
                            data.clue_idx,
                        ))));
                } else {
                    self.game_action_emitter
                        .emit(GameActionEvent::ClueToggleComplete(
                            data.orientation,
                            data.clue_idx,
                        ));
                }
            }
            Clickable::Surface => {
                // Surface clicks are always treated as focus removal, regardless of duration
                self.game_action_emitter
                    .emit(GameActionEvent::ClueFocus(None));
            }
        }
    }

    fn handle_left_click(&self, clickable: &Clickable) {
        match clickable {
            Clickable::CandidateCellTile(data) => {
                self.game_action_emitter.emit(GameActionEvent::CellClick(
                    data.row,
                    data.col,
                    Some(data.variant),
                ));
            }
            Clickable::SolutionTile(data) => {
                self.game_action_emitter
                    .emit(GameActionEvent::CellClick(data.row, data.col, None));
            }
            Clickable::Clue(data) => {
                self.game_action_emitter
                    .emit(GameActionEvent::ClueFocus(Some((
                        data.orientation,
                        data.clue_idx,
                    ))));
            }
            Clickable::Surface => {
                self.game_action_emitter
                    .emit(GameActionEvent::ClueFocus(None));
            }
        }
    }

    fn handle_right_click(&self, clickable: &Clickable) {
        match clickable {
            Clickable::CandidateCellTile(data) => {
                self.game_action_emitter
                    .emit(GameActionEvent::CellRightClick(
                        data.row,
                        data.col,
                        Some(data.variant),
                    ));
            }
            Clickable::SolutionTile(data) => {
                self.game_action_emitter
                    .emit(GameActionEvent::CellRightClick(data.row, data.col, None));
            }
            Clickable::Clue(data) => {
                self.game_action_emitter
                    .emit(GameActionEvent::ClueToggleComplete(
                        data.orientation,
                        data.clue_idx,
                    ));
            }
            _ => {} // No right-click handling for other clickables
        }
    }

    fn handle_key_press(&self, key: gdk::Key) {
        match key {
            gdk::Key::a | gdk::Key::k => {
                self.game_action_emitter
                    .emit(GameActionEvent::ClueFocusNext(-1));
            }
            gdk::Key::d | gdk::Key::j => {
                self.game_action_emitter
                    .emit(GameActionEvent::ClueFocusNext(1));
            }
            gdk::Key::c => {
                self.game_action_emitter
                    .emit(GameActionEvent::ClueToggleSelectedComplete);
            }
            gdk::Key::Escape => {
                self.game_action_emitter
                    .emit(GameActionEvent::ClueFocus(None));
            }
            _ => {} // Ignore other keys
        }
    }

    fn handle_global_event(&mut self, event: &GlobalEvent) {
        match event {
            GlobalEvent::SettingsChanged(settings) => {
                self.settings = settings.clone();
            }
            _ => (),
        }
    }
}
