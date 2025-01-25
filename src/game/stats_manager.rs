use crate::model::Difficulty;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug)]
pub struct StatsManager {
    data_dir: PathBuf,
    scores: HashMap<Difficulty, Vec<GameStats>>,
    global_stats: HashMap<Difficulty, GlobalStats>,
}

impl StatsManager {
    pub fn new() -> Self {
        let data_dir = glib::user_data_dir().join("gwatson");
        if !data_dir.exists() {
            let _ = fs::create_dir_all(&data_dir);
        }

        let mut manager = Self {
            data_dir,
            scores: HashMap::new(),
            global_stats: HashMap::new(),
        };

        // Load existing data
        manager.load_all();
        manager
    }

    fn scores_path(&self, difficulty: Difficulty) -> PathBuf {
        self.data_dir.join(format!(
            "scores_{}.json",
            difficulty.to_string().to_lowercase()
        ))
    }

    fn global_stats_path(&self, difficulty: Difficulty) -> PathBuf {
        self.data_dir.join(format!(
            "global_stats_{}.json",
            difficulty.to_string().to_lowercase()
        ))
    }

    fn load_all(&mut self) {
        // Initialize empty data for all difficulties
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Moderate,
            Difficulty::Hard,
            Difficulty::Veteran,
        ] {
            self.scores.insert(difficulty, Vec::new());
            self.global_stats.insert(
                difficulty,
                GlobalStats {
                    difficulty,
                    ..Default::default()
                },
            );

            // Try to load scores
            if let Ok(contents) = fs::read_to_string(self.scores_path(difficulty)) {
                if let Ok(scores) = serde_json::from_str(&contents) {
                    self.scores.insert(difficulty, scores);
                }
            }

            // Try to load global stats
            if let Ok(contents) = fs::read_to_string(self.global_stats_path(difficulty)) {
                if let Ok(stats) = serde_json::from_str(&contents) {
                    self.global_stats.insert(difficulty, stats);
                }
            }
        }
    }

    fn save_scores(&self, difficulty: Difficulty) -> std::io::Result<()> {
        if let Some(scores) = self.scores.get(&difficulty) {
            let contents = serde_json::to_string_pretty(scores)?;
            fs::write(self.scores_path(difficulty), contents)?;
        }
        Ok(())
    }

    fn save_global_stats(&self, difficulty: Difficulty) -> std::io::Result<()> {
        if let Some(stats) = self.global_stats.get(&difficulty) {
            let contents = serde_json::to_string_pretty(stats)?;
            fs::write(self.global_stats_path(difficulty), contents)?;
        }
        Ok(())
    }

    pub fn record_game(&mut self, stats: &GameStats) -> std::io::Result<()> {
        let difficulty = stats.difficulty;

        // Update scores
        let scores = self.scores.entry(difficulty).or_default();
        scores.push(stats.clone());

        // Sort by completion time only
        scores.sort_by(|a, b| a.completion_time.cmp(&b.completion_time));

        // Keep only top 20 scores
        scores.truncate(20);

        // Update global stats
        let global_stats = self.global_stats.entry(difficulty).or_default();
        global_stats.total_games_played += 1;
        global_stats.total_time_played += stats.completion_time;
        global_stats.total_hints_used += stats.hints_used;

        // Save to files
        self.save_scores(difficulty)?;
        self.save_global_stats(difficulty)?;

        Ok(())
    }

    pub fn get_high_scores(&self, difficulty: Difficulty, limit: usize) -> Vec<GameStats> {
        self.scores
            .get(&difficulty)
            .map(|scores| scores.iter().take(limit).cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_global_stats(&self, difficulty: Difficulty) -> GlobalStats {
        self.global_stats
            .get(&difficulty)
            .cloned()
            .unwrap_or_else(|| GlobalStats {
                difficulty,
                ..Default::default()
            })
    }
}
