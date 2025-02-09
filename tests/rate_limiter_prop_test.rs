use proptest::prelude::*;
use strainer::config::{BackoffConfig, Thresholds};

// Basic property tests that don't require mocking
proptest! {
    #[test]
    fn test_config_validation(
        // Ensure warning is always greater than resume
        warning_val in 2..40u8,
        critical_val in 41..90u8,
        resume_val in 1..2u8,
        min_backoff in 1..30u32,
        max_backoff in 31..120u32,
    ) {
        let config = Thresholds {
            warning: warning_val,
            critical: critical_val,
            resume: resume_val,
        };

        // Test that threshold values are in valid ranges
        prop_assert!(config.warning < config.critical);
        prop_assert!(config.resume < config.warning);

        let backoff = BackoffConfig {
            min_seconds: min_backoff,
            max_seconds: max_backoff,
        };

        // Test that backoff values are in valid ranges
        prop_assert!(backoff.min_seconds < backoff.max_seconds);
    }
}

// Extended property tests that require mocking
#[cfg(feature = "testing")]
mod prop_tests {
    use super::*;
    use strainer::{providers::RateLimitInfo, test_utils::MockProvider, RateLimiter, RateLimits};

    proptest! {
        #[test]
        fn test_rate_limiter_never_panics(
            requests in 0..1000u32,
            tokens in 0..10000u32,
            input_tokens in 0..5000u32,
            warning_val in 2..40u8,
            critical_val in 41..90u8,
            resume_val in 1..2u8,
            min_backoff in 1..30u32,
            max_backoff in 31..120u32,
        ) {
            let provider = MockProvider::new();
            provider.set_response(RateLimitInfo {
                requests_used: requests,
                tokens_used: tokens,
                input_tokens_used: input_tokens,
            });

            let mut limiter = RateLimiter::new(
                RateLimits {
                    requests_per_minute: Some(100),
                    tokens_per_minute: Some(1000),
                    input_tokens_per_minute: Some(500),
                },
                Thresholds {
                    warning: warning_val,
                    critical: critical_val,
                    resume: resume_val,
                },
                BackoffConfig {
                    min_seconds: min_backoff,
                    max_seconds: max_backoff,
                },
                provider,
            );

            prop_assert!(limiter.check_limits().is_ok());
        }
    }
}
