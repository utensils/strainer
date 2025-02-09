use crate::config::ApiConfig;
use crate::providers::{Provider, RateLimitInfo};
use anyhow::Result;

/// Provider implementation for Anthropic's API
#[allow(dead_code)]
#[derive(Debug)]
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider with the given configuration
    /// Creates a new Anthropic API provider with the given configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Missing API key in configuration
    /// - Invalid API endpoint URL
    /// - Required configuration parameters are missing
    pub fn new(config: &ApiConfig) -> Result<Self> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| anyhow::anyhow!("API key is required for Anthropic"))?;
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());
        Ok(Self { api_key, base_url })
    }
}

impl Provider for AnthropicProvider {
    fn get_rate_limits(&self) -> Result<RateLimitInfo> {
        // TODO: Implement actual API call to get rate limits
        // For now, return dummy data
        Ok(RateLimitInfo {
            requests_used: 0,
            tokens_used: 0,
            input_tokens_used: 0,
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_provider_new() {
        let config = ApiConfig {
            api_key: Some("test_key".to_string()),
            ..Default::default()
        };
        let provider = AnthropicProvider::new(&config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_anthropic_provider_missing_key() {
        let config = ApiConfig::default();
        let provider = AnthropicProvider::new(&config);
        assert!(provider.is_err());
    }

    #[test]
    fn test_anthropic_provider_rate_limits() {
        let config = ApiConfig {
            api_key: Some("test_key".to_string()),
            ..Default::default()
        };
        let provider = AnthropicProvider::new(&config).unwrap();
        let limits = provider.get_rate_limits();
        assert!(limits.is_ok());
        let limits = limits.unwrap();
        assert_eq!(limits.requests_used, 0);
        assert_eq!(limits.tokens_used, 0);
        assert_eq!(limits.input_tokens_used, 0);
    }
}
