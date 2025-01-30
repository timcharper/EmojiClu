use crate::model::Difficulty;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct GameStats {
    pub completion_time: Duration,
    pub hints_used: u32,
    pub grid_size: usize,
    pub difficulty: Difficulty,
    pub timestamp: i64,
    pub playthrough_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalStats {
    pub difficulty: Difficulty,
    pub total_games_played: u32,
    pub total_time_played: Duration,
    pub total_hints_used: u32,
}
