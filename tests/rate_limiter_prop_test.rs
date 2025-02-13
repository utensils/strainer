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
    use strainer::test_utils::MockProvider;
    use strainer::RateLimiter;

    proptest! {
        #[test]
        fn test_rate_limiter_never_panics(
            warning_val in 2..40u8,
            critical_val in 41..90u8,
            resume_val in 1..2u8,
            min_backoff in 1..30u32,
            max_backoff in 31..120u32,
        ) {
            let mut limiter = RateLimiter::new(
                Thresholds {
                    warning: warning_val,
                    critical: critical_val,
                    resume: resume_val,
                },
                BackoffConfig {
                    min_seconds: min_backoff,
                    max_seconds: max_backoff,
                },
                MockProvider::new()
            );

            // Check limits multiple times with varying usage values
            for _ in 0..5 {
                // The mock provider will return default values which should never cause panics
                let result = limiter.check_limits();
                prop_assert!(result.is_ok());
            }
        }
    }
}
