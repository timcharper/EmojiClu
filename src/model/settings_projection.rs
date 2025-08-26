use crate::destroyable::Destroyable;
use crate::events::EventHandler;
use crate::game::settings::Settings;
use crate::model::GameEngineEvent;
use std::cell::RefCell;
use std::rc::Rc;

pub struct SettingsProjection {
    settings: Settings,
}

impl Destroyable for SettingsProjection {
    fn destroy(&mut self) {
        // No-op: handled centrally
    }
}

impl SettingsProjection {
    pub fn new(initial: &Settings) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            settings: initial.clone(),
        }))
    }

    pub fn current_settings(&self) -> Settings {
        self.settings.clone()
    }
}

impl EventHandler<GameEngineEvent> for SettingsProjection {
    fn handle_event(&mut self, event: &GameEngineEvent) {
        if let GameEngineEvent::SettingsChanged(new_settings) = event {
            self.settings = new_settings.clone();
        }
    }
}
