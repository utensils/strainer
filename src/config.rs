use crate::providers::config::{AnthropicConfig, MockConfig, OpenAIConfig, ProviderConfig};
use anyhow::{anyhow, Result};
use dirs;
use serde::de::Deserializer;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::{env, path::PathBuf};

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub provider_config: ProviderConfig,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub parameters: HashMap<String, String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            provider_config: ProviderConfig::Anthropic(AnthropicConfig::default()),
            api_key: None,
            base_url: None,
            parameters: HashMap::default(),
        }
    }
}

impl Serialize for ApiConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        // Serialize provider_config fields manually
        match &self.provider_config {
            ProviderConfig::Anthropic(cfg) => {
                map.serialize_entry("type", "anthropic")?;
                map.serialize_entry("model", &cfg.model)?;
                map.serialize_entry("max_tokens", &cfg.max_tokens)?;
                if !cfg.parameters.is_empty() {
                    map.serialize_entry("parameters", &cfg.parameters)?;
                }
            }
            ProviderConfig::OpenAI(cfg) => {
                map.serialize_entry("type", "openai")?;
                map.serialize_entry("model", &cfg.model)?;
                map.serialize_entry("max_tokens", &cfg.max_tokens)?;
                if !cfg.parameters.is_empty() {
                    map.serialize_entry("parameters", &cfg.parameters)?;
                }
            }
            ProviderConfig::Mock(cfg) => {
                map.serialize_entry("type", "mock")?;
                if !cfg.parameters.is_empty() {
                    map.serialize_entry("parameters", &cfg.parameters)?;
                }
            }
        }
        if let Some(api_key) = &self.api_key {
            map.serialize_entry("api_key", api_key)?;
        }
        if let Some(base_url) = &self.base_url {
            map.serialize_entry("base_url", base_url)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for ApiConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if let serde_json::Value::Object(mut obj) = value {
            let api_key = obj
                .remove("api_key")
                .and_then(|v| v.as_str().map(ToString::to_string));
            let base_url = obj
                .remove("base_url")
                .and_then(|v| v.as_str().map(ToString::to_string));
            let provider_config: ProviderConfig =
                serde_json::from_value(serde_json::Value::Object(obj))
                    .map_err(serde::de::Error::custom)?;
            Ok(Self {
                provider_config,
                api_key,
                base_url,
                parameters: HashMap::default(),
            })
        } else {
            Err(serde::de::Error::custom("Expected a map for ApiConfig"))
        }
    }
}

impl ApiConfig {
    #[must_use]
    pub fn base_url_default(&self) -> Option<String> {
        self.base_url.as_ref().map_or_else(
            || match &self.provider_config {
                ProviderConfig::Anthropic(_) => Some("https://api.anthropic.com/v1".to_string()),
                ProviderConfig::OpenAI(_) => Some("https://api.openai.com/v1".to_string()),
                ProviderConfig::Mock(_) => None,
            },
            |url| Some(url.clone()),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "text".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub api: ApiConfig,
    pub limits: RateLimits,
    pub thresholds: Thresholds,
    pub backoff: BackoffConfig,
    pub process: ProcessConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_minute: Option<u32>,
    pub tokens_per_minute: Option<u32>,
    pub input_tokens_per_minute: Option<u32>,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            requests_per_minute: Some(30),
            tokens_per_minute: Some(50000),
            input_tokens_per_minute: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    #[serde(default = "default_warning_threshold")]
    pub warning: u8,
    #[serde(default = "default_critical_threshold")]
    pub critical: u8,
    #[serde(default = "default_resume_threshold")]
    pub resume: u8,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            warning: default_warning_threshold(),
            critical: default_critical_threshold(),
            resume: default_resume_threshold(),
        }
    }
}

const fn default_warning_threshold() -> u8 {
    80
}
const fn default_critical_threshold() -> u8 {
    90
}
const fn default_resume_threshold() -> u8 {
    70
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackoffConfig {
    #[serde(default = "default_min_backoff")]
    pub min_seconds: u32,
    #[serde(default = "default_max_backoff")]
    pub max_seconds: u32,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            min_seconds: default_min_backoff(),
            max_seconds: default_max_backoff(),
        }
    }
}

const fn default_min_backoff() -> u32 {
    1
}
const fn default_max_backoff() -> u32 {
    60
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    #[serde(default = "default_pause_on_warning")]
    pub pause_on_warning: bool,
    #[serde(default = "default_pause_on_critical")]
    pub pause_on_critical: bool,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            pause_on_warning: default_pause_on_warning(),
            pause_on_critical: default_pause_on_critical(),
        }
    }
}

const fn default_pause_on_warning() -> bool {
    false
}
const fn default_pause_on_critical() -> bool {
    true
}

impl Config {
    /// Create a new configuration builder
    #[must_use]
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    /// Load configuration from default locations and environment variables
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Configuration validation fails
    pub fn load() -> Result<Self> {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let current_dir = env::current_dir()?;

        let config_paths = [
            current_dir.join("strainer.toml"),
            home_dir.join(".config/strainer/config.toml"),
            home_dir.join(".strainer.toml"),
        ];

        // Try to load from file first
        let builder = config_paths.iter().try_fold(
            Self::builder(),
            |builder, path: &std::path::PathBuf| -> Result<ConfigBuilder, anyhow::Error> {
                if path.exists() {
                    builder.from_file(path)
                } else {
                    Ok(builder)
                }
            },
        )?;

        // Then load from environment, which will override file settings
        builder.from_env()?.build()
    }

    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Required fields are missing
    /// - Field values are invalid
    /// - Provider-specific validation fails
    pub fn validate(&self) -> Result<()> {
        // Validate API configuration
        match &self.api.provider_config {
            ProviderConfig::Mock(_) => {}
            _ => {
                if self.api.api_key.is_none() {
                    return Err(anyhow!("API key is required for non-mock provider"));
                }
            }
        }

        // Validate provider-specific configuration
        self.api.provider_config.validate()?;

        // Validate thresholds
        if self.thresholds.warning >= self.thresholds.critical {
            return Err(anyhow!(
                "Warning threshold must be less than critical threshold"
            ));
        }
        if self.thresholds.resume >= self.thresholds.warning {
            return Err(anyhow!(
                "Resume threshold must be less than warning threshold"
            ));
        }

        // Validate backoff configuration
        if self.backoff.min_seconds >= self.backoff.max_seconds {
            return Err(anyhow!("Minimum backoff must be less than maximum backoff"));
        }

        Ok(())
    }

    /// Merge another configuration into this one
    pub fn merge(&mut self, other: Self) {
        // API configuration is merged
        if let Some(key) = &other.api.api_key {
            self.api
                .api_key
                .get_or_insert_with(String::new)
                .clone_from(key);
        }

        if let Some(base_url) = other.api.base_url {
            self.api.base_url = Some(base_url);
        }

        // Provider configuration is merged
        match (&mut self.api.provider_config, &other.api.provider_config) {
            (ProviderConfig::Anthropic(self_config), ProviderConfig::Anthropic(other_config)) => {
                // Merge direct fields
                self_config.model.clone_from(&other_config.model);
                self_config.max_tokens = other_config.max_tokens;
                // Merge parameters
                self_config
                    .parameters
                    .extend(other_config.parameters.clone());
            }
            (ProviderConfig::OpenAI(self_config), ProviderConfig::OpenAI(other_config)) => {
                // Merge direct fields
                self_config.model.clone_from(&other_config.model);
                self_config.max_tokens = other_config.max_tokens;

                // Merge parameters
                self_config
                    .parameters
                    .extend(other_config.parameters.clone());
            }
            (ProviderConfig::Mock(self_config), ProviderConfig::Mock(other_config)) => {
                // For mock, just merge parameters
                self_config
                    .parameters
                    .extend(other_config.parameters.clone());
            }
            _ => {
                // Different provider types - replace entirely
                self.api.provider_config = other.api.provider_config.clone();
                // Update base URL if it's not already set
                if self.api.base_url.is_none() {
                    self.api.base_url = self.api.base_url_default();
                }
            }
        }

        // Rate limits are merged if set
        if let Some(rpm) = other.limits.requests_per_minute {
            self.limits.requests_per_minute = Some(rpm);
        }
        if let Some(tpm) = other.limits.tokens_per_minute {
            self.limits.tokens_per_minute = Some(tpm);
        }
        if let Some(itpm) = other.limits.input_tokens_per_minute {
            self.limits.input_tokens_per_minute = Some(itpm);
        }

        // Thresholds are merged if they differ from defaults
        if other.thresholds.warning != default_warning_threshold() {
            self.thresholds.warning = other.thresholds.warning;
        }
        if other.thresholds.critical != default_critical_threshold() {
            self.thresholds.critical = other.thresholds.critical;
        }
        if other.thresholds.resume != default_resume_threshold() {
            self.thresholds.resume = other.thresholds.resume;
        }

        // Process settings are merged if they differ from defaults
        if other.process.pause_on_warning != ProcessConfig::default().pause_on_warning {
            self.process.pause_on_warning = other.process.pause_on_warning;
        }
        if other.process.pause_on_critical != default_pause_on_critical() {
            self.process.pause_on_critical = other.process.pause_on_critical;
        }
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            api: ApiConfig::default(),
            limits: RateLimits::default(),
            thresholds: Thresholds::default(),
            backoff: BackoffConfig::default(),
            process: ProcessConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Builder for creating Config instances with various sources
#[derive(Debug)]
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    /// Create a new configuration builder with default values
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: Config {
                api: ApiConfig {
                    provider_config: ProviderConfig::Anthropic(AnthropicConfig::default()),
                    api_key: None,
                    base_url: None,
                    parameters: HashMap::default(),
                },
                limits: RateLimits::default(),
                thresholds: Thresholds::default(),
                backoff: BackoffConfig::default(),
                process: ProcessConfig::default(),
                logging: LoggingConfig::default(),
            },
        }
    }

    /// Load configuration from a file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The file contains invalid TOML
    /// - The configuration is invalid
    pub fn from_file(mut self, path: &PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        self.config = config;
        Ok(self)
    }

    /// Load configuration from environment variables
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The environment variables contain invalid values
    /// - The configuration is invalid
    pub fn from_env(mut self) -> Result<Self> {
        // API Configuration
        if let Ok(api_key) = env::var("STRAINER_API_KEY") {
            self.config.api.api_key = Some(api_key);
        }

        if let Ok(base_url) = env::var("STRAINER_BASE_URL") {
            self.config.api.base_url = Some(base_url);
        }

        // Provider Configuration
        if let Ok(provider_type) = env::var("STRAINER_PROVIDER_TYPE") {
            self.config.api.provider_config = match provider_type.to_lowercase().as_str() {
                "openai" => ProviderConfig::OpenAI(OpenAIConfig::default()),
                "mock" => ProviderConfig::Mock(MockConfig::default()),
                _ => ProviderConfig::Anthropic(AnthropicConfig::default()),
            };
        }

        if let Ok(model) = env::var("STRAINER_MODEL") {
            self = self.with_model(model);
        }

        if let Ok(max_tokens) = env::var("STRAINER_MAX_TOKENS") {
            if let Ok(tokens) = max_tokens.parse() {
                self = self.with_max_tokens(tokens);
            }
        }

        // Rate Limits
        if let Ok(rpm) = env::var("STRAINER_REQUESTS_PER_MINUTE") {
            if let Ok(value) = rpm.parse() {
                self.config.limits.requests_per_minute = Some(value);
            }
        }

        if let Ok(tpm) = env::var("STRAINER_TOKENS_PER_MINUTE") {
            if let Ok(value) = tpm.parse() {
                self.config.limits.tokens_per_minute = Some(value);
            }
        }

        if let Ok(itpm) = env::var("STRAINER_INPUT_TOKENS_PER_MINUTE") {
            if let Ok(value) = itpm.parse() {
                self.config.limits.input_tokens_per_minute = Some(value);
            }
        }

        // Thresholds
        if let Ok(warning) = env::var("STRAINER_WARNING_THRESHOLD") {
            if let Ok(value) = warning.parse() {
                self.config.thresholds.warning = value;
            }
        }

        if let Ok(critical) = env::var("STRAINER_CRITICAL_THRESHOLD") {
            if let Ok(value) = critical.parse() {
                self.config.thresholds.critical = value;
            }
        }

        if let Ok(resume) = env::var("STRAINER_RESUME_THRESHOLD") {
            if let Ok(value) = resume.parse() {
                self.config.thresholds.resume = value;
            }
        }

        // Process Control
        if let Ok(pause_warning) = env::var("STRAINER_PAUSE_ON_WARNING") {
            if let Ok(value) = pause_warning.parse() {
                self.config.process.pause_on_warning = value;
            }
        }

        if let Ok(pause_critical) = env::var("STRAINER_PAUSE_ON_CRITICAL") {
            if let Ok(value) = pause_critical.parse() {
                self.config.process.pause_on_critical = value;
            }
        }

        Ok(self)
    }

    /// Set the provider configuration
    #[must_use]
    pub fn with_provider_config(mut self, config: ProviderConfig) -> Self {
        self.config.api.provider_config = config;
        self
    }

    /// Set the model name
    #[must_use]
    pub fn with_model(mut self, model: String) -> Self {
        match &mut self.config.api.provider_config {
            ProviderConfig::Anthropic(config) => config.model = model,
            ProviderConfig::OpenAI(config) => config.model = model,
            ProviderConfig::Mock(_) => {}
        }
        self
    }

    /// Set the maximum number of tokens
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        match &mut self.config.api.provider_config {
            ProviderConfig::Anthropic(config) => config.max_tokens = max_tokens,
            ProviderConfig::OpenAI(config) => config.max_tokens = max_tokens,
            ProviderConfig::Mock(_) => {}
        }
        self
    }

    /// Set the API key
    #[must_use]
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.config.api.api_key = Some(api_key);
        self
    }

    /// Set the base URL
    #[must_use]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.config.api.base_url = Some(base_url);
        self
    }

    /// Set requests per minute limit
    #[must_use]
    pub const fn with_requests_per_minute(mut self, rpm: u32) -> Self {
        self.config.limits.requests_per_minute = Some(rpm);
        self
    }

    /// Set tokens per minute limit
    #[must_use]
    pub const fn with_tokens_per_minute(mut self, tpm: u32) -> Self {
        self.config.limits.tokens_per_minute = Some(tpm);
        self
    }

    /// Set input tokens per minute limit
    #[must_use]
    pub const fn with_input_tokens_per_minute(mut self, itpm: u32) -> Self {
        self.config.limits.input_tokens_per_minute = Some(itpm);
        self
    }

    /// Set warning threshold
    #[must_use]
    pub const fn with_warning_threshold(mut self, threshold: u8) -> Self {
        self.config.thresholds.warning = threshold;
        self
    }

    /// Set critical threshold
    #[must_use]
    pub const fn with_critical_threshold(mut self, threshold: u8) -> Self {
        self.config.thresholds.critical = threshold;
        self
    }

    /// Set resume threshold
    #[must_use]
    pub const fn with_resume_threshold(mut self, threshold: u8) -> Self {
        self.config.thresholds.resume = threshold;
        self
    }

    /// Set pause on warning
    #[must_use]
    pub const fn with_pause_on_warning(mut self, pause: bool) -> Self {
        self.config.process.pause_on_warning = pause;
        self
    }

    /// Set pause on critical
    #[must_use]
    pub const fn with_pause_on_critical(mut self, pause: bool) -> Self {
        self.config.process.pause_on_critical = pause;
        self
    }

    /// Build and validate the final configuration
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The configuration is invalid
    pub fn build(self) -> Result<Config> {
        let config = self.config;
        config.validate()?;
        Ok(config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::config::{MockConfig, OpenAIConfig};

    #[test]
    fn test_config_validation() {
        // Test valid config
        let config = Config {
            api: ApiConfig {
                provider_config: ProviderConfig::OpenAI(OpenAIConfig {
                    model: "gpt-4".to_string(),
                    max_tokens: 2000,
                    parameters: HashMap::default(),
                }),
                api_key: Some("test-key".to_string()),
                base_url: Some("https://api.openai.com/v1".to_string()),
                parameters: HashMap::default(),
            },
            limits: RateLimits::default(),
            thresholds: Thresholds::default(),
            backoff: BackoffConfig::default(),
            process: ProcessConfig::default(),
            logging: LoggingConfig::default(),
        };

        assert!(config.validate().is_ok());

        // Test invalid config (no API key)
        let config = Config::default();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_merge() {
        let mut base = Config::default();
        base.api.provider_config = ProviderConfig::Anthropic(AnthropicConfig::default());
        base.api.api_key = Some("base-key".to_string());
        base.limits.requests_per_minute = Some(60);

        let other = Config {
            api: ApiConfig {
                provider_config: ProviderConfig::Mock(MockConfig::default()),
                api_key: Some("other-key".to_string()),
                base_url: Some("http://test.local".to_string()),
                parameters: HashMap::default(),
            },
            limits: RateLimits {
                requests_per_minute: Some(120),
                tokens_per_minute: Some(100_000),
                input_tokens_per_minute: Some(50_000),
            },
            ..Default::default()
        };

        println!(
            "  Self before: provider={:?}, api_key={:?}",
            base.api.provider_config, base.api.api_key
        );
        println!(
            "  Other: provider={:?}, api_key={:?}",
            other.api.provider_config, other.api.api_key
        );

        base.merge(other);

        println!(
            "  Self after: provider={:?}, api_key={:?}",
            base.api.provider_config, base.api.api_key
        );

        match base.api.provider_config {
            ProviderConfig::Mock(_) => {}
            _ => panic!("Expected Mock provider"),
        }
        assert_eq!(base.api.api_key, Some("other-key".to_string()));
        assert_eq!(base.api.base_url, Some("http://test.local".to_string()));
        assert_eq!(base.limits.requests_per_minute, Some(120));
        assert_eq!(base.limits.tokens_per_minute, Some(100_000));
        assert_eq!(base.limits.input_tokens_per_minute, Some(50_000));
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(matches!(
            config.api.provider_config,
            ProviderConfig::Anthropic(_)
        ));
    }

    #[test]
    fn test_provider_type() {
        let config = Config {
            api: ApiConfig {
                provider_config: ProviderConfig::Mock(MockConfig {
                    parameters: HashMap::default(),
                    requests_per_minute: 100,
                    tokens_per_minute: 1000,
                    input_tokens_per_minute: 500,
                }),
                api_key: None,
                base_url: None,
                parameters: HashMap::default(),
            },
            limits: RateLimits::default(),
            thresholds: Thresholds::default(),
            backoff: BackoffConfig::default(),
            process: ProcessConfig::default(),
            logging: LoggingConfig::default(),
        };
        assert!(matches!(
            config.api.provider_config,
            ProviderConfig::Mock(_)
        ));
    }
}
