use crate::config::ApiConfig;
use anyhow::Result;

/// Trait defining the interface for rate limit providers
pub trait Provider: std::fmt::Debug + std::any::Any + Send + Sync {
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

    fn as_any(&self) -> &dyn std::any::Any;
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
        "mock" => Ok(Box::new(mock::MockProvider::new(config)?)),
        _ => Err(anyhow::anyhow!("Unsupported provider: {}", config.provider)),
    }
}

pub mod anthropic;
pub mod mock;
pub mod rate_limiter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_anthropic_provider() {
        let config = ApiConfig {
            provider: String::from("anthropic"),
            api_key: Some("test_key".to_string()),
            ..Default::default()
        };
        let provider = create_provider(&config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_create_unsupported_provider() {
        let config = ApiConfig {
            provider: String::from("unsupported"),
            ..Default::default()
        };
        let provider = create_provider(&config);
        assert!(provider.is_err());
        assert_eq!(
            provider.unwrap_err().to_string(),
            "Unsupported provider: unsupported"
        );
    }

    #[test]
    fn test_rate_limit_info_debug() {
        let info = RateLimitInfo {
            requests_used: 10,
            tokens_used: 100,
            input_tokens_used: 50,
        };
        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("requests_used: 10"));
        assert!(debug_str.contains("tokens_used: 100"));
        assert!(debug_str.contains("input_tokens_used: 50"));
    }
}
