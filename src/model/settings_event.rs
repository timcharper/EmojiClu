use crate::game::settings::Settings;
use std::rc::Rc;

#[derive(Debug)]
pub enum SettingsEvent {
    SettingsChanged(Rc<Settings>),
}
