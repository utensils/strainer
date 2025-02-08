use crate::config::{BackoffConfig, RateLimits, Thresholds};
use std::time::{Duration, Instant};
use tracing::{info, warn};

#[derive(Debug)]
#[allow(dead_code)]
pub struct UsageStats {
    pub requests_used: u32,
    pub tokens_used: u32,
    pub input_tokens_used: u32,
    pub last_check: Instant,
}

impl Default for UsageStats {
    fn default() -> Self {
        Self {
            requests_used: 0,
            tokens_used: 0,
            input_tokens_used: 0,
            last_check: Instant::now(),
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct RateLimiter {
    limits: RateLimits,
    thresholds: Thresholds,
    backoff: BackoffConfig,
    usage: UsageStats,
}

#[allow(dead_code)]
impl RateLimiter {
    #[must_use]
    pub fn new(limits: RateLimits, thresholds: Thresholds, backoff: BackoffConfig) -> Self {
        Self {
            limits,
            thresholds,
            backoff,
            usage: UsageStats::default(),
        }
    }

    /// Check if any rate limits are exceeded and calculate appropriate backoff
    pub fn check_limits(&mut self) -> (bool, Duration) {
        let now = Instant::now();
        // We don't use elapsed yet, but we'll need it for rate calculation
        let _elapsed = now.duration_since(self.usage.last_check);

        // If all limits are None, allow proceeding
        if self.limits.requests_per_minute.is_none()
            && self.limits.tokens_per_minute.is_none()
            && self.limits.input_tokens_per_minute.is_none()
        {
            return (
                true,
                Duration::from_secs(u64::from(self.backoff.min_seconds)),
            );
        }

        // Calculate current usage percentages
        let requests_percent = if let Some(limit) = self.limits.requests_per_minute {
            Self::calculate_usage_percent(self.usage.requests_used, limit)
        } else {
            0
        };

        let tokens_percent = if let Some(limit) = self.limits.tokens_per_minute {
            Self::calculate_usage_percent(self.usage.tokens_used, limit)
        } else {
            0
        };

        let input_tokens_percent = if let Some(limit) = self.limits.input_tokens_per_minute {
            Self::calculate_usage_percent(self.usage.input_tokens_used, limit)
        } else {
            0
        };

        // Log current usage
        info!(
            "Rate limit status - Requests: {}%, Tokens: {}%, Input Tokens: {}%",
            requests_percent, tokens_percent, input_tokens_percent
        );

        // Check if we're over any thresholds
        let max_percent = requests_percent
            .max(tokens_percent)
            .max(input_tokens_percent);

        // Check thresholds in order of priority
        let critical = u32::from(self.thresholds.critical);
        let warning = u32::from(self.thresholds.warning);

        // Critical threshold takes precedence
        // Note: We treat usage exactly at the critical threshold as exceeding it
        if max_percent >= critical {
            warn!("Usage at or above critical threshold ({}%)", critical);
            return (
                false,
                Duration::from_secs(u64::from(self.backoff.max_seconds)),
            );
        }

        // Then check warning threshold
        if max_percent >= warning {
            warn!("Usage at or above warning threshold ({}%)", warning);
            return (
                true,
                Duration::from_secs(u64::from(self.backoff.min_seconds)),
            );
        }

        if max_percent <= u32::from(self.thresholds.resume) {
            // Reset usage stats if we're below resume threshold
            self.reset_usage_stats();
        }

        // Default backoff
        (
            true,
            Duration::from_secs(u64::from(self.backoff.min_seconds)),
        )
    }

    /// Update usage statistics with new API response data
    pub fn update_usage(&mut self, requests: u32, tokens: u32, input_tokens: u32) {
        self.usage.requests_used = requests;
        self.usage.tokens_used = tokens;
        self.usage.input_tokens_used = input_tokens;
        self.usage.last_check = Instant::now();
    }

    /// Reset usage statistics
    pub fn reset_usage_stats(&mut self) {
        self.usage = UsageStats::default();
    }

    /// Calculate usage percentage
    #[allow(clippy::cast_possible_truncation, clippy::cast_lossless)]
    const fn calculate_usage_percent(used: u32, limit: u32) -> u32 {
        if limit == 0 {
            return 0;
        }
        // Use u64 for intermediate calculation to avoid overflow
        let used = used as u64;
        let limit = limit as u64;

        // Calculate percentage with high precision
        let percent = (used * 100) / limit;
        let remainder = (used * 100) % limit;

        // Round up if we're more than halfway to the next percentage point
        // This ensures that when we're exactly at a threshold (e.g. 75%),
        // we round up and consider it as exceeding the threshold
        if remainder >= limit / 2 {
            (percent + 1) as u32
        } else {
            percent as u32
        }
    }

    /// Get appropriate backoff duration based on current usage
    fn get_backoff_duration(&self) -> Duration {
        let max_percent = Self::calculate_usage_percent(
            self.usage.requests_used,
            self.limits.requests_per_minute.unwrap_or(0),
        )
        .max(Self::calculate_usage_percent(
            self.usage.tokens_used,
            self.limits.tokens_per_minute.unwrap_or(0),
        ))
        .max(Self::calculate_usage_percent(
            self.usage.input_tokens_used,
            self.limits.input_tokens_per_minute.unwrap_or(0),
        ));

        if max_percent >= u32::from(self.thresholds.critical) {
            Duration::from_secs(u64::from(self.backoff.max_seconds))
        } else {
            // If we're at warning threshold or below, use min_seconds
            Duration::from_secs(u64::from(self.backoff.min_seconds))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test fixtures
    fn create_test_limiter() -> RateLimiter {
        RateLimiter::new(
            RateLimits {
                requests_per_minute: Some(100),
                tokens_per_minute: Some(1000),
                input_tokens_per_minute: Some(500),
            },
            Thresholds {
                warning: 30,
                critical: 50,
                resume: 25,
            },
            BackoffConfig {
                min_seconds: 5,
                max_seconds: 60,
            },
        )
    }

    #[test]
    fn test_usage_stats_default() {
        let stats = UsageStats::default();
        assert_eq!(stats.requests_used, 0);
        assert_eq!(stats.tokens_used, 0);
        assert_eq!(stats.input_tokens_used, 0);
    }

    #[test]
    fn test_rate_limiter_new() {
        let limiter = create_test_limiter();
        assert_eq!(limiter.limits.requests_per_minute, Some(100));
        assert_eq!(limiter.limits.tokens_per_minute, Some(1000));
        assert_eq!(limiter.limits.input_tokens_per_minute, Some(500));
    }

    #[test]
    fn test_calculate_usage_percent() {
        assert_eq!(RateLimiter::calculate_usage_percent(50, 100), 50);
        assert_eq!(RateLimiter::calculate_usage_percent(0, 100), 0);
        assert_eq!(RateLimiter::calculate_usage_percent(100, 100), 100);
        assert_eq!(RateLimiter::calculate_usage_percent(25, 100), 25);
        assert_eq!(RateLimiter::calculate_usage_percent(10, 0), 0);
    }

    #[test]
    fn test_usage_calculation() {
        let mut limiter = create_test_limiter();
        limiter.update_usage(50, 500, 250);
        assert_eq!(limiter.usage.requests_used, 50);
        assert_eq!(limiter.usage.tokens_used, 500);
        assert_eq!(limiter.usage.input_tokens_used, 250);
    }

    #[test]
    fn test_check_limits() {
        let mut limiter = create_test_limiter();

        // Test normal usage
        limiter.update_usage(10, 100, 50);
        let (can_proceed, backoff) = limiter.check_limits();
        assert!(can_proceed);
        assert_eq!(backoff, Duration::from_secs(5));

        // Test warning threshold
        limiter.update_usage(35, 350, 175);
        let (can_proceed, backoff) = limiter.check_limits();
        assert!(can_proceed);
        assert_eq!(backoff, Duration::from_secs(5));

        // Test critical threshold
        limiter.update_usage(60, 600, 300);
        let (can_proceed, backoff) = limiter.check_limits();
        assert!(!can_proceed);
        assert_eq!(backoff, Duration::from_secs(60));
    }
}
