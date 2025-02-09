use super::Provider;
use crate::config::{BackoffConfig, RateLimits, Thresholds};
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
    limits: RateLimits,
    thresholds: Thresholds,
    backoff: BackoffConfig,
    usage: UsageStats,
    provider: Box<dyn Provider>,
}

impl RateLimiter {
    /// Create a new `RateLimiter` with the specified configuration
    #[must_use]
    pub fn new(
        limits: RateLimits,
        thresholds: Thresholds,
        backoff: BackoffConfig,
        provider: Box<dyn Provider>,
    ) -> Self {
        Self {
            limits,
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
        // Get current usage from provider
        let rate_info = self.provider.get_rate_limits()?;

        // If all limits are None, allow proceeding with minimum backoff
        if self.limits.requests_per_minute.is_none()
            && self.limits.tokens_per_minute.is_none()
            && self.limits.input_tokens_per_minute.is_none()
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
        let requests_percent = self.limits.requests_per_minute.map_or(0, |limit| {
            Self::calculate_usage_percent(self.usage.requests_used, limit)
        });

        let tokens_percent = self.limits.tokens_per_minute.map_or(0, |limit| {
            Self::calculate_usage_percent(self.usage.tokens_used, limit)
        });

        let input_tokens_percent = self.limits.input_tokens_per_minute.map_or(0, |limit| {
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
    use crate::providers::RateLimitInfo;
    use crate::test_utils::MockProvider;

    fn create_test_limiter() -> RateLimiter {
        let provider = MockProvider::new();
        let provider_ref = provider.as_ref();
        if let Some(mock_provider) = provider_ref.as_any().downcast_ref::<MockProvider>() {
            mock_provider.set_response(RateLimitInfo {
                requests_used: 0,
                tokens_used: 0,
                input_tokens_used: 0,
            });
        }

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
            provider,
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
    fn test_basic_thresholds() -> Result<()> {
        let mut limiter = create_test_limiter();

        // Test normal usage (10%)
        if let Some(mock_provider) = limiter
            .provider
            .as_ref()
            .as_any()
            .downcast_ref::<MockProvider>()
        {
            mock_provider.set_response(RateLimitInfo {
                requests_used: 10,
                tokens_used: 100,
                input_tokens_used: 50,
            });
        }
        let (can_proceed, backoff) = limiter.check_limits()?;
        assert!(can_proceed);
        assert_eq!(backoff, Duration::from_secs(5));

        // Test warning threshold (30%)
        if let Some(mock_provider) = limiter
            .provider
            .as_ref()
            .as_any()
            .downcast_ref::<MockProvider>()
        {
            mock_provider.set_response(RateLimitInfo {
                requests_used: 30,
                tokens_used: 300,
                input_tokens_used: 150,
            });
        }
        let (can_proceed, backoff) = limiter.check_limits()?;
        assert!(can_proceed);
        assert_eq!(backoff, Duration::from_secs(5));

        // Test critical threshold (50%)
        if let Some(mock_provider) = limiter
            .provider
            .as_ref()
            .as_any()
            .downcast_ref::<MockProvider>()
        {
            mock_provider.set_response(RateLimitInfo {
                requests_used: 50,
                tokens_used: 500,
                input_tokens_used: 250,
            });
        }
        let (can_proceed, backoff) = limiter.check_limits()?;
        assert!(!can_proceed);
        assert_eq!(backoff, Duration::from_secs(60));

        Ok(())
    }

    #[test]
    fn test_mixed_usage() -> Result<()> {
        let mut limiter = create_test_limiter();

        // Test where only requests are high (60%)
        if let Some(mock_provider) = limiter
            .provider
            .as_ref()
            .as_any()
            .downcast_ref::<MockProvider>()
        {
            mock_provider.set_response(RateLimitInfo {
                requests_used: 60,
                tokens_used: 200,
                input_tokens_used: 100,
            });
        }
        let (can_proceed, backoff) = limiter.check_limits()?;
        assert!(!can_proceed);
        assert_eq!(backoff, Duration::from_secs(60));

        // Test where only tokens are high
        if let Some(mock_provider) = limiter
            .provider
            .as_ref()
            .as_any()
            .downcast_ref::<MockProvider>()
        {
            mock_provider.set_response(RateLimitInfo {
                requests_used: 20,
                tokens_used: 800,
                input_tokens_used: 100,
            });
        }
        let (can_proceed, backoff) = limiter.check_limits()?;
        assert!(!can_proceed);
        assert_eq!(backoff, Duration::from_secs(60));

        Ok(())
    }

    #[test]
    fn test_no_limits() -> Result<()> {
        let provider = MockProvider::new();
        if let Some(mock_provider) = provider.as_ref().as_any().downcast_ref::<MockProvider>() {
            mock_provider.set_response(RateLimitInfo {
                requests_used: 1000,
                tokens_used: 10000,
                input_tokens_used: 5000,
            });
        }

        let mut limiter = RateLimiter::new(
            RateLimits {
                requests_per_minute: None,
                tokens_per_minute: None,
                input_tokens_per_minute: None,
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
            provider,
        );

        // Should always proceed with minimum backoff when no limits are set
        let (can_proceed, backoff) = limiter.check_limits()?;
        assert!(can_proceed);
        assert_eq!(backoff, Duration::from_secs(5));
        Ok(())
    }

    #[test]
    fn test_resume_threshold() -> Result<()> {
        let mut limiter = create_test_limiter();

        // Start at critical
        if let Some(mock_provider) = limiter
            .provider
            .as_ref()
            .as_any()
            .downcast_ref::<MockProvider>()
        {
            mock_provider.set_response(RateLimitInfo {
                requests_used: 60,
                tokens_used: 600,
                input_tokens_used: 300,
            });
        }
        let (can_proceed, _) = limiter.check_limits()?;
        assert!(!can_proceed);

        // Drop below resume threshold
        if let Some(mock_provider) = limiter
            .provider
            .as_ref()
            .as_any()
            .downcast_ref::<MockProvider>()
        {
            mock_provider.set_response(RateLimitInfo {
                requests_used: 20,
                tokens_used: 200,
                input_tokens_used: 100,
            });
        }
        let (can_proceed, backoff) = limiter.check_limits()?;
        assert!(can_proceed);
        assert_eq!(backoff, Duration::from_secs(5));

        Ok(())
    }
}
