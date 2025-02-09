use strainer::{RateLimiter, RateLimits, Thresholds, BackoffConfig};

#[cfg(feature = "testing")]
mod tests {
    use super::*;
    use strainer::test_utils::MockProvider;
    
    #[test]
    fn test_rate_limiter_integration() {
        let provider = MockProvider::new();
        
        // Set up test response
        provider.set_response(strainer::providers::RateLimitInfo {
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
        
        // Verify the limiter was created successfully
        assert!(limiter.check_limits().is_ok());
    }
}