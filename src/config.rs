use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub key: Option<String>,
    #[serde(default = "default_api_base_url")]
    pub base_url: String,
}

fn default_api_provider() -> String {
    "anthropic".to_string()
}
fn default_api_base_url() -> String {
    "https://api.anthropic.com/v1".to_string()
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            provider: default_api_provider(),
            key: None,
            base_url: default_api_base_url(),
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
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}
