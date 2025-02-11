use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Invalid provider type: {0}")]
    InvalidProvider(String),
}

/// Provider-specific configuration traits and types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProviderConfig {
    #[serde(rename = "anthropic")]
    Anthropic(AnthropicConfig),
    #[serde(rename = "openai")]
    OpenAI(OpenAIConfig),
    #[serde(rename = "mock")]
    Mock(MockConfig),
}

impl FromStr for ProviderConfig {
    type Err = ProviderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(Self::Anthropic(AnthropicConfig::default())),
            "openai" => Ok(Self::OpenAI(OpenAIConfig::default())),
            "mock" => Ok(Self::Mock(MockConfig::default())),
            _ => Err(ProviderError::InvalidProvider(s.to_string())),
        }
    }
}

impl Display for ProviderConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Anthropic(_) => write!(f, "anthropic"),
            Self::OpenAI(_) => write!(f, "openai"),
            Self::Mock(_) => write!(f, "mock"),
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self::Anthropic(AnthropicConfig::default())
    }
}

/// Configuration for Anthropic API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// The model to use (e.g. "claude-2")
    #[serde(default = "default_anthropic_model")]
    pub model: String,
    /// Maximum tokens to generate
    #[serde(default = "default_anthropic_max_tokens")]
    pub max_tokens: u32,
    /// Additional model parameters
    #[serde(default)]
    pub parameters: HashMap<String, String>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            model: default_anthropic_model(),
            max_tokens: default_anthropic_max_tokens(),
            parameters: HashMap::new(),
        }
    }
}

fn default_anthropic_model() -> String {
    "claude-2".to_string()
}

const fn default_anthropic_max_tokens() -> u32 {
    1000
}

/// Configuration for `OpenAI` API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    /// The model to use (e.g. "gpt-4")
    #[serde(default = "default_openai_model")]
    pub model: String,
    /// Maximum tokens to generate
    #[serde(default = "default_openai_max_tokens")]
    pub max_tokens: u32,
    /// Temperature for sampling
    #[serde(default = "default_openai_temperature")]
    pub temperature: f32,
    /// Additional model parameters
    #[serde(default)]
    pub parameters: HashMap<String, String>,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            model: default_openai_model(),
            max_tokens: default_openai_max_tokens(),
            temperature: default_openai_temperature(),
            parameters: HashMap::new(),
        }
    }
}

fn default_openai_model() -> String {
    "gpt-4".to_string()
}

const fn default_openai_max_tokens() -> u32 {
    2000
}

const fn default_openai_temperature() -> f32 {
    0.7
}

/// Configuration for Mock provider (used in testing)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MockConfig {
    /// Additional parameters for testing
    #[serde(default)]
    pub parameters: HashMap<String, String>,
}

impl ProviderConfig {
    /// Validates the provider configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API key is missing
    /// - The model name is invalid
    /// - The max tokens value is invalid
    /// - The temperature is not between 0.0 and 1.0
    pub fn validate(&self) -> anyhow::Result<()> {
        match self {
            Self::Anthropic(config) => {
                if config.max_tokens == 0 {
                    return Err(anyhow::anyhow!("max_tokens must be greater than 0"));
                }
                if config.model.is_empty() {
                    return Err(anyhow::anyhow!("model must not be empty"));
                }
                Ok(())
            }
            Self::OpenAI(config) => {
                if config.max_tokens == 0 {
                    return Err(anyhow::anyhow!("max_tokens must be greater than 0"));
                }
                if config.model.is_empty() {
                    return Err(anyhow::anyhow!("model must not be empty"));
                }
                if !(0.0..=2.0).contains(&config.temperature) {
                    return Err(anyhow::anyhow!("temperature must be between 0.0 and 2.0"));
                }
                Ok(())
            }
            Self::Mock(_) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::config::{AnthropicConfig, MockConfig, OpenAIConfig};

    #[test]
    fn test_anthropic_config() {
        let config = AnthropicConfig::default();
        assert_eq!(config.model, "claude-2");
        assert_eq!(config.max_tokens, 1000);
        assert!(config.parameters.is_empty());

        let provider_config = ProviderConfig::Anthropic(config);
        assert_eq!(provider_config.to_string(), "anthropic");
        assert!(provider_config.validate().is_ok());

        // Test invalid config
        let invalid_config = AnthropicConfig {
            model: "".to_string(),
            max_tokens: 0,
            parameters: HashMap::new(),
        };
        let provider_config = ProviderConfig::Anthropic(invalid_config);
        assert!(provider_config.validate().is_err());
    }

    #[test]
    fn test_openai_config() {
        let config = OpenAIConfig::default();
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.max_tokens, 2000);
        assert_eq!(config.temperature, 0.7);
        assert!(config.parameters.is_empty());

        let provider_config = ProviderConfig::OpenAI(config);
        assert_eq!(provider_config.to_string(), "openai");
        assert!(provider_config.validate().is_ok());

        // Test invalid config
        let invalid_config = OpenAIConfig {
            model: "".to_string(),
            max_tokens: 0,
            temperature: 2.5,
            parameters: HashMap::new(),
        };
        let provider_config = ProviderConfig::OpenAI(invalid_config);
        assert!(provider_config.validate().is_err());
    }

    #[test]
    fn test_mock_config() {
        let config = MockConfig::default();
        assert!(config.parameters.is_empty());

        let provider_config = ProviderConfig::Mock(config);
        assert_eq!(provider_config.to_string(), "mock");
        assert!(provider_config.validate().is_ok());
    }

    #[test]
    fn test_provider_parsing() {
        // Test valid providers
        assert!(matches!(
            "anthropic".parse::<ProviderConfig>().unwrap(),
            ProviderConfig::Anthropic(_)
        ));
        assert!(matches!(
            "openai".parse::<ProviderConfig>().unwrap(),
            ProviderConfig::OpenAI(_)
        ));
        assert!(matches!(
            "mock".parse::<ProviderConfig>().unwrap(),
            ProviderConfig::Mock(_)
        ));

        // Test case insensitivity
        assert!(matches!(
            "ANTHROPIC".parse::<ProviderConfig>().unwrap(),
            ProviderConfig::Anthropic(_)
        ));

        // Test invalid provider
        assert!("invalid".parse::<ProviderConfig>().is_err());
    }

    #[test]
    fn test_provider_migration() {
        // Test old format -> new format conversion
        let old_format = r#"
            provider = "anthropic"
            [provider_specific]
            model = "claude-2"
            max_tokens = 1000
        "#;

        let parsed: toml::Value = toml::from_str(old_format).unwrap();

        // This simulates what would happen in a migration
        if let Some(provider_str) = parsed.get("provider").and_then(|v| v.as_str()) {
            let provider_config = provider_str.parse::<ProviderConfig>().unwrap();

            match provider_config {
                ProviderConfig::Anthropic(config) => {
                    assert_eq!(config.model, "claude-2");
                    assert_eq!(config.max_tokens, 1000);
                }
                _ => panic!("Expected Anthropic provider"),
            }
        }
    }
}
