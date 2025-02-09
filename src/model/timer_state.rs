use std::time::{Duration, SystemTime};

use serde_with::serde_as;
use serde_with::TimestampSeconds;

#[serde_as]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TimerState {
    #[serde_as(as = "Option<TimestampSeconds>")]
    pub paused_timestamp: Option<SystemTime>,
    pub paused_duration: Duration,
    #[serde_as(as = "TimestampSeconds")]
    pub started_timestamp: SystemTime,
    #[serde_as(as = "Option<TimestampSeconds>")]
    pub ended_timestamp: Option<SystemTime>,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            paused_timestamp: None,
            paused_duration: Duration::from_secs(0),
            started_timestamp: SystemTime::now(),
            ended_timestamp: None,
        }
    }
}

impl TimerState {
    pub fn is_paused(&self) -> bool {
        self.paused_timestamp.is_some()
    }

    pub fn elapsed(&self) -> Duration {
        let until_time = self
            .paused_timestamp
            .or(self.ended_timestamp)
            .unwrap_or(SystemTime::now());

        until_time
            .duration_since(self.started_timestamp)
            .unwrap_or(Duration::default())
            .saturating_sub(self.paused_duration)
    }

    pub fn paused(&self, now: SystemTime) -> TimerState {
        let mut new_state = self.clone();
        new_state.paused_timestamp = Some(now);
        new_state
    }

    pub fn resumed(&self) -> TimerState {
        let mut new_state = self.clone();
        if let Some(pause_time) = new_state.paused_timestamp.take() {
            new_state.paused_duration = new_state
                .paused_duration
                .saturating_add(pause_time.elapsed().unwrap_or(Duration::default()));
        }
        new_state
    }

    pub fn ended(&self, now: SystemTime) -> TimerState {
        let mut new_state = self.clone();
        new_state.ended_timestamp = Some(now);
        new_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elapsed_with_pause() {
        let now = SystemTime::now();
        let timer = TimerState {
            started_timestamp: now,
            paused_timestamp: Some(now + Duration::from_secs(5)),
            paused_duration: Duration::from_secs(0),
            ended_timestamp: None,
        };

        assert_eq!(timer.elapsed(), Duration::from_secs(5));
    }

    #[test]
    fn test_elapsed_with_end() {
        let now = SystemTime::now();
        let timer = TimerState {
            started_timestamp: now,
            paused_timestamp: None,
            paused_duration: Duration::from_secs(0),
            ended_timestamp: Some(now + Duration::from_secs(10)),
        };

        assert_eq!(timer.elapsed(), Duration::from_secs(10));
    }

    #[test]
    fn test_elapsed_with_pause_and_accumulated_pause() {
        let now = SystemTime::now();
        let timer = TimerState {
            started_timestamp: now,
            paused_timestamp: Some(now + Duration::from_secs(10)),
            paused_duration: Duration::from_secs(3),
            ended_timestamp: None,
        };

        assert_eq!(timer.elapsed(), Duration::from_secs(7)); // 10 seconds total - 3 seconds paused
    }

    #[test]
    fn test_elapsed_running() {
        let now = SystemTime::now();
        let timer = TimerState {
            started_timestamp: now - Duration::from_secs(5), // Started 5 seconds ago
            paused_timestamp: None,
            paused_duration: Duration::from_secs(0),
            ended_timestamp: None,
        };

        // Since we're using real time here for Instant::now(), we just verify it's at least 5 seconds
        assert!(timer.elapsed() >= Duration::from_secs(5));
    }
}
