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
    use strainer::config::ApiConfig;
    use strainer::providers::mock::MockProvider;
    use strainer::RateLimiter;

    proptest! {
        #[test]
        fn test_rate_limiter_never_panics(
            _requests in 0..1000u32,
            _tokens in 0..10000u32,
            _input_tokens in 0..5000u32,
            warning_val in 2..40u8,
            critical_val in 41..90u8,
            resume_val in 1..2u8,
            min_backoff in 1..30u32,
            max_backoff in 31..120u32,
        ) {
            let config = ApiConfig::default();
            let provider = MockProvider::new(&config).unwrap();

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
                Box::new(provider)
            );

            prop_assert!(limiter.check_limits().is_ok());
        }
    }
}
