use crate::{game::settings::Settings, ui::ResourceSet};
use std::rc::Rc;

use super::LayoutConfiguration;

/// Events that are not specific to any one component of the game.
#[derive(Debug)]
pub enum GlobalEvent {
    SettingsChanged(Settings),
    DimensionsChanged(Rc<ResourceSet>),
    LayoutChanged(LayoutConfiguration),
}
