use std::{cell::RefCell, rc::Rc};

use gtk4::gdk;

use crate::{
    destroyable::Destroyable,
    events::{EventEmitter, EventHandler},
    model::{Clickable, GameEngineCommand, InputEvent, SettingsProjection, LONG_PRESS_DURATION},
};

pub struct InputTranslator {
    game_engine_command_emitter: EventEmitter<GameEngineCommand>,
    settings_projection: Rc<RefCell<SettingsProjection>>,
}

impl Destroyable for InputTranslator {
    fn destroy(&mut self) {
        // No-op: subscriptions are handled centrally via EventHandler
    }
}

impl InputTranslator {
    pub fn new(
        game_engine_command_emitter: EventEmitter<GameEngineCommand>,
        settings_projection: Rc<RefCell<SettingsProjection>>,
    ) -> Rc<RefCell<Self>> {
        let input_translator = Rc::new(RefCell::new(Self {
            game_engine_command_emitter,
            settings_projection: settings_projection.clone(),
        }));

        input_translator
    }

    fn handle_touch_click(&self, clickable: &Clickable, duration: std::time::Duration) {
        let is_long_press = duration >= LONG_PRESS_DURATION;
        match clickable {
            Clickable::CandidateCellTile(data) => {
                // Long press = left click, short press = right click
                if is_long_press {
                    self.game_engine_command_emitter
                        .emit(GameEngineCommand::CellSelect(
                            data.row,
                            data.col,
                            Some(data.variant),
                        ));
                } else {
                    self.game_engine_command_emitter
                        .emit(GameEngineCommand::CellClear(
                            data.row,
                            data.col,
                            Some(data.variant),
                        ));
                }
            }
            Clickable::SolutionTile(data) => {
                // Long press = left click, short press = right click
                if is_long_press {
                    self.game_engine_command_emitter
                        .emit(GameEngineCommand::CellSelect(data.row, data.col, None));
                } else {
                    self.game_engine_command_emitter
                        .emit(GameEngineCommand::CellClear(data.row, data.col, None));
                }
            }
            Clickable::Clue(address) => {
                // Long press = left click (focus), short press = right click (toggle complete)
                if is_long_press {
                    self.game_engine_command_emitter
                        .emit(GameEngineCommand::ClueFocus(Some(*address)));
                } else {
                    self.game_engine_command_emitter
                        .emit(GameEngineCommand::ClueToggleComplete(*address));
                }
            }
            Clickable::Surface => {
                // Surface clicks are always treated as focus removal, regardless of duration
                self.game_engine_command_emitter
                    .emit(GameEngineCommand::ClueFocus(None));
            }
        }
    }

    fn handle_left_click(&self, clickable: &Clickable) {
        match clickable {
            Clickable::CandidateCellTile(data) => {
                self.game_engine_command_emitter
                    .emit(GameEngineCommand::CellSelect(
                        data.row,
                        data.col,
                        Some(data.variant),
                    ));
            }
            Clickable::SolutionTile(data) => {
                self.game_engine_command_emitter
                    .emit(GameEngineCommand::CellSelect(data.row, data.col, None));
            }
            Clickable::Clue(address) => {
                self.game_engine_command_emitter
                    .emit(GameEngineCommand::ClueFocus(Some(*address)));
            }
            Clickable::Surface => {
                self.game_engine_command_emitter
                    .emit(GameEngineCommand::ClueFocus(None));
            }
        }
    }

    fn handle_right_click(&self, clickable: &Clickable) {
        match clickable {
            Clickable::CandidateCellTile(data) => {
                self.game_engine_command_emitter
                    .emit(GameEngineCommand::CellClear(
                        data.row,
                        data.col,
                        Some(data.variant),
                    ));
            }
            Clickable::SolutionTile(data) => {
                self.game_engine_command_emitter
                    .emit(GameEngineCommand::CellClear(data.row, data.col, None));
            }
            Clickable::Clue(address) => {
                self.game_engine_command_emitter
                    .emit(GameEngineCommand::ClueToggleComplete(*address));
            }
            _ => {} // No right-click handling for other clickables
        }
    }

    fn handle_key_press(&self, key: gdk::Key) {
        match key {
            gdk::Key::a | gdk::Key::k => {
                self.game_engine_command_emitter
                    .emit(GameEngineCommand::ClueFocusNext(-1));
            }
            gdk::Key::d | gdk::Key::j => {
                self.game_engine_command_emitter
                    .emit(GameEngineCommand::ClueFocusNext(1));
            }
            gdk::Key::c => {
                self.game_engine_command_emitter
                    .emit(GameEngineCommand::ClueToggleSelectedComplete);
            }
            gdk::Key::Escape => {
                self.game_engine_command_emitter
                    .emit(GameEngineCommand::ClueFocus(None));
            }
            _ => {} // Ignore other keys
        }
    }

    // Extracted wrappers for the match branches in EventHandler::handle_event.
    // These contain the touch-mode checks and delegate to the existing handlers.
    fn handle_left_click_event(&self, clickable: &Clickable) {
        if !self
            .settings_projection
            .borrow()
            .current_settings()
            .touch_screen_controls
        {
            self.handle_left_click(clickable);
        }
    }

    fn handle_right_click_event(&self, clickable: &Clickable) {
        if !self
            .settings_projection
            .borrow()
            .current_settings()
            .touch_screen_controls
        {
            self.handle_right_click(clickable);
        }
    }

    fn handle_touch_event(&self, clickable: &Clickable, duration: std::time::Duration) {
        if self
            .settings_projection
            .borrow()
            .current_settings()
            .touch_screen_controls
        {
            self.handle_touch_click(clickable, duration);
        }
    }
}

impl EventHandler<InputEvent> for InputTranslator {
    fn handle_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::LeftClick(clickable) => {
                // In touch mode, we wait for LeftClickUp to determine the action
                self.handle_left_click_event(clickable);
            }
            InputEvent::RightClick(clickable) => {
                // Ignore right clicks in touch mode
                self.handle_right_click_event(clickable);
            }
            InputEvent::TouchEvent(clickable, duration) => {
                self.handle_touch_event(clickable, *duration);
            }
            InputEvent::KeyPressed(key) => self.handle_key_press(*key),
        }
    }
}
