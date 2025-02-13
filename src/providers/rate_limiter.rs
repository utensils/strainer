use super::Provider;
use crate::config::{BackoffConfig, Thresholds};
use anyhow::Result;
use std::time::{Duration, Instant};
use tracing::{info, warn};

#[derive(Debug)]
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

impl UsageStats {
    fn new(requests: u32, tokens: u32, input_tokens: u32) -> Self {
        Self {
            requests_used: requests,
            tokens_used: tokens,
            input_tokens_used: input_tokens,
            last_check: Instant::now(),
        }
    }
}

/// `RateLimiter` manages API rate limits with thresholds for warning and critical levels
#[derive(Debug)]
pub struct RateLimiter {
    thresholds: Thresholds,
    backoff: BackoffConfig,
    usage: UsageStats,
    provider: Box<dyn Provider>,
}

impl RateLimiter {
    /// Create a new `RateLimiter` with the specified configuration
    #[must_use]
    pub fn new(
        thresholds: Thresholds,
        backoff: BackoffConfig,
        provider: Box<dyn Provider>,
    ) -> Self {
        Self {
            thresholds,
            backoff,
            usage: UsageStats::default(),
            provider,
        }
    }

    /// Calculate the usage percentage, with proper handling of edge cases
    #[allow(clippy::cast_possible_truncation, clippy::cast_lossless)]
    #[must_use]
    pub fn calculate_usage_percent(used: u32, limit: u32) -> u32 {
        if limit == 0 {
            return 0;
        }
        // Use u64 for intermediate calculation to avoid overflow
        let used = u64::from(used);
        let limit = u64::from(limit);

        // Calculate percentage with high precision
        let percent = (used * 100) / limit;
        percent as u32
    }

    /// Check if any rate limits are exceeded and get appropriate backoff time
    /// Check if the current usage is within configured limits
    ///
    /// # Returns
    ///
    /// Returns a tuple of (bool, Duration) where:
    /// - bool indicates if the operation should proceed
    /// - Duration indicates how long to wait before retrying if needed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unable to fetch current rate limit information
    /// - Rate limit data is invalid or corrupted
    /// - Provider communication fails
    pub fn check_limits(&mut self) -> Result<(bool, Duration)> {
        // Get current usage and limits from provider
        let rate_info = self.provider.get_rate_limits()?;
        let rate_config = self.provider.get_rate_limits_config()?;

        // If all limits are None, allow proceeding with minimum backoff
        if rate_config.requests_per_minute.is_none()
            && rate_config.tokens_per_minute.is_none()
            && rate_config.input_tokens_per_minute.is_none()
        {
            return Ok((
                true,
                Duration::from_secs(u64::from(self.backoff.min_seconds)),
            ));
        }

        // Update internal usage stats
        self.usage = UsageStats::new(
            rate_info.requests_used,
            rate_info.tokens_used,
            rate_info.input_tokens_used,
        );

        // Calculate percentages for each limit type
        let requests_percent = rate_config.requests_per_minute.map_or(0, |limit| {
            Self::calculate_usage_percent(self.usage.requests_used, limit)
        });

        let tokens_percent = rate_config.tokens_per_minute.map_or(0, |limit| {
            Self::calculate_usage_percent(self.usage.tokens_used, limit)
        });

        let input_tokens_percent = rate_config.input_tokens_per_minute.map_or(0, |limit| {
            Self::calculate_usage_percent(self.usage.input_tokens_used, limit)
        });

        // Log current usage
        info!(
            "Rate limit status - Requests: {}%, Tokens: {}%, Input Tokens: {}%",
            requests_percent, tokens_percent, input_tokens_percent
        );

        // Find the highest usage percentage
        let max_percent = requests_percent
            .max(tokens_percent)
            .max(input_tokens_percent);

        // Convert thresholds to u32 for comparison
        let critical = u32::from(self.thresholds.critical);
        let warning = u32::from(self.thresholds.warning);
        let resume = u32::from(self.thresholds.resume);

        // Check thresholds in priority order
        if max_percent >= critical {
            warn!("Usage at or above critical threshold ({}%)", critical);
            Ok((
                false,
                Duration::from_secs(u64::from(self.backoff.max_seconds)),
            ))
        } else if max_percent >= warning {
            warn!("Usage at or above warning threshold ({}%)", warning);
            Ok((
                true,
                Duration::from_secs(u64::from(self.backoff.min_seconds)),
            ))
        } else if max_percent <= resume {
            // Reset usage stats when below resume threshold
            self.usage = UsageStats::default();
            Ok((
                true,
                Duration::from_secs(u64::from(self.backoff.min_seconds)),
            ))
        } else {
            // Normal operation
            Ok((
                true,
                Duration::from_secs(u64::from(self.backoff.min_seconds)),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RateLimits;
    use crate::providers::{RateLimitInfo, RateLimitsConfig};
    use std::sync::atomic::{AtomicU32, Ordering};

    // Basic validation tests
    #[test]
    fn test_rate_limits_validation() {
        let limits = RateLimits {
            requests_per_minute: Some(100),
            tokens_per_minute: Some(1000),
            input_tokens_per_minute: Some(500),
        };

        assert!(limits.requests_per_minute.unwrap() > 0);
        assert!(limits.tokens_per_minute.unwrap() > 0);
        assert!(limits.input_tokens_per_minute.unwrap() > 0);
    }

    #[test]
    fn test_thresholds_validation() {
        let thresholds = Thresholds {
            warning: 30,
            critical: 50,
            resume: 25,
        };

        assert!(thresholds.warning < thresholds.critical);
        assert!(thresholds.resume < thresholds.warning);
    }

    #[test]
    fn test_backoff_validation() {
        let backoff = BackoffConfig {
            min_seconds: 1,
            max_seconds: 5,
        };

        assert!(backoff.min_seconds < backoff.max_seconds);
    }

    fn create_test_limiter() -> RateLimiter {
        let thresholds = Thresholds {
            warning: 30,
            critical: 50,
            resume: 25,
        };

        let backoff = BackoffConfig {
            min_seconds: 1,
            max_seconds: 5,
        };

        RateLimiter::new(thresholds, backoff, Box::new(TestMockProvider::new()))
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
        assert_eq!(limiter.thresholds.warning, 30);
        assert_eq!(limiter.thresholds.critical, 50);
        assert_eq!(limiter.thresholds.resume, 25);
        assert_eq!(limiter.backoff.min_seconds, 1);
        assert_eq!(limiter.backoff.max_seconds, 5);
    }

    #[test]
    fn test_calculate_usage_percent() {
        assert_eq!(RateLimiter::calculate_usage_percent(50, 100), 50);
        assert_eq!(RateLimiter::calculate_usage_percent(0, 100), 0);
        assert_eq!(RateLimiter::calculate_usage_percent(100, 100), 100);
        assert_eq!(RateLimiter::calculate_usage_percent(200, 100), 200);
    }

    #[test]
    fn test_basic_thresholds() -> Result<()> {
        let mut limiter = create_test_limiter();

        // Test below warning threshold
        {
            let mock_provider = limiter
                .provider
                .as_any()
                .downcast_ref::<TestMockProvider>()
                .unwrap();
            mock_provider.requests_used.store(10, Ordering::Relaxed);
            mock_provider.tokens_used.store(100, Ordering::Relaxed);
            mock_provider.input_tokens_used.store(50, Ordering::Relaxed);
        }

        let (proceed, _) = limiter.check_limits()?;
        assert!(proceed, "Should proceed when below warning threshold");

        // Test at warning threshold
        {
            let mock_provider = limiter
                .provider
                .as_any()
                .downcast_ref::<TestMockProvider>()
                .unwrap();
            mock_provider.requests_used.store(30, Ordering::Relaxed);
            mock_provider.tokens_used.store(300, Ordering::Relaxed);
            mock_provider
                .input_tokens_used
                .store(150, Ordering::Relaxed);
        }

        let (proceed, _) = limiter.check_limits()?;
        assert!(proceed, "Should proceed at warning threshold");

        // Test at critical threshold
        {
            let mock_provider = limiter
                .provider
                .as_any()
                .downcast_ref::<TestMockProvider>()
                .unwrap();
            mock_provider.requests_used.store(50, Ordering::Relaxed);
            mock_provider.tokens_used.store(500, Ordering::Relaxed);
            mock_provider
                .input_tokens_used
                .store(250, Ordering::Relaxed);
        }

        let (proceed, _) = limiter.check_limits()?;
        assert!(!proceed, "Should not proceed at critical threshold");

        Ok(())
    }

    #[test]
    fn test_mixed_usage() -> Result<()> {
        let mut limiter = create_test_limiter();

        // Test with mixed usage levels
        {
            let mock_provider = limiter
                .provider
                .as_any()
                .downcast_ref::<TestMockProvider>()
                .unwrap();
            mock_provider.requests_used.store(10, Ordering::Relaxed); // Below warning
            mock_provider.tokens_used.store(400, Ordering::Relaxed); // Above warning
            mock_provider
                .input_tokens_used
                .store(600, Ordering::Relaxed); // Above critical
        }

        let (proceed, _) = limiter.check_limits()?;
        assert!(
            !proceed,
            "Should not proceed when any metric is above critical"
        );

        Ok(())
    }

    #[test]
    fn test_no_limits() -> Result<()> {
        let mut limiter = create_test_limiter();

        // Test with no limits set
        {
            let mock_provider = limiter
                .provider
                .as_any()
                .downcast_ref::<TestMockProvider>()
                .unwrap();
            mock_provider.set_limits(Some(0), Some(0), Some(0)); // Set all limits to 0 to disable them
            mock_provider.requests_used.store(1000, Ordering::Relaxed);
            mock_provider.tokens_used.store(10000, Ordering::Relaxed);
            mock_provider
                .input_tokens_used
                .store(5000, Ordering::Relaxed);
        }

        let (proceed, _) = limiter.check_limits()?;
        assert!(proceed, "Should proceed when no limits are set");

        Ok(())
    }

    #[test]
    fn test_resume_threshold() -> Result<()> {
        let mut limiter = create_test_limiter();

        // Start above critical
        {
            let mock_provider = limiter
                .provider
                .as_any()
                .downcast_ref::<TestMockProvider>()
                .unwrap();
            mock_provider.requests_used.store(60, Ordering::Relaxed);
            mock_provider.tokens_used.store(600, Ordering::Relaxed);
            mock_provider
                .input_tokens_used
                .store(300, Ordering::Relaxed);
        }

        let (proceed, _) = limiter.check_limits()?;
        assert!(!proceed, "Should not proceed above critical threshold");

        // Drop below resume threshold
        {
            let mock_provider = limiter
                .provider
                .as_any()
                .downcast_ref::<TestMockProvider>()
                .unwrap();
            mock_provider.requests_used.store(20, Ordering::Relaxed);
            mock_provider.tokens_used.store(200, Ordering::Relaxed);
            mock_provider
                .input_tokens_used
                .store(100, Ordering::Relaxed);
        }

        let (proceed, _) = limiter.check_limits()?;
        assert!(proceed, "Should proceed below resume threshold");

        Ok(())
    }

    #[derive(Debug)]
    struct TestMockProvider {
        requests_used: AtomicU32,
        tokens_used: AtomicU32,
        input_tokens_used: AtomicU32,
        requests_limit: AtomicU32,
        tokens_limit: AtomicU32,
        input_tokens_limit: AtomicU32,
    }

    impl TestMockProvider {
        const fn new() -> Self {
            Self {
                requests_used: AtomicU32::new(0),
                tokens_used: AtomicU32::new(0),
                input_tokens_used: AtomicU32::new(0),
                requests_limit: AtomicU32::new(100),
                tokens_limit: AtomicU32::new(1000),
                input_tokens_limit: AtomicU32::new(500),
            }
        }

        fn set_limits(
            &self,
            requests: Option<u32>,
            tokens: Option<u32>,
            input_tokens: Option<u32>,
        ) {
            if let Some(r) = requests {
                self.requests_limit.store(r, Ordering::Relaxed);
            }
            if let Some(t) = tokens {
                self.tokens_limit.store(t, Ordering::Relaxed);
            }
            if let Some(i) = input_tokens {
                self.input_tokens_limit.store(i, Ordering::Relaxed);
            }
        }
    }

    impl Provider for TestMockProvider {
        fn get_rate_limits(&self) -> Result<RateLimitInfo> {
            Ok(RateLimitInfo {
                requests_used: self.requests_used.load(Ordering::Relaxed),
                tokens_used: self.tokens_used.load(Ordering::Relaxed),
                input_tokens_used: self.input_tokens_used.load(Ordering::Relaxed),
            })
        }

        fn get_rate_limits_config(&self) -> Result<RateLimitsConfig> {
            let requests = self.requests_limit.load(Ordering::Relaxed);
            let tokens = self.tokens_limit.load(Ordering::Relaxed);
            let input_tokens = self.input_tokens_limit.load(Ordering::Relaxed);

            Ok(RateLimitsConfig {
                requests_per_minute: if requests > 0 { Some(requests) } else { None },
                tokens_per_minute: if tokens > 0 { Some(tokens) } else { None },
                input_tokens_per_minute: if input_tokens > 0 {
                    Some(input_tokens)
                } else {
                    None
                },
            })
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
}
