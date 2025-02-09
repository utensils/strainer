use strainer::{BackoffConfig, RateLimits, Thresholds};

// Basic tests that don't require mocking
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
        min_seconds: 5,
        max_seconds: 60,
    };

    assert!(backoff.min_seconds < backoff.max_seconds);
}

// Extended tests that require mocking
#[cfg(feature = "testing")]
mod integration_tests {
    use super::*;
    use strainer::{test_utils::MockProvider, RateLimiter};

    #[test]
    fn test_rate_limiter_integration() {
        let provider = MockProvider::new();
        let mock_provider = provider.as_any().downcast_ref::<MockProvider>().unwrap();

        // Set up test response
        mock_provider.set_response(strainer::RateLimitInfo {
            requests_used: 10,
            tokens_used: 100,
            input_tokens_used: 50,
        });

        let mut limiter = RateLimiter::new(
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
        );

        // Test limits
        assert!(limiter.check_limits().is_ok());
    }
}
