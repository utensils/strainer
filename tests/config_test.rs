use std::{env, fs, path::PathBuf};

use anyhow::Result;
use serde_json::json;
use tempfile::tempdir;

use strainer::config::Config;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.api.provider, "anthropic");
    assert!(config.api.base_url.is_none());
    assert!(config.api.api_key.is_none());
    assert!(config.api.provider_specific.is_empty());
}

#[test]
fn test_config_from_file() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");

    let config_content = r#"
        [api]
        provider = "anthropic"
        api_key = "test-key"
        base_url = "https://custom.api.com"
        
        [api.provider_specific]
        model = "claude-2"
        max_tokens = 1000
        
        [limits]
        requests_per_minute = 60
        tokens_per_minute = 100_000
    "#;

    fs::write(&config_path, config_content)?;

    let config = Config::from_file(&config_path)?;
    assert_eq!(config.api.provider, "anthropic");
    assert_eq!(config.api.api_key, Some("test-key".to_string()));
    assert_eq!(
        config.api.base_url,
        Some("https://custom.api.com".to_string())
    );
    assert_eq!(
        config.api.provider_specific.get("model").unwrap(),
        &json!("claude-2")
    );
    assert_eq!(
        config.api.provider_specific.get("max_tokens").unwrap(),
        &json!(1000)
    );
    assert_eq!(config.limits.requests_per_minute, Some(60));
    assert_eq!(config.limits.tokens_per_minute, Some(100_000));

    Ok(())
}

struct EnvGuard {
    vars: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn new(vars: Vec<&'static str>) -> Self {
        let vars = vars
            .into_iter()
            .map(|var| (var, env::var(var).ok()))
            .collect();
        Self { vars }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // Restore original environment state
        for (var, original_value) in &self.vars {
            match original_value {
                Some(value) => env::set_var(var, value),
                None => env::remove_var(var),
            }
        }
    }
}

struct DirGuard {
    original_dir: PathBuf,
}

impl DirGuard {
    fn new() -> Result<Self> {
        Ok(Self {
            original_dir: env::current_dir()?,
        })
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        // Restore original directory on scope exit, ignore errors in drop
        let _ = env::set_current_dir(&self.original_dir);
    }
}

#[test]
fn test_config_from_env() -> Result<()> {
    use tempfile::tempdir;

    // Create a directory guard to restore the working directory
    let _dir_guard = DirGuard::new()?;
    // First, clear any existing environment variables
    for var in &[
        "STRAINER_API_KEY",
        "STRAINER_PROVIDER",
        "STRAINER_BASE_URL",
        "STRAINER_REQUESTS_PER_MINUTE",
        "STRAINER_TOKENS_PER_MINUTE",
    ] {
        env::remove_var(var);
    }

    // Set test environment variables
    env::set_var("STRAINER_API_KEY", "env-key");
    env::set_var("STRAINER_PROVIDER", "anthropic");
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");
    env::set_var("STRAINER_REQUESTS_PER_MINUTE", "30");
    env::set_var("STRAINER_TOKENS_PER_MINUTE", "50000");

    // Create a guard to clean up environment variables on test completion
    let _guard = EnvGuard::new(vec![
        "STRAINER_API_KEY",
        "STRAINER_PROVIDER",
        "STRAINER_BASE_URL",
        "STRAINER_REQUESTS_PER_MINUTE",
        "STRAINER_TOKENS_PER_MINUTE",
    ]);

    // Create an isolated directory and set it for Config::load
    let dir = tempdir()?;
    env::set_current_dir(dir.path())?; // Set temp directory for this test

    // Test 1: Direct environment config
    let env_config = Config::from_env()?;

    // Debug Prints
    println!("API Key: {:?}", env_config.api.api_key);
    println!("Provider: {:?}", env_config.api.provider);
    println!("Base URL: {:?}", env_config.api.base_url);
    println!(
        "Requests per Minute: {:?}",
        env_config.limits.requests_per_minute
    );
    println!(
        "Tokens per Minute: {:?}",
        env_config.limits.tokens_per_minute
    );

    // Verify direct environment values
    assert_eq!(env_config.api.api_key, Some("env-key".to_string()));
    assert_eq!(env_config.api.provider, "anthropic");
    assert_eq!(
        env_config.api.base_url,
        Some("https://env.api.com".to_string())
    );
    assert_eq!(env_config.limits.requests_per_minute, Some(30));
    assert_eq!(env_config.limits.tokens_per_minute, Some(50_000));

    // Test 2: Full config loading process
    let loaded_config = Config::load()?;

    // Verify loaded config values are same as environment
    assert_eq!(loaded_config.api.api_key, Some("env-key".to_string()));
    assert_eq!(loaded_config.api.provider, "anthropic");
    assert_eq!(
        loaded_config.api.base_url,
        Some("https://env.api.com".to_string())
    );
    assert_eq!(loaded_config.limits.requests_per_minute, Some(30));
    assert_eq!(loaded_config.limits.tokens_per_minute, Some(50_000));

    Ok(())
}

#[test]
fn test_config_merge() {
    let mut base_config = Config::default();
    base_config.api.api_key = Some("base-key".to_string());
    base_config.api.base_url = Some("https://base.api.com".to_string());
    base_config.limits.requests_per_minute = Some(10);

    let mut other_config = Config::default();
    other_config.api.api_key = Some("other-key".to_string());
    other_config.limits.tokens_per_minute = Some(20_000);
    other_config
        .api
        .provider_specific
        .insert("model".to_string(), json!("claude-2"));

    base_config.merge(other_config);

    assert_eq!(base_config.api.api_key, Some("other-key".to_string()));
    assert_eq!(
        base_config.api.base_url,
        Some("https://base.api.com".to_string())
    );
    assert_eq!(base_config.limits.requests_per_minute, Some(10));
    assert_eq!(base_config.limits.tokens_per_minute, Some(20_000));
    assert_eq!(
        base_config.api.provider_specific.get("model").unwrap(),
        &json!("claude-2")
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

#[test]
fn test_load_with_env_override() -> Result<()> {
    // Create guards for directory and variables
    let _dir_guard = DirGuard::new()?;

    // Create environment guard before setting any variables
    let _env_guard = EnvGuard::new(vec![
        "STRAINER_API_KEY",
        "STRAINER_TOKENS_PER_MINUTE",
        "STRAINER_BASE_URL",
        "STRAINER_REQUESTS_PER_MINUTE",
    ]);

    let dir = tempdir()?;
    let config_path = dir.path().join("strainer.toml");

    let config_content = r#"
        [api]
        provider = "anthropic"
        api_key = "file-key"
        base_url = "https://file.api.com"
        
        [limits]
        requests_per_minute = 30
    "#;

    fs::write(&config_path, config_content)?;

    // Clear any existing environment variables first
    env::remove_var("STRAINER_API_KEY");
    env::remove_var("STRAINER_BASE_URL");
    env::remove_var("STRAINER_TOKENS_PER_MINUTE");
    env::remove_var("STRAINER_REQUESTS_PER_MINUTE");

    // Set environment variables
    env::set_var("STRAINER_API_KEY", "env-key");
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");
    env::set_var("STRAINER_TOKENS_PER_MINUTE", "50000");
    env::set_var("STRAINER_REQUESTS_PER_MINUTE", "60");

    env::set_current_dir(dir.path())?;
    let config = Config::load()?;

    // Debug Prints
    println!("Loaded API Key: {:?}", config.api.api_key);
    println!("Loaded Base URL: {:?}", config.api.base_url);
    println!(
        "Loaded Requests per Minute: {:?}",
        config.limits.requests_per_minute
    );
    println!(
        "Loaded Tokens per Minute: {:?}",
        config.limits.tokens_per_minute
    );

    // Environment variables should override file values
    assert_eq!(config.api.api_key, Some("env-key".to_string())); // env overrides file
    assert_eq!(config.api.base_url, Some("https://env.api.com".to_string())); // env overrides file
    assert_eq!(config.limits.requests_per_minute, Some(60)); // env overrides file
    assert_eq!(config.limits.tokens_per_minute, Some(50_000)); // env provides this value

    Ok(())
}
