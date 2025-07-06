use log::trace;

use crate::model::{GameBoard, Solution};
use crate::solver::clue_generator::ClueGeneratorResult;
use crate::solver::generate_clues;
use std::path::PathBuf;
use std::time::SystemTime;
use std::{fs, rc::Rc};

use super::{Difficulty, TimerState};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameStateSnapshot {
    pub board: GameBoard,
    pub timer_state: TimerState,
    pub hints_used: u32,
}

fn game_state_path() -> PathBuf {
    let data_dir = glib::user_data_dir();
    let mut path = data_dir.join("emojiclu");
    path.push("game_state.json");
    path
}

impl GameStateSnapshot {
    pub fn new(board: GameBoard, timer_state: TimerState, hints_used: u32) -> Self {
        let paused_timer_state = if timer_state.is_paused() {
            timer_state.paused(SystemTime::now())
        } else {
            timer_state
        };
        Self {
            board,
            timer_state: paused_timer_state,
            hints_used,
        }
    }

    pub fn generate_new(difficulty: Difficulty, seed: Option<u64>) -> Self {
        let solution = Rc::new(Solution::new(difficulty, seed));
        trace!(target: "game_state", "Generated solution: {:?}", solution);
        let blank_board = GameBoard::new(Rc::clone(&solution));
        let ClueGeneratorResult {
            clues: _,
            board,
            revealed_tiles: _,
        } = generate_clues(&blank_board);

        Self::new(board, TimerState::default(), 0)
    }

    pub fn save(&self) -> bool {
        save_game_state_snapshot(self)
    }
}

fn save_game_state_snapshot(game_state: &GameStateSnapshot) -> bool {
    let path = game_state_path();
    if let Some(dir) = path.parent() {
        if let Err(_) = fs::create_dir_all(dir) {
            return false;
        }
    }
    match serde_json::to_string(game_state) {
        Ok(contents) => {
            if fs::write(path, contents).is_ok() {
                return true;
            }
        }
        Err(_) => return false,
    }
    false
}

pub fn load_game_state_snapshot() -> Option<GameStateSnapshot> {
    let path = game_state_path();
    if let Ok(contents) = fs::read_to_string(&path) {
        if let Ok(game_state) = serde_json::from_str::<GameStateSnapshot>(&contents) {
            return Some(game_state);
        }
    }
    None
}
