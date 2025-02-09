use crate::config::ApiConfig;
use anyhow::Result;

/// Trait defining the interface for rate limit providers
pub trait Provider {
    /// Get the current rate limits from the provider
    /// Get the current rate limits for the API provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unable to fetch rate limit information from the API
    /// - Network connectivity issues
    /// - Invalid API response format
    fn get_rate_limits(&self) -> Result<RateLimitInfo>;
}

/// Represents rate limit information from a provider
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub requests_used: u32,
    pub tokens_used: u32,
    pub input_tokens_used: u32,
}

/// Create a new provider based on the configuration
/// Creates a new API provider based on the given configuration
///
/// # Errors
///
/// Returns an error if:
/// - Unknown provider type specified in config
/// - Invalid configuration parameters
/// - Provider initialization fails
pub fn create_provider(config: &ApiConfig) -> Result<Box<dyn Provider>> {
    match config.provider.as_str() {
        "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new(config)?)),
        _ => Err(anyhow::anyhow!("Unsupported provider: {}", config.provider)),
    }
}

pub mod anthropic;
pub mod rate_limiter;
