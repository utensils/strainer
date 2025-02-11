use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, env, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub limits: RateLimits,
    #[serde(default)]
    pub thresholds: Thresholds,
    #[serde(default)]
    pub backoff: BackoffConfig,
    #[serde(default)]
    pub process: ProcessConfig,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[allow(clippy::struct_field_names)]
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

const fn default_warning_threshold() -> u8 {
    30
}
const fn default_critical_threshold() -> u8 {
    50
}
const fn default_resume_threshold() -> u8 {
    25
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackoffConfig {
    #[serde(default = "default_min_backoff")]
    pub min_seconds: u32,
    #[serde(default = "default_max_backoff")]
    pub max_seconds: u32,
}

const fn default_min_backoff() -> u32 {
    5
}
const fn default_max_backoff() -> u32 {
    60
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            min_seconds: default_min_backoff(),
            max_seconds: default_max_backoff(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    #[serde(default)]
    pub pause_on_warning: bool,
    #[serde(default = "default_pause_on_critical")]
    pub pause_on_critical: bool,
}

const fn default_pause_on_critical() -> bool {
    true
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            pause_on_warning: false,
            pause_on_critical: default_pause_on_critical(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_api_provider")]
    pub provider: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    #[serde(default)]
    pub provider_specific: HashMap<String, Value>,
}

fn default_api_provider() -> String {
    // Default to Anthropic as the provider
    "anthropic".to_string()
}
impl ApiConfig {
    fn default_base_url(provider: &str) -> Option<String> {
        match provider {
            "anthropic" => Some("https://api.anthropic.com/v1".to_string()),
            _ => None,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        let provider = default_api_provider();
        Self {
            provider: provider.clone(),
            api_key: None,
            base_url: Self::default_base_url(&provider),
            provider_specific: HashMap::new(),
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

fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "text".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file at the specified path
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The file cannot be read
    /// - The file contents are not valid UTF-8
    /// - The TOML content cannot be parsed into the Config structure
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }

    /// Load configuration from environment variables
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Environment variables contain invalid values
    pub fn from_env() -> Result<Self> {
        // Start with empty config (not default)
        let mut config = Self {
            api: ApiConfig {
                provider: String::new(), // Start with empty provider
                api_key: None,
                base_url: None,
                provider_specific: HashMap::new(),
            },
            limits: RateLimits::default(),
            thresholds: Thresholds::default(),
            backoff: BackoffConfig::default(),
            process: ProcessConfig::default(),
            logging: LoggingConfig::default(),
        };

        // Merge environment variables - override file config if present.
        let env_provider = env::var("STRAINER_PROVIDER").ok();
        let env_api_key = env::var("STRAINER_API_KEY").ok();
        let env_base_url = env::var("STRAINER_BASE_URL").ok();

        // Set provider if it's present in environment
        if let Some(provider) = env_provider {
            if !provider.is_empty() {
                config.api.provider = provider;
            }
        }

        if let Some(api_key) = env_api_key {
            if !api_key.is_empty() {
                config.api.api_key = Some(api_key);
            }
        }

        if let Some(base_url) = env_base_url {
            if !base_url.is_empty() {
                config.api.base_url = Some(base_url);
            }
        }

        if let Ok(rpm) = env::var("STRAINER_REQUESTS_PER_MINUTE") {
            config.limits.requests_per_minute = Some(rpm.parse()?);
        }

        if let Ok(tpm) = env::var("STRAINER_TOKENS_PER_MINUTE") {
            config.limits.tokens_per_minute = Some(tpm.parse()?);
        }

        if let Ok(itpm) = env::var("STRAINER_INPUT_TOKENS_PER_MINUTE") {
            config.limits.input_tokens_per_minute = Some(itpm.parse()?);
        }

        // Thresholds
        if let Ok(warning) = env::var("STRAINER_WARNING_THRESHOLD") {
            config.thresholds.warning = warning.parse()?;
        }

        if let Ok(critical) = env::var("STRAINER_CRITICAL_THRESHOLD") {
            config.thresholds.critical = critical.parse()?;
        }

        if let Ok(resume) = env::var("STRAINER_RESUME_THRESHOLD") {
            config.thresholds.resume = resume.parse()?;
        }

        // Process configuration
        if let Ok(pause_on_warning) = env::var("STRAINER_PAUSE_ON_WARNING") {
            config.process.pause_on_warning = pause_on_warning.parse()?;
        }

        if let Ok(pause_on_critical) = env::var("STRAINER_PAUSE_ON_CRITICAL") {
            config.process.pause_on_critical = pause_on_critical.parse()?;
        }

        // Backoff configuration
        if let Ok(min_backoff) = env::var("STRAINER_MIN_BACKOFF") {
            config.backoff.min_seconds = min_backoff.parse()?;
        }

        if let Ok(max_backoff) = env::var("STRAINER_MAX_BACKOFF") {
            config.backoff.max_seconds = max_backoff.parse()?;
        }

        // Logging configuration
        if let Ok(log_level) = env::var("STRAINER_LOG_LEVEL") {
            config.logging.level = log_level;
        }

        if let Ok(log_format) = env::var("STRAINER_LOG_FORMAT") {
            config.logging.format = log_format;
        }

        // After merging env config, override API key if environment variable is set
        if let Ok(env_api_key) = std::env::var("STRAINER_API_KEY") {
            if !env_api_key.is_empty() {
                config.api.api_key = Some(env_api_key);
            }
        }

        Ok(config)
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
            PathBuf::from("strainer.toml"),
            current_dir.join("strainer.toml"),
            home_dir.join(".config/strainer/config.toml"),
            PathBuf::from("/etc/strainer/config.toml"),
        ];

        // Find the first valid configuration file
        let mut config = None;
        for path in &config_paths {
            eprintln!("Checking for config at: {}", path.display());
            if path.exists() {
                eprintln!("Found config file at: {}", path.display());
                match Self::from_file(path) {
                    Ok(file_config) => {
                        eprintln!("Successfully loaded config from {}", path.display());
                        eprintln!(
                            "Using config from file: provider={:?}, api_key={:?}",
                            file_config.api.provider, file_config.api.api_key
                        );
                        config = Some(file_config);
                        break;
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Error loading config from {}: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }
        }

        // Use either file config or default
        let mut config = config.unwrap_or_else(|| {
            eprintln!("No config file found, using defaults");
            Self::default()
        });

        // Load environment config and merge it to override file config values
        if let Ok(env_config) = Self::from_env() {
            eprintln!(
                "Merging environment config: provider={:?}, api_key={:?}",
                env_config.api.provider, env_config.api.api_key
            );
            config.merge(env_config);
        } else {
            eprintln!("No environment config found");
        }

        eprintln!(
            "Final config before validation: provider={:?}, api_key={:?}",
            config.api.provider, config.api.api_key
        );

        // Validate the configuration
        config.validate()?;
        eprintln!(
            "Final config: provider={:?}, api_key={:?}",
            config.api.provider, config.api.api_key
        );

        Ok(config)
    }

    /// Merge another configuration into this one, with the other configuration taking precedence
    pub fn merge(&mut self, other: Self) {
        eprintln!("Merging configs:");
        eprintln!(
            "  Self before: provider={:?}, api_key={:?}, base_url={:?}",
            self.api.provider, self.api.api_key, self.api.base_url
        );
        eprintln!(
            "  Other: provider={:?}, api_key={:?}, base_url={:?}",
            other.api.provider, other.api.api_key, other.api.base_url
        );

        // API configuration - merge values that are explicitly set
        if other.api.api_key.is_some() {
            self.api.api_key = other.api.api_key;
        }

        // Base URL is overridden only if explicitly set
        if let Some(ref url) = other.api.base_url {
            self.api.base_url = Some(url.clone());
        }

        // Provider is overridden if not empty, otherwise keep existing provider
        if !other.api.provider.is_empty() {
            self.api.provider = other.api.provider;
        } else if self.api.provider.is_empty() {
            // If both are empty, use the default provider
            self.api.provider = "anthropic".to_string();
        }

        // Provider specific settings are merged
        for (key, value) in other.api.provider_specific {
            self.api.provider_specific.insert(key, value);
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

        // Thresholds are merged
        self.thresholds = other.thresholds;

        // Backoff settings are merged
        self.backoff = other.backoff;

        // Process settings are merged
        self.process = other.process;

        // Logging settings are merged
        self.logging = other.logging;

        eprintln!(
            "  Self after: provider={:?}, api_key={:?}, base_url={:?}",
            self.api.provider, self.api.api_key, self.api.base_url
        );
    }

    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Required fields are missing
    /// - Field values are invalid
    pub fn validate(&self) -> Result<()> {
        eprintln!(
            "Validating config: provider={:?}, api_key={:?}",
            self.api.provider, self.api.api_key
        );

        // Validate required fields
        // API key validation - not required for mock provider
        if self.api.provider != "mock" && self.api.api_key.is_none() {
            eprintln!("Validation failed: API key required for non-mock provider");
            return Err(anyhow!("API key is required"));
        }

        // Validate rate limits
        if let Some(rpm) = self.limits.requests_per_minute {
            if rpm == 0 {
                eprintln!("Validation failed: requests_per_minute must be greater than 0");
                return Err(anyhow!("requests_per_minute must be greater than 0"));
            }
        }

        // Validate thresholds
        if self.thresholds.warning >= self.thresholds.critical {
            eprintln!("Validation failed: warning threshold must be less than critical threshold");
            return Err(anyhow!(
                "warning threshold must be less than critical threshold"
            ));
        }
        if self.thresholds.resume >= self.thresholds.warning {
            eprintln!("Validation failed: resume threshold must be less than warning threshold");
            return Err(anyhow!(
                "resume threshold must be less than warning threshold"
            ));
        }

        eprintln!("Validation passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.api.provider, "anthropic");
        assert_eq!(
            config.api.base_url,
            Some("https://api.anthropic.com/v1".to_string())
        );
        assert!(config.api.api_key.is_none());
        assert!(config.api.provider_specific.is_empty());
    }

    #[test]
    fn test_config_merge() {
        let mut base_config = Config::default();
        base_config.api.api_key = Some("base-key".to_string());
        base_config.api.base_url = Some("https://base.api.com".to_string());
        base_config.limits.requests_per_minute = Some(10);

        let mut other_config = Config::default();
        other_config.api.api_key = Some("other-key".to_string());
        other_config.api.base_url = Some("https://other.api.com".to_string());
        other_config.limits.tokens_per_minute = Some(20_000);
        other_config
            .api
            .provider_specific
            .insert("model".to_string(), Value::String("claude-2".to_string()));

        base_config.merge(other_config.clone());

        // API key and base_url should be overridden when provided
        assert_eq!(base_config.api.api_key, Some("other-key".to_string()));
        assert_eq!(
            base_config.api.base_url,
            Some("https://other.api.com".to_string())
        );

        // Rate limits should only be overridden when set in other config
        assert_eq!(
            base_config.limits.requests_per_minute,
            Some(10),
            "base value should be kept when other is None"
        );
        assert_eq!(
            base_config.limits.tokens_per_minute,
            Some(20_000),
            "other value should override when set"
        );

        // Provider specific settings should be merged
        assert_eq!(
            base_config.api.provider_specific.get("model").unwrap(),
            &Value::String("claude-2".to_string())
        );

        // Test with explicitly set rate limit
        other_config.limits.requests_per_minute = Some(30);
        base_config.merge(other_config);
        assert_eq!(
            base_config.limits.requests_per_minute,
            Some(30),
            "explicitly set value should override"
        );
    }

    #[test]
    fn test_config_validation() {
        // Test missing API key
        let config = Config::default();
        assert!(config.validate().is_err());

        // Test invalid rate limits
        let mut config = Config::default();
        config.api.api_key = Some("test-key".to_string());
        config.limits.requests_per_minute = Some(0);
        assert!(config.validate().is_err());

        // Test invalid thresholds
        let mut config = Config::default();
        config.api.api_key = Some("test-key".to_string());
        config.thresholds.warning = 90;
        config.thresholds.critical = 80;
        assert!(config.validate().is_err());

        // Test valid config
        let mut config = Config::default();
        config.api.api_key = Some("test-key".to_string());
        config.limits.requests_per_minute = Some(60);
        assert!(config.validate().is_ok());
    }
}
