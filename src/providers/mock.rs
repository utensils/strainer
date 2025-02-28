use crate::config::ApiConfig;
use crate::providers::config::MockConfig;
use crate::providers::{Provider, RateLimitInfo, RateLimitsConfig};
use anyhow::Result;

/// Mock provider for testing
#[derive(Debug)]
pub struct MockProvider {
    pub requests_used: u32,
    pub tokens_used: u32,
    pub input_tokens_used: u32,
    #[allow(dead_code)]
    config: MockConfig,
}

impl MockProvider {
    /// Create a new mock provider with initial usage values
    ///
    /// # Errors
    ///
    /// This implementation never returns an error, but the Result type is used
    /// to maintain consistency with the Provider trait requirements.
    pub fn new(config: &ApiConfig) -> Result<Self> {
        let provider_config = match &config.provider_config {
            crate::providers::config::ProviderConfig::Mock(cfg) => cfg.clone(),
            _ => return Err(anyhow::anyhow!("Invalid provider configuration")),
        };

        Ok(Self {
            requests_used: 0,
            tokens_used: 0,
            input_tokens_used: 0,
            config: provider_config,
        })
    }

    /// Set the usage values for testing
    pub fn set_usage(&mut self, requests: u32, tokens: u32, input_tokens: u32) {
        self.requests_used = requests;
        self.tokens_used = tokens;
        self.input_tokens_used = input_tokens;
    }
}

impl Provider for MockProvider {
    fn get_rate_limits(&self) -> Result<RateLimitInfo> {
        Ok(RateLimitInfo {
            requests_used: self.requests_used,
            tokens_used: self.tokens_used,
            input_tokens_used: self.input_tokens_used,
        })
    }

    fn get_rate_limits_config(&self) -> Result<RateLimitsConfig> {
        Ok(RateLimitsConfig {
            requests_per_minute: Some(self.config.requests_per_minute),
            tokens_per_minute: Some(self.config.tokens_per_minute),
            input_tokens_per_minute: Some(self.config.input_tokens_per_minute),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::config::ProviderConfig;
    use std::collections::HashMap;

    #[test]
    fn test_mock_provider_new() {
        let config = ApiConfig {
            provider_config: ProviderConfig::Mock(MockConfig::default()),
            api_key: None,
            base_url: None,
            parameters: HashMap::default(),
        };
        let provider = MockProvider::new(&config).unwrap();
        assert_eq!(provider.requests_used, 0);
        assert_eq!(provider.tokens_used, 0);
        assert_eq!(provider.input_tokens_used, 0);
    }

    #[test]
    fn test_mock_provider_invalid_config() {
        let config = ApiConfig {
            provider_config: ProviderConfig::Anthropic(
                crate::providers::config::AnthropicConfig::default(),
            ),
            api_key: Some("test_key".to_string()),
            base_url: None,
            parameters: HashMap::default(),
        };
        let provider = MockProvider::new(&config);
        assert!(provider.is_err());
        assert_eq!(
            provider.unwrap_err().to_string(),
            "Invalid provider configuration"
        );
    }

    #[test]
    fn test_mock_provider_set_usage() {
        let config = ApiConfig {
            provider_config: ProviderConfig::Mock(MockConfig::default()),
            api_key: None,
            base_url: None,
            parameters: HashMap::default(),
        };
        let mut provider = MockProvider::new(&config).unwrap();
        provider.set_usage(10, 100, 50);
        assert_eq!(provider.requests_used, 10);
        assert_eq!(provider.tokens_used, 100);
        assert_eq!(provider.input_tokens_used, 50);
    }

    #[test]
    fn test_mock_provider_get_rate_limits() {
        let config = ApiConfig {
            provider_config: ProviderConfig::Mock(MockConfig::default()),
            api_key: None,
            base_url: None,
            parameters: HashMap::default(),
        };
        let mut provider = MockProvider::new(&config).unwrap();
        provider.set_usage(10, 100, 50);
        let limits = provider.get_rate_limits().unwrap();
        assert_eq!(limits.requests_used, 10);
        assert_eq!(limits.tokens_used, 100);
        assert_eq!(limits.input_tokens_used, 50);
    }

    #[test]
    fn test_mock_provider_as_any() {
        let config = ApiConfig {
            provider_config: ProviderConfig::Mock(MockConfig::default()),
            api_key: None,
            base_url: None,
            parameters: HashMap::default(),
        };
        let provider = MockProvider::new(&config).unwrap();
        let _: &MockProvider = provider.as_any().downcast_ref().unwrap();
    }
}
