use crate::config::ApiConfig;
use anyhow::Result;

pub mod anthropic;
pub mod config;
pub mod mock;
pub mod rate_limiter;

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
    match &config.provider_config {
        config::ProviderConfig::Anthropic(_) => {
            Ok(Box::new(anthropic::AnthropicProvider::new(config)?))
        }
        config::ProviderConfig::OpenAI(_) => {
            Err(anyhow::anyhow!("OpenAI provider not yet implemented"))
        }
        config::ProviderConfig::Mock(_) => Ok(Box::new(mock::MockProvider::new(config)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::config::{AnthropicConfig, ProviderConfig};

    #[test]
    fn test_create_anthropic_provider() {
        let config = ApiConfig {
            provider_config: ProviderConfig::Anthropic(AnthropicConfig::default()),
            api_key: Some("test_key".to_string()),
            base_url: None,
        };
        let provider = create_provider(&config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_create_unsupported_provider() {
        let config = ApiConfig {
            provider_config: ProviderConfig::OpenAI(config::OpenAIConfig::default()),
            api_key: Some("test_key".to_string()),
            base_url: None,
        };
        let provider = create_provider(&config);
        assert!(provider.is_err());
        assert_eq!(
            provider.unwrap_err().to_string(),
            "OpenAI provider not yet implemented"
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
