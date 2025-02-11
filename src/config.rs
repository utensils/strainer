use crate::providers::config::ProviderConfig;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{env, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub limits: RateLimits,
    #[serde(default)]
    pub thresholds: Thresholds,
    #[serde(default)]
    pub backoff: BackoffConfig,
    #[serde(default)]
    pub process: ProcessConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateLimits {
    pub requests_per_minute: Option<u32>,
    pub tokens_per_minute: Option<u32>,
    pub input_tokens_per_minute: Option<u32>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(flatten)]
    pub provider_config: ProviderConfig,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        let config = Self {
            provider_config: ProviderConfig::default(),
            api_key: None,
            base_url: None,
        };
        Self {
            base_url: config.base_url(),
            ..config
        }
    }
}

impl ApiConfig {
    #[must_use]
    pub fn base_url(&self) -> Option<String> {
        match &self.provider_config {
            ProviderConfig::Anthropic(_) => Some("https://api.anthropic.com/v1".to_string()),
            ProviderConfig::OpenAI(_) => Some("https://api.openai.com/v1".to_string()),
            ProviderConfig::Mock(_) => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "text".to_string()
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
        if let Some(api_key) = other.api.api_key {
            self.api.api_key = Some(api_key);
        }

        if let Some(base_url) = other.api.base_url {
            self.api.base_url = Some(base_url);
        }

        // Provider configuration is replaced if different
        if std::mem::discriminant(&other.api.provider_config)
            != std::mem::discriminant(&self.api.provider_config)
        {
            self.api.provider_config = other.api.provider_config;
            // Update base URL if it's not already set
            if self.api.base_url.is_none() {
                self.api.base_url = self.api.base_url();
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
}

/// Builder for creating Config instances with various sources
#[derive(Debug)]
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    /// Create a new `ConfigBuilder` with default values
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    /// Load configuration from a TOML file
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The file cannot be read
    /// - The file contents are not valid UTF-8
    /// - The TOML content cannot be parsed
    pub fn from_file(mut self, path: &PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let file_config: Config = toml::from_str(&contents)?;
        self.config.merge(file_config);
        Ok(self)
    }

    /// Load configuration from environment variables
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Environment variables contain invalid values
    pub fn from_env(mut self) -> Result<Self> {
        let mut env_config = Config::default();

        // API Configuration
        if let Ok(provider_type) = env::var("STRAINER_PROVIDER_TYPE") {
            env_config.api.provider_config = provider_type.parse().map_err(|e| anyhow!("{}", e))?;
            // Update base URL if not explicitly set
            if env::var("STRAINER_BASE_URL").is_err() {
                env_config.api.base_url = env_config.api.base_url();
            }
        }

        if let Ok(api_key) = env::var("STRAINER_API_KEY") {
            env_config.api.api_key = Some(api_key);
        }

        if let Ok(base_url) = env::var("STRAINER_BASE_URL") {
            env_config.api.base_url = Some(base_url);
        }

        // Rate Limits
        if let Ok(rpm) = env::var("STRAINER_REQUESTS_PER_MINUTE") {
            env_config.limits.requests_per_minute = Some(rpm.parse()?);
        }

        if let Ok(tpm) = env::var("STRAINER_TOKENS_PER_MINUTE") {
            env_config.limits.tokens_per_minute = Some(tpm.parse()?);
        }

        if let Ok(itpm) = env::var("STRAINER_INPUT_TOKENS_PER_MINUTE") {
            env_config.limits.input_tokens_per_minute = Some(itpm.parse()?);
        }

        // Thresholds
        if let Ok(warning) = env::var("STRAINER_WARNING_THRESHOLD") {
            env_config.thresholds.warning = warning.parse()?;
        }

        if let Ok(critical) = env::var("STRAINER_CRITICAL_THRESHOLD") {
            env_config.thresholds.critical = critical.parse()?;
        }

        if let Ok(resume) = env::var("STRAINER_RESUME_THRESHOLD") {
            env_config.thresholds.resume = resume.parse()?;
        }

        // Process Configuration
        if let Ok(pause_on_warning) = env::var("STRAINER_PAUSE_ON_WARNING") {
            env_config.process.pause_on_warning = pause_on_warning.parse()?;
        }

        if let Ok(pause_on_critical) = env::var("STRAINER_PAUSE_ON_CRITICAL") {
            env_config.process.pause_on_critical = pause_on_critical.parse()?;
        }

        // Backoff Configuration
        if let Ok(min_backoff) = env::var("STRAINER_MIN_BACKOFF") {
            env_config.backoff.min_seconds = min_backoff.parse()?;
        }

        if let Ok(max_backoff) = env::var("STRAINER_MAX_BACKOFF") {
            env_config.backoff.max_seconds = max_backoff.parse()?;
        }

        // Logging Configuration
        if let Ok(log_level) = env::var("STRAINER_LOG_LEVEL") {
            env_config.logging.level = log_level;
        }

        if let Ok(log_format) = env::var("STRAINER_LOG_FORMAT") {
            env_config.logging.format = log_format;
        }

        self.config.merge(env_config);
        Ok(self)
    }

    /// Set the API provider
    #[must_use]
    pub fn with_provider(mut self, provider: &str) -> Self {
        // Try to parse the provider string, fallback to current if invalid
        if let Ok(provider_config) = provider.parse() {
            self.config.api.provider_config = provider_config;
        }
        self
    }

    /// Set provider-specific configuration
    #[must_use]
    pub fn with_provider_config(mut self, config: ProviderConfig) -> Self {
        self.config.api.provider_config = config;
        self
    }

    /// Set the model for the current provider
    #[must_use]
    pub fn with_model(mut self, model: String) -> Self {
        match &mut self.config.api.provider_config {
            ProviderConfig::Anthropic(cfg) => cfg.model = model,
            ProviderConfig::OpenAI(cfg) => cfg.model = model,
            ProviderConfig::Mock(_) => {}
        }
        self
    }

    /// Set the maximum tokens for the current provider
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        match &mut self.config.api.provider_config {
            ProviderConfig::Anthropic(cfg) => cfg.max_tokens = max_tokens,
            ProviderConfig::OpenAI(cfg) => cfg.max_tokens = max_tokens,
            ProviderConfig::Mock(_) => {}
        }
        self
    }

    /// Set the temperature for `OpenAI` provider
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        if let ProviderConfig::OpenAI(cfg) = &mut self.config.api.provider_config {
            cfg.temperature = temperature;
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
    use crate::providers::config::{AnthropicConfig, MockConfig};
    use tempfile::tempdir;

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_err()); // Should fail without API key

        config.api.provider_config = ProviderConfig::Mock(MockConfig::default());
        assert!(config.validate().is_ok()); // Mock provider doesn't require API key

        config.api.provider_config = ProviderConfig::Anthropic(AnthropicConfig::default());
        assert!(config.validate().is_err()); // Should fail without API key

        config.api.api_key = Some("test-key".to_string());
        assert!(config.validate().is_ok()); // Should pass with API key

        config.thresholds.warning = 95;
        config.thresholds.critical = 90;
        assert!(config.validate().is_err()); // Should fail with invalid thresholds

        config.thresholds.warning = 80;
        config.thresholds.critical = 90;
        config.thresholds.resume = 85;
        assert!(config.validate().is_err()); // Should fail with invalid resume threshold

        config.thresholds.resume = 70;
        assert!(config.validate().is_ok()); // Should pass with valid thresholds

        config.backoff.min_seconds = 60;
        config.backoff.max_seconds = 30;
        assert!(config.validate().is_err()); // Should fail with invalid backoff
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
        match config.api.provider_config {
            ProviderConfig::Anthropic(cfg) => {
                assert_eq!(cfg.model, "claude-2");
                assert_eq!(cfg.max_tokens, 1000);
            }
            _ => panic!("Expected Anthropic provider as default"),
        }
        assert_eq!(config.api.api_key, None);
        assert_eq!(
            config.api.base_url,
            Some("https://api.anthropic.com/v1".to_string())
        );
        assert_eq!(config.limits.requests_per_minute, None);
        assert_eq!(config.limits.tokens_per_minute, None);
        assert_eq!(config.limits.input_tokens_per_minute, None);
        assert_eq!(config.thresholds.warning, 80);
        assert_eq!(config.thresholds.critical, 90);
        assert_eq!(config.thresholds.resume, 70);
        assert_eq!(config.backoff.min_seconds, 1);
        assert_eq!(config.backoff.max_seconds, 60);
        assert!(!config.process.pause_on_warning);
        assert!(config.process.pause_on_critical);
    }

    #[test]
    fn test_load_with_env_override() {
        let original_dir = env::current_dir().unwrap();
        let temp_dir = tempdir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();

        // Set test environment variables
        env::set_var("STRAINER_PROVIDER_TYPE", "mock");
        env::set_var("STRAINER_API_KEY", "env-key");
        env::set_var("STRAINER_BASE_URL", "https://env.api.com");

        // Load config from environment
        let config = Config::builder().from_env().unwrap().build().unwrap();

        // Environment should override defaults
        match config.api.provider_config {
            ProviderConfig::Mock(_) => {}
            _ => panic!("Expected Mock provider"),
        }
        assert_eq!(config.api.api_key, Some("env-key".to_string()));
        assert_eq!(config.api.base_url, Some("https://env.api.com".to_string()));

        // Clean up
        env::remove_var("STRAINER_PROVIDER_TYPE");
        env::remove_var("STRAINER_API_KEY");
        env::remove_var("STRAINER_BASE_URL");
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_provider_type() {
        let config = Config::builder()
            .with_api_key("test-key".to_string())
            .build()
            .unwrap();
        match &config.api.provider_config {
            ProviderConfig::Anthropic(_) => {
                assert_eq!(config.api.provider_config.to_string(), "anthropic");
            }
            _ => panic!("Expected Anthropic provider as default"),
        }

        let config = Config::builder()
            .with_api_key("test-key".to_string())
            .with_provider("openai")
            .build()
            .unwrap();
        match &config.api.provider_config {
            ProviderConfig::OpenAI(_) => {
                assert_eq!(config.api.provider_config.to_string(), "openai");
            }
            _ => panic!("Expected OpenAI provider"),
        }

        let config = Config::builder().with_provider("mock").build().unwrap();
        match &config.api.provider_config {
            ProviderConfig::Mock(_) => {
                assert_eq!(config.api.provider_config.to_string(), "mock");
            }
            _ => panic!("Expected Mock provider"),
        }
    }
}
