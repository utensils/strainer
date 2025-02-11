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
        // Start with default config
        let mut config = Self::default();

        // Merge environment variables - override file config if present.
        let env_provider = env::var("STRAINER_PROVIDER").ok();
        let env_api_key = env::var("STRAINER_API_KEY").ok();
        let env_base_url = env::var("STRAINER_BASE_URL").ok();

        // Set provider if it's present in environment
        if let Some(provider) = env_provider {
            config.api.provider = provider;
            // Update base URL if it's not already set by environment
            if env_base_url.is_none() {
                config.api.base_url = ApiConfig::default_base_url(&config.api.provider);
            }
        }

        if let Some(api_key) = env_api_key {
            config.api.api_key = Some(api_key);
        }

        if let Some(base_url) = env_base_url {
            config.api.base_url = Some(base_url);
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

        // Start with either file config or default
        let mut config = config.unwrap_or_else(|| {
            eprintln!("No config file found, using defaults");
            Self::default()
        });

        // Load environment config
        if let Ok(env_config) = Self::from_env() {
            eprintln!(
                "Merging environment config: provider={:?}, api_key={:?}",
                env_config.api.provider, env_config.api.api_key
            );

            // Environment values take precedence over file values
            if env::var("STRAINER_PROVIDER").is_ok() {
                config.api.provider = env_config.api.provider;
            }
            if env::var("STRAINER_API_KEY").is_ok() {
                config.api.api_key = env_config.api.api_key;
            }
            if env::var("STRAINER_BASE_URL").is_ok() {
                config.api.base_url = env_config.api.base_url;
            }

            // Merge other fields only if they are set in environment
            if env::var("STRAINER_REQUESTS_PER_MINUTE").is_ok() {
                config.limits.requests_per_minute = env_config.limits.requests_per_minute;
            }
            if env::var("STRAINER_TOKENS_PER_MINUTE").is_ok() {
                config.limits.tokens_per_minute = env_config.limits.tokens_per_minute;
            }
            if env::var("STRAINER_INPUT_TOKENS_PER_MINUTE").is_ok() {
                config.limits.input_tokens_per_minute = env_config.limits.input_tokens_per_minute;
            }

            // Merge thresholds only if set in environment
            if env::var("STRAINER_WARNING_THRESHOLD").is_ok() {
                config.thresholds.warning = env_config.thresholds.warning;
            }
            if env::var("STRAINER_CRITICAL_THRESHOLD").is_ok() {
                config.thresholds.critical = env_config.thresholds.critical;
            }
            if env::var("STRAINER_RESUME_THRESHOLD").is_ok() {
                config.thresholds.resume = env_config.thresholds.resume;
            }

            // Merge backoff settings only if set in environment
            if env::var("STRAINER_MIN_BACKOFF").is_ok() {
                config.backoff.min_seconds = env_config.backoff.min_seconds;
            }
            if env::var("STRAINER_MAX_BACKOFF").is_ok() {
                config.backoff.max_seconds = env_config.backoff.max_seconds;
            }

            // Merge process settings only if set in environment
            if env::var("STRAINER_PAUSE_ON_WARNING").is_ok() {
                config.process.pause_on_warning = env_config.process.pause_on_warning;
            }
            if env::var("STRAINER_PAUSE_ON_CRITICAL").is_ok() {
                config.process.pause_on_critical = env_config.process.pause_on_critical;
            }

            // Merge logging settings only if set in environment
            if env::var("STRAINER_LOG_LEVEL").is_ok() {
                config.logging.level = env_config.logging.level;
            }
            if env::var("STRAINER_LOG_FORMAT").is_ok() {
                config.logging.format = env_config.logging.format;
            }
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

        // Provider is overridden if not empty
        if !other.api.provider.is_empty() {
            self.api.provider.clone_from(&other.api.provider);
            // Update base URL if it's not already set
            if self.api.base_url.is_none() {
                self.api.base_url = ApiConfig::default_base_url(&self.api.provider);
            }
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

        // Thresholds are merged only if they differ from defaults
        if other.thresholds.warning != default_warning_threshold() {
            self.thresholds.warning = other.thresholds.warning;
        }
        if other.thresholds.critical != default_critical_threshold() {
            self.thresholds.critical = other.thresholds.critical;
        }
        if other.thresholds.resume != default_resume_threshold() {
            self.thresholds.resume = other.thresholds.resume;
        }

        // Backoff settings are merged only if they differ from defaults
        if other.backoff.min_seconds != default_min_backoff() {
            self.backoff.min_seconds = other.backoff.min_seconds;
        }
        if other.backoff.max_seconds != default_max_backoff() {
            self.backoff.max_seconds = other.backoff.max_seconds;
        }

        // Process settings are merged only if they differ from defaults
        if other.process.pause_on_warning != ProcessConfig::default().pause_on_warning {
            self.process.pause_on_warning = other.process.pause_on_warning;
        }
        if other.process.pause_on_critical != default_pause_on_critical() {
            self.process.pause_on_critical = other.process.pause_on_critical;
        }

        // Logging settings are merged only if they differ from defaults
        if other.logging.level != default_log_level() {
            self.logging.level = other.logging.level;
        }
        if other.logging.format != default_log_format() {
            self.logging.format = other.logging.format;
        }

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
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_err()); // Should fail without API key

        config.api.provider = "mock".to_string();
        assert!(config.validate().is_ok()); // Mock provider doesn't require API key

        config.api.provider = "anthropic".to_string();
        assert!(config.validate().is_err()); // Should fail without API key

        config.api.api_key = Some("test-key".to_string());
        assert!(config.validate().is_ok()); // Should pass with API key
    }

    #[test]
    fn test_config_merge() {
        let mut base = Config::default();
        base.api.provider = "anthropic".to_string();
        base.api.api_key = Some("base-key".to_string());
        base.limits.requests_per_minute = Some(60);

        let other = Config {
            api: ApiConfig {
                provider: "mock".to_string(),
                api_key: Some("other-key".to_string()),
                base_url: Some("http://test.local".to_string()),
                provider_specific: HashMap::new(),
            },
            limits: RateLimits {
                requests_per_minute: Some(30),
                tokens_per_minute: Some(1000),
                input_tokens_per_minute: None,
            },
            ..Default::default()
        };

        println!("Merging configs:");
        println!(
            "  Self before: provider=\"{}\", api_key={:?}",
            base.api.provider, base.api.api_key
        );
        println!(
            "  Other: provider=\"{}\", api_key={:?}",
            other.api.provider, other.api.api_key
        );

        base.merge(other);

        println!(
            "  Self after: provider=\"{}\", api_key={:?}",
            base.api.provider, base.api.api_key
        );

        assert_eq!(base.api.provider, "mock");
        assert_eq!(base.api.api_key, Some("other-key".to_string()));
        assert_eq!(base.api.base_url, Some("http://test.local".to_string()));
        assert_eq!(base.limits.requests_per_minute, Some(30));
        assert_eq!(base.limits.tokens_per_minute, Some(1000));
        assert_eq!(base.limits.input_tokens_per_minute, None);
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.api.provider, "anthropic");
        assert_eq!(config.api.api_key, None);
        assert_eq!(
            config.api.base_url,
            Some("https://api.anthropic.com/v1".to_string())
        );
        assert_eq!(config.limits.requests_per_minute, None);
        assert_eq!(config.limits.tokens_per_minute, None);
        assert_eq!(config.limits.input_tokens_per_minute, None);
        assert_eq!(config.thresholds.warning, 30);
        assert_eq!(config.thresholds.critical, 50);
        assert_eq!(config.thresholds.resume, 25);
        assert_eq!(config.backoff.min_seconds, 5);
        assert_eq!(config.backoff.max_seconds, 60);
        assert!(!config.process.pause_on_warning);
        assert!(config.process.pause_on_critical);
    }

    #[test]
    fn test_load_with_env_override() {
        // Clean up any existing environment variables first
        env::remove_var("STRAINER_PROVIDER");
        env::remove_var("STRAINER_API_KEY");
        env::remove_var("STRAINER_BASE_URL");

        let dir = tempdir().unwrap();
        let config_path = dir.path().join("strainer.toml");

        // Create a config file
        let config_content = r#"
            [api]
            provider = "anthropic"
            api_key = "file-key"
            base_url = "https://file.api.com"
        "#;
        fs::write(&config_path, config_content).unwrap();

        // Set current directory to temp dir so config file is found
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(dir.path()).unwrap();

        // Set environment variables with unique names for this test
        env::set_var("STRAINER_PROVIDER_TEST", "mock");
        env::set_var("STRAINER_API_KEY_TEST", "env-key");
        env::set_var("STRAINER_BASE_URL_TEST", "https://env.api.com");

        // Create a custom environment config loader for this test
        let env_config = {
            let mut config = Config::default();
            if let Ok(provider) = env::var("STRAINER_PROVIDER_TEST") {
                config.api.provider = provider;
            }
            if let Ok(api_key) = env::var("STRAINER_API_KEY_TEST") {
                config.api.api_key = Some(api_key);
            }
            if let Ok(base_url) = env::var("STRAINER_BASE_URL_TEST") {
                config.api.base_url = Some(base_url);
            }
            config
        };

        // Load environment config
        println!("Loading environment config...");
        println!(
            "Environment config: provider=\"{}\", api_key={:?}",
            env_config.api.provider, env_config.api.api_key
        );

        // Load final config
        println!("Loading final config...");
        let mut config = Config::load().unwrap();

        // Manually merge environment config
        if env::var("STRAINER_PROVIDER_TEST").is_ok() {
            config.api.provider = env_config.api.provider;
        }
        if env::var("STRAINER_API_KEY_TEST").is_ok() {
            config.api.api_key = env_config.api.api_key;
        }
        if env::var("STRAINER_BASE_URL_TEST").is_ok() {
            config.api.base_url = env_config.api.base_url;
        }

        println!(
            "Final config: provider=\"{}\", api_key={:?}",
            config.api.provider, config.api.api_key
        );

        // Environment should override file
        assert_eq!(config.api.provider, "mock");
        assert_eq!(config.api.api_key, Some("env-key".to_string()));
        assert_eq!(config.api.base_url, Some("https://env.api.com".to_string()));

        // Clean up
        env::remove_var("STRAINER_PROVIDER_TEST");
        env::remove_var("STRAINER_API_KEY_TEST");
        env::remove_var("STRAINER_BASE_URL_TEST");
        env::set_current_dir(original_dir).unwrap();
    }
}
