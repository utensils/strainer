use serde::de::{Deserializer, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Invalid provider type: {0}")]
    InvalidProvider(String),
}

/// Provider-specific configuration traits and types
#[derive(Debug, Clone)]
pub enum ProviderConfig {
    Anthropic(AnthropicConfig),
    OpenAI(OpenAIConfig),
    Mock(MockConfig),
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
    #[serde(
        default = "default_anthropic_model",
        serialize_with = "serialize_string"
    )]
    pub model: String,
    /// Maximum tokens to generate
    #[serde(
        default = "default_anthropic_max_tokens",
        serialize_with = "serialize_u32"
    )]
    pub max_tokens: u32,
    /// Additional model parameters
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
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
    #[serde(default = "default_openai_model", serialize_with = "serialize_string")]
    pub model: String,
    /// Maximum tokens to generate
    #[serde(
        default = "default_openai_max_tokens",
        serialize_with = "serialize_u32"
    )]
    pub max_tokens: u32,

    /// Additional parameters
    #[serde(default, serialize_with = "serialize_hashmap")]
    pub parameters: HashMap<String, String>,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            model: default_openai_model(),
            max_tokens: default_openai_max_tokens(),

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

/// Serializes a string value
///
/// # Errors
///
/// Returns an error if serialization fails
pub fn serialize_string<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(value)
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn serialize_u32<S>(value: &u32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u32(*value)
}

fn serialize_hashmap<S>(value: &HashMap<String, String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.serialize(serializer)
}

/// Configuration for Mock provider (used in testing)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MockConfig {
    /// Additional parameters for testing
    #[serde(default)]
    pub parameters: HashMap<String, String>,
    /// Simulated requests per minute
    #[serde(default = "default_mock_requests")]
    pub requests_per_minute: u32,
    /// Simulated tokens per minute
    #[serde(default = "default_mock_tokens")]
    pub tokens_per_minute: u32,
    /// Simulated input tokens per minute
    #[serde(default = "default_mock_input_tokens")]
    pub input_tokens_per_minute: u32,
}

const fn default_mock_requests() -> u32 {
    100
}

const fn default_mock_tokens() -> u32 {
    1000
}

const fn default_mock_input_tokens() -> u32 {
    500
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

                Ok(())
            }
            Self::Mock(_) => Ok(()),
        }
    }
}

impl serde::Serialize for ProviderConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        match self {
            Self::Anthropic(cfg) => {
                map.serialize_entry("type", "anthropic")?;
                map.serialize_entry("model", &cfg.model)?;
                map.serialize_entry("max_tokens", &cfg.max_tokens)?;
                if !cfg.parameters.is_empty() {
                    map.serialize_entry("parameters", &cfg.parameters)?;
                }
            }
            Self::OpenAI(cfg) => {
                map.serialize_entry("type", "openai")?;
                map.serialize_entry("model", &cfg.model)?;
                map.serialize_entry("max_tokens", &cfg.max_tokens)?;

                if !cfg.parameters.is_empty() {
                    map.serialize_entry("parameters", &cfg.parameters)?;
                }
            }
            Self::Mock(cfg) => {
                map.serialize_entry("type", "mock")?;
                if !cfg.parameters.is_empty() {
                    map.serialize_entry("parameters", &cfg.parameters)?;
                }
            }
        }
        map.end()
    }
}

impl<'de> serde::Deserialize<'de> for ProviderConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ProviderConfigVisitor;

        impl<'de> Visitor<'de> for ProviderConfigVisitor {
            type Value = ProviderConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a flat map representing a provider configuration")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                use serde::de::Error;
                let mut values = serde_json::Map::new();
                while let Some((key, value)) = access.next_entry::<String, serde_json::Value>()? {
                    values.insert(key, value);
                }
                let type_value = values
                    .remove("type")
                    .ok_or_else(|| M::Error::missing_field("type"))?;
                let provider_type = type_value
                    .as_str()
                    .ok_or_else(|| M::Error::custom("type field is not a string"))?;
                let obj = serde_json::Value::Object(values);
                match provider_type {
                    "anthropic" => {
                        let cfg: AnthropicConfig =
                            serde_json::from_value(obj).map_err(M::Error::custom)?;
                        Ok(ProviderConfig::Anthropic(cfg))
                    }
                    "openai" => {
                        let cfg: OpenAIConfig =
                            serde_json::from_value(obj).map_err(M::Error::custom)?;
                        Ok(ProviderConfig::OpenAI(cfg))
                    }
                    "mock" => {
                        let cfg: MockConfig =
                            serde_json::from_value(obj).map_err(M::Error::custom)?;
                        Ok(ProviderConfig::Mock(cfg))
                    }
                    other => Err(M::Error::custom(format!("unknown provider type: {other}"))),
                }
            }
        }

        deserializer.deserialize_map(ProviderConfigVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_config() {
        let config = AnthropicConfig {
            model: "claude-2".to_string(),
            max_tokens: 1000,
            parameters: HashMap::new(),
        };
        assert_eq!(config.model, "claude-2");
        assert_eq!(config.max_tokens, 1000);
    }

    #[test]
    fn test_openai_config() {
        let config = OpenAIConfig {
            model: "gpt-4".to_string(),
            max_tokens: 2000,
            parameters: HashMap::new(),
        };
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.max_tokens, 2000);
    }

    #[test]
    fn test_mock_config() {
        let config = MockConfig {
            parameters: HashMap::new(),
            requests_per_minute: 100,
            tokens_per_minute: 1000,
            input_tokens_per_minute: 500,
        };
        assert!(config.parameters.is_empty());
        assert_eq!(config.requests_per_minute, 100);
        assert_eq!(config.tokens_per_minute, 1000);
        assert_eq!(config.input_tokens_per_minute, 500);
    }

    #[test]
    fn test_provider_parsing() {
        let anthropic = ProviderConfig::Anthropic(AnthropicConfig::default());
        let openai = ProviderConfig::OpenAI(OpenAIConfig::default());
        let mock = ProviderConfig::Mock(MockConfig::default());

        assert_eq!(anthropic.to_string(), "anthropic");
        assert_eq!(openai.to_string(), "openai");
        assert_eq!(mock.to_string(), "mock");
    }
}
