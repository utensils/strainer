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
    #[serde(default = "default_api_base_url")]
    pub base_url: Option<String>,
    #[serde(default)]
    pub provider_specific: HashMap<String, Value>,
}

fn default_api_provider() -> String {
    // Default to Anthropic as the provider
    "anthropic".to_string()
}
#[allow(clippy::unnecessary_wraps)]
fn default_api_base_url() -> Option<String> {
    Some("https://api.anthropic.com/v1".to_string())
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            provider: default_api_provider(),
            api_key: None,
            base_url: None,
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
        let mut config = Self::default();

        // Create a clean environment-only config
        if let Ok(api_key) = env::var("STRAINER_API_KEY") {
            config.api.api_key = Some(api_key);
        }

        if let Ok(provider) = env::var("STRAINER_PROVIDER") {
            config.api.provider = provider;
        }

        if let Ok(base_url) = env::var("STRAINER_BASE_URL") {
            config.api.base_url = Some(base_url);
        }

        if let Ok(rpm) = env::var("STRAINER_REQUESTS_PER_MINUTE") {
            config.limits.requests_per_minute = Some(rpm.parse()?);
        }

        if let Ok(tpm) = env::var("STRAINER_TOKENS_PER_MINUTE") {
            config.limits.tokens_per_minute = Some(tpm.parse()?);
        }

        // Clear any pre-existing provider-specific data that might have come from default
        config.api.provider_specific.clear();

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

        let config_paths = [
            PathBuf::from("strainer.toml"),
            home_dir.join(".config/strainer/config.toml"),
            PathBuf::from("/etc/strainer/config.toml"),
        ];

        let mut config = Self::default();

        // Load from file if found
        for path in &config_paths {
            if let Ok(file_config) = Self::from_file(path) {
                config = file_config;
                break;
            }
        }

        // Override with environment variables
        if let Ok(env_config) = Self::from_env() {
            config.merge(env_config);
        }

        config.validate()?;

        Ok(config)
    }

    /// Merge another configuration into this one
    pub fn merge(&mut self, other: Self) {
        // API config merging
        if let Some(key) = other.api.api_key {
            self.api.api_key = Some(key);
        }
        if let Some(url) = other.api.base_url {
            self.api.base_url = Some(url);
        }
        if !other.api.provider.is_empty() {
            self.api.provider = other.api.provider;
        }
        // Merge provider specific settings
        for (key, value) in other.api.provider_specific {
            self.api.provider_specific.insert(key, value);
        }

        // Rate limits merging
        if let Some(rpm) = other.limits.requests_per_minute {
            self.limits.requests_per_minute = Some(rpm);
        }
        if let Some(tpm) = other.limits.tokens_per_minute {
            self.limits.tokens_per_minute = Some(tpm);
        }
        if let Some(itpm) = other.limits.input_tokens_per_minute {
            self.limits.input_tokens_per_minute = Some(itpm);
        }

        // Thresholds merging
        self.thresholds = other.thresholds;

        // Backoff config merging
        self.backoff = other.backoff;

        // Process config merging
        self.process = other.process;

        // Logging config merging
        self.logging = other.logging;
    }

    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Required fields are missing
    /// - Field values are invalid
    pub fn validate(&self) -> Result<()> {
        // Validate required fields
        // API key validation - not required for mock provider
        if self.api.provider != "mock" && self.api.api_key.is_none() {
            return Err(anyhow!("API key is required"));
        }

        // Validate rate limits
        if let Some(rpm) = self.limits.requests_per_minute {
            if rpm == 0 {
                return Err(anyhow!("requests_per_minute must be greater than 0"));
            }
        }

        // Validate thresholds
        if self.thresholds.warning >= self.thresholds.critical {
            return Err(anyhow!(
                "warning threshold must be less than critical threshold"
            ));
        }
        if self.thresholds.resume >= self.thresholds.warning {
            return Err(anyhow!(
                "resume threshold must be less than warning threshold"
            ));
        }

        Ok(())
    }
}
