use proptest::prelude::*;
use std::time::Duration;

use strainer::{
    config::{BackoffConfig, RateLimits, Thresholds},
    rate_limiter::RateLimiter,
};

// Helper function to calculate expected behavior
fn calculate_expected_behavior(
    usage_percent: u32,
    warning_threshold: u8,
    critical_threshold: u8,
    resume_threshold: u8,
) -> (bool, Duration, &'static str) {
    if usage_percent >= u32::from(critical_threshold) {
        (false, Duration::from_secs(60), "Above critical threshold")
    } else if usage_percent >= u32::from(warning_threshold) {
        (true, Duration::from_secs(5), "Above warning threshold")
    } else if usage_percent <= u32::from(resume_threshold) {
        (true, Duration::from_secs(5), "Below resume threshold")
    } else {
        (true, Duration::from_secs(5), "Normal operation")
    }
}

proptest! {
    // Test rate limiter behavior with random valid thresholds
    #[test]
    fn test_rate_limiter_thresholds(
        warning in 20u8..40u8,
        critical in 41u8..80u8,
        resume in 10u8..19u8,
        requests_limit in 100u32..10000u32,
        tokens_limit in 1000u32..100000u32,
        input_tokens_limit in 500u32..50000u32,
    ) {
        // Create rate limiter with generated thresholds
        let mut limiter = RateLimiter::new(
            RateLimits {
                requests_per_minute: Some(requests_limit),
                tokens_per_minute: Some(tokens_limit),
                input_tokens_per_minute: Some(input_tokens_limit),
            },
            Thresholds {
                warning,
                critical,
                resume,
            },
            BackoffConfig {
                min_seconds: 5,
                max_seconds: 60,
            },
        );

        // Test various usage percentages
        for usage_percent in &[0, 15, 25, 35, 45, 55, 75, 100] {
            let requests = (requests_limit * usage_percent) / 100;
            let tokens = (tokens_limit * usage_percent) / 100;
            let input_tokens = (input_tokens_limit * usage_percent) / 100;

            limiter.update_usage(requests, tokens, input_tokens);
            let (can_proceed, backoff) = limiter.check_limits();

            let (expected_proceed, expected_backoff, scenario) =
                calculate_expected_behavior(*usage_percent, warning, critical, resume);

            prop_assert_eq!(
                can_proceed,
                expected_proceed,
                "Failed for {}% usage in scenario: {}",
                usage_percent,
                scenario
            );
            prop_assert_eq!(
                backoff,
                expected_backoff,
                "Incorrect backoff for {}% usage in scenario: {}",
                usage_percent,
                scenario
            );
        }
    }

    // Test rate limiter with random usage values
    #[test]
    fn test_rate_limiter_random_usage(
        requests in 0u32..200u32,
        tokens in 0u32..2000u32,
        input_tokens in 0u32..1000u32,
    ) {
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
        );

        limiter.update_usage(requests, tokens, input_tokens);
        let (can_proceed, backoff) = limiter.check_limits();

        // Calculate highest usage percentage across all metrics
        let requests_percent = (requests * 100) / 100;
        let tokens_percent = (tokens * 100) / 1000;
        let input_tokens_percent = (input_tokens * 100) / 500;
        let max_percent = requests_percent.max(tokens_percent).max(input_tokens_percent);

        let (expected_proceed, expected_backoff, scenario) =
            calculate_expected_behavior(max_percent, 30, 50, 25);

        prop_assert_eq!(
            can_proceed,
            expected_proceed,
            "Failed for usage - Requests: {}, Tokens: {}, Input Tokens: {} in scenario: {}",
            requests,
            tokens,
            input_tokens,
            scenario
        );
        prop_assert_eq!(
            backoff,
            expected_backoff,
            "Incorrect backoff for usage - Requests: {}, Tokens: {}, Input Tokens: {} in scenario: {}",
            requests,
            tokens,
            input_tokens,
            scenario
        );
    }

    // Test rate limiter with edge cases
    #[test]
    fn test_rate_limiter_edge_cases(
        // Test with values very close to thresholds
        warning_usage in 29u32..31u32,
        critical_usage in 49u32..51u32,
        resume_usage in 24u32..26u32,
    ) {
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
        );

        // Test warning threshold edge
        let requests = warning_usage;
        let tokens = warning_usage * 10;
        let input_tokens = warning_usage * 5;
        limiter.update_usage(requests, tokens, input_tokens);
        let (can_proceed, backoff) = limiter.check_limits();
        let (expected_proceed, expected_backoff, _) =
            calculate_expected_behavior(warning_usage, 30, 50, 25);
        prop_assert_eq!(
            can_proceed,
            expected_proceed,
            "Failed at warning threshold edge: {}%",
            warning_usage
        );
        prop_assert_eq!(
            backoff,
            expected_backoff,
            "Incorrect backoff at warning threshold edge: {}%",
            warning_usage
        );

        // Test critical threshold edge
        let requests = critical_usage;
        let tokens = critical_usage * 10;
        let input_tokens = critical_usage * 5;
        limiter.update_usage(requests, tokens, input_tokens);
        let (can_proceed, backoff) = limiter.check_limits();
        let (expected_proceed, expected_backoff, _) =
            calculate_expected_behavior(critical_usage, 30, 50, 25);
        prop_assert_eq!(
            can_proceed,
            expected_proceed,
            "Failed at critical threshold edge: {}%",
            critical_usage
        );
        prop_assert_eq!(
            backoff,
            expected_backoff,
            "Incorrect backoff at critical threshold edge: {}%",
            critical_usage
        );

        // Test resume threshold edge
        let requests = resume_usage;
        let tokens = resume_usage * 10;
        let input_tokens = resume_usage * 5;
        limiter.update_usage(requests, tokens, input_tokens);
        let (can_proceed, backoff) = limiter.check_limits();
        let (expected_proceed, expected_backoff, _) =
            calculate_expected_behavior(resume_usage, 30, 50, 25);
        prop_assert_eq!(
            can_proceed,
            expected_proceed,
            "Failed at resume threshold edge: {}%",
            resume_usage
        );
        prop_assert_eq!(
            backoff,
            expected_backoff,
            "Incorrect backoff at resume threshold edge: {}%",
            resume_usage
        );
    }
}
