use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct TimerState {
    pub paused_timestamp: Option<Instant>,
    pub paused_duration: Duration,
    pub started_timestamp: Instant,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            paused_timestamp: None,
            paused_duration: Duration::from_secs(0),
            started_timestamp: Instant::now(),
        }
    }
}

impl TimerState {
    pub fn elapsed(&self) -> Duration {
        let mut elapsed = if self.paused_timestamp.is_some() {
            let paused_timestamp = self.paused_timestamp.unwrap();
            paused_timestamp.saturating_duration_since(self.started_timestamp)
        } else {
            self.started_timestamp.elapsed()
        };

        // remove paused duration
        elapsed = elapsed.saturating_sub(self.paused_duration);
        elapsed
    }
}
