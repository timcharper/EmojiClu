use gtk::{
    glib::{ToVariant, Variant},
    prelude::ActionGroupExt,
    ApplicationWindow,
};
use log::trace;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum GameEvent {
    CellClick(usize, usize, Option<char>),
    CellRightClick(usize, usize, Option<char>),
    HorizontalClueClick(usize), // clue_idx
    VerticalClueClick(usize),   // clue_idx
    NewGame(usize),             // grid rows, grid columns
    InitDisplay,
    Solve,
    RewindLastGood,
    IncrementHintsUsed,
    ShowHint,
    Undo,
    Redo,
    Pause,
    Resume,
    Quit,
}

impl GameEvent {
    pub fn to_variant(&self) -> Variant {
        let result = serde_json::to_string(self)
            .expect("Failed to serialize event")
            .to_variant();

        trace!(target: "game_event", "Serialized event: {:?}", result);
        result
    }

    pub fn from_variant(variant: &Variant) -> Option<Self> {
        let serialized = variant.get::<String>()?;
        let result = serde_json::from_str(&serialized).ok()?;
        trace!(target: "game_event", "Deserialized event: {:?}", result);
        Some(result)
    }
    pub fn dispatch_event(window: &ApplicationWindow, event: GameEvent) {
        trace!(target: "game_event", "Dispatching event: {:?}", event);
        window.activate_action("game-event", Some(&event.to_variant()));
    }
}
