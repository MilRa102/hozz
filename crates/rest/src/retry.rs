use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::RestError;

/// Policy controlling retry attempts, exponential backoff, and jitter.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of total attempts (including the initial attempt).
    pub max_attempts: u32,
    /// Initial backoff delay for the first retry attempt.
    pub initial_delay: Duration,
    /// Maximum delay cap for any single retry attempt.
    pub max_delay: Duration,
    /// Multiplier applied to the delay on each subsequent retry.
    pub backoff_factor: f64,
    /// Whether to apply random jitter (+/- 25%) to backoff delays.
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(10),
            backoff_factor: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Creates a policy with no retries (only 1 attempt).
    pub fn none() -> Self {
        Self {
            max_attempts: 1,
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            backoff_factor: 1.0,
            jitter: false,
        }
    }

    /// Creates a builder-like configuration for retry policy.
    pub fn new(max_attempts: u32, initial_delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay,
            ..Default::default()
        }
    }

    /// Sets maximum retry attempts.
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Sets maximum delay cap.
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }

    /// Sets backoff multiplier factor.
    pub fn with_backoff_factor(mut self, backoff_factor: f64) -> Self {
        self.backoff_factor = backoff_factor;
        self
    }

    /// Enables or disables jitter.
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Determines whether another attempt should be made for the given attempt number and error.
    pub fn should_retry(&self, attempt: u32, error: &RestError) -> bool {
        if attempt >= self.max_attempts {
            return false;
        }
        error.is_transient()
    }

    /// Calculates delay duration for a specific retry attempt (1-based attempt index for retry).
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt <= 1 {
            return self.apply_jitter(self.initial_delay);
        }

        let exponent = (attempt - 1) as i32;
        let mult = self.backoff_factor.powi(exponent);
        let calculated_secs = self.initial_delay.as_secs_f64() * mult;
        let delay = Duration::from_secs_f64(calculated_secs).min(self.max_delay);

        self.apply_jitter(delay)
    }

    fn apply_jitter(&self, delay: Duration) -> Duration {
        if !self.jitter || delay.is_zero() {
            return delay;
        }

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(500);

        // Jitter multiplier between 0.75 and 1.25
        let jitter_factor = 0.75 + (f64::from(nanos % 500) / 1000.0);
        let jittered_secs = delay.as_secs_f64() * jitter_factor;

        Duration::from_secs_f64(jittered_secs).min(self.max_delay)
    }
}
