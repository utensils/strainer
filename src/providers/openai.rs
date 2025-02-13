use crate::config::ApiConfig;
use crate::providers::config::OpenAIConfig;
use crate::providers::{Provider, RateLimitInfo, RateLimitsConfig};
use anyhow::Result;

#[derive(Debug)]
pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    config: OpenAIConfig,
    requests_used: u32,
    tokens_used: u32,
    input_tokens_used: u32,
}

impl OpenAIProvider {
    pub fn new(config: &ApiConfig) -> Result<Self> {
        let api_key = config.api_key.clone().ok_or_else(|| {
            anyhow::anyhow!("API key is required for OpenAI provider")
        })?;

        let base_url = config.base_url.clone().unwrap_or_else(|| {
            "https://api.openai.com/v1".to_string()
        });

        let provider_config = match &config.provider_config {
            crate::providers::config::ProviderConfig::OpenAI(cfg) => cfg.clone(),
            _ => return Err(anyhow::anyhow!("Invalid provider configuration")),
        };

        Ok(Self {
            api_key,
            base_url,
            config: provider_config,
            requests_used: 0,
            tokens_used: 0,
            input_tokens_used: 0,
        })
    }
}

impl Provider for OpenAIProvider {
    fn get_rate_limits(&self) -> Result<RateLimitInfo> {
        Ok(RateLimitInfo {
            requests_used: self.requests_used,
            tokens_used: self.tokens_used,
            input_tokens_used: self.input_tokens_used,
        })
    }

    fn get_rate_limits_config(&self) -> Result<RateLimitsConfig> {
        Ok(RateLimitsConfig {
            requests_per_minute: Some(3500),  // OpenAI's default rate limit
            tokens_per_minute: Some(90000),   // OpenAI's default token limit
            input_tokens_per_minute: Some(45000), // OpenAI's default input token limit
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
} 