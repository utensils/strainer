pub mod cli;
pub mod config;
pub mod init;
pub mod process;
pub mod providers;

// Re-export key types for convenience
pub use config::{BackoffConfig, Config, RateLimits, Thresholds};
pub use init::{initialize_config, InitOptions};
pub use providers::rate_limiter::RateLimiter;
pub use providers::{Provider, RateLimitInfo};

// Test utilities module - only compiled with test or testing feature
#[cfg(any(test, feature = "testing"))]
pub mod test_utils;
