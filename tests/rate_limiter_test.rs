use std::thread;
use std::time::Duration;

use strainer::{
    config::{BackoffConfig, RateLimits, Thresholds},
    rate_limiter::RateLimiter,
};

// Test fixtures
fn create_test_rate_limiter() -> RateLimiter {
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
fn test_initial_rate_limiter_state() {
    let mut limiter = create_test_rate_limiter();
    let (can_proceed, backoff) = limiter.check_limits();

    // Initial state should allow proceeding with minimum backoff
    assert!(can_proceed);
    assert_eq!(backoff, Duration::from_secs(5));
}

#[test]
fn test_rate_limiter_update_usage() {
    let mut limiter = create_test_rate_limiter();

    // Test below warning threshold (30%)
    limiter.update_usage(25, 250, 125);
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(can_proceed);
    assert_eq!(backoff, Duration::from_secs(5));

    // Test at warning threshold (30%)
    limiter.update_usage(30, 300, 150);
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(can_proceed);
    assert_eq!(backoff, Duration::from_secs(5));

    // Test above warning but below critical (40%)
    limiter.update_usage(40, 400, 200);
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(can_proceed);
    assert_eq!(backoff, Duration::from_secs(5));

    // Test at critical threshold (50%)
    limiter.update_usage(50, 500, 250);
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(!can_proceed);
    assert_eq!(backoff, Duration::from_secs(60));

    // Test above critical threshold (60%)
    limiter.update_usage(60, 600, 300);
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(!can_proceed);
    assert_eq!(backoff, Duration::from_secs(60));
}

#[test]
fn test_rate_limiter_resume_threshold() {
    let mut limiter = create_test_rate_limiter();

    // First go above critical threshold
    limiter.update_usage(60, 600, 300);
    let (can_proceed, _) = limiter.check_limits();
    assert!(!can_proceed);

    // Then drop below resume threshold (25%)
    limiter.update_usage(20, 200, 100);
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(can_proceed);
    assert_eq!(backoff, Duration::from_secs(5));
}

#[test]
fn test_rate_limiter_time_based_limiting() {
    let mut limiter = create_test_rate_limiter();

    // Initial state
    let (can_proceed, _) = limiter.check_limits();
    assert!(can_proceed);

    // Simulate high usage
    limiter.update_usage(80, 800, 400);
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(!can_proceed);
    assert_eq!(backoff, Duration::from_secs(60));

    // Wait for a short time
    thread::sleep(Duration::from_millis(100));

    // Usage should still be high
    let (can_proceed, _) = limiter.check_limits();
    assert!(!can_proceed);

    // Reset usage
    limiter.reset_usage_stats();
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(can_proceed);
    assert_eq!(backoff, Duration::from_secs(5));
}

#[test]
fn test_rate_limiter_zero_limits() {
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
    );

    // Even with high usage, should proceed if limits are None
    limiter.update_usage(1000, 10000, 5000);
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(can_proceed);
    assert_eq!(backoff, Duration::from_secs(5));
}

#[test]
fn test_rate_limiter_mixed_usage() {
    let mut limiter = create_test_rate_limiter();

    // Test mixed usage where only one metric is above critical
    limiter.update_usage(60, 200, 100); // Requests at 60%, others at 20%
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(!can_proceed); // Should not proceed due to requests being over critical
    assert_eq!(backoff, Duration::from_secs(60));

    // Test mixed usage where one is at warning, others below
    limiter.update_usage(30, 100, 50); // Requests at 30%, others at 10%
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(can_proceed);
    assert_eq!(backoff, Duration::from_secs(5));
}

#[test]
fn test_rate_limiter_edge_cases() {
    let mut limiter = create_test_rate_limiter();

    // Test 0% usage
    limiter.update_usage(0, 0, 0);
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(can_proceed);
    assert_eq!(backoff, Duration::from_secs(5));

    // Test 100% usage
    limiter.update_usage(100, 1000, 500);
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(!can_proceed);
    assert_eq!(backoff, Duration::from_secs(60));

    // Test usage above 100%
    limiter.update_usage(150, 1500, 750);
    let (can_proceed, backoff) = limiter.check_limits();
    assert!(!can_proceed);
    assert_eq!(backoff, Duration::from_secs(60));
}

#[test]
fn test_rate_limiter_concurrent_usage() {
    let mut limiter = create_test_rate_limiter();

    // Simulate rapid concurrent requests
    for i in 0..5 {
        limiter.update_usage(20 * (i + 1), 200 * (i + 1), 100 * (i + 1));
        let (can_proceed, backoff) = limiter.check_limits();

        if i < 2 {
            assert!(can_proceed);
            assert_eq!(backoff, Duration::from_secs(5));
        } else {
            assert!(!can_proceed);
            assert_eq!(backoff, Duration::from_secs(60));
        }

        // Small delay to simulate real-world concurrent requests
        thread::sleep(Duration::from_millis(10));
    }
}
