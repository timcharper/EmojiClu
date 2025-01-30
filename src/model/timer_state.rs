use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct TimerState {
    pub paused_timestamp: Option<Instant>,
    pub paused_duration: Duration,
    pub started_timestamp: Instant,
    pub ended_timestamp: Option<Instant>,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            paused_timestamp: None,
            paused_duration: Duration::from_secs(0),
            started_timestamp: Instant::now(),
            ended_timestamp: None,
        }
    }
}

impl TimerState {
    pub fn elapsed(&self) -> Duration {
        let until_time = self
            .paused_timestamp
            .or(self.ended_timestamp)
            .unwrap_or(Instant::now());

        until_time
            .saturating_duration_since(self.started_timestamp)
            .saturating_sub(self.paused_duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elapsed_with_pause() {
        let now = Instant::now();
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
        let now = Instant::now();
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
        let now = Instant::now();
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
        let now = Instant::now();
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
