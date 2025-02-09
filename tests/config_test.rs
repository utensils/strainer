use std::{env, fs};

use anyhow::Result;
use serde_json::json;
use strainer::config::Config;
use tempfile::tempdir;

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

#[test]
fn test_config_from_env() -> Result<()> {
    env::set_var("STRAINER_API_KEY", "env-test-key");
    env::set_var("STRAINER_PROVIDER", "anthropic");
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");
    env::set_var("STRAINER_REQUESTS_PER_MINUTE", "30");
    env::set_var("STRAINER_TOKENS_PER_MINUTE", "50000");

    let config = Config::from_env()?;
    assert_eq!(config.api.provider, "anthropic");
    assert_eq!(config.api.api_key, Some("env-test-key".to_string()));
    assert_eq!(config.api.base_url, Some("https://env.api.com".to_string()));
    assert_eq!(config.limits.requests_per_minute, Some(30));
    assert_eq!(config.limits.tokens_per_minute, Some(50_000));

    // Cleanup
    env::remove_var("STRAINER_API_KEY");
    env::remove_var("STRAINER_PROVIDER");
    env::remove_var("STRAINER_BASE_URL");
    env::remove_var("STRAINER_REQUESTS_PER_MINUTE");
    env::remove_var("STRAINER_TOKENS_PER_MINUTE");

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
    let dir = tempdir()?;
    let config_path = dir.path().join("strainer.toml");

    let config_content = r#"
        [api]
        provider = "anthropic"
        api_key = "file-key"
        base_url = "https://file.api.com"
        
        [limits]
        requests_per_minute = 60
    "#;

    fs::write(&config_path, config_content)?;

    // Set environment variables
    env::set_var("STRAINER_API_KEY", "env-key");
    env::set_var("STRAINER_TOKENS_PER_MINUTE", "50000");

    // Change to the temporary directory
    let original_dir = env::current_dir()?;
    env::set_current_dir(dir.path())?;

    let config = Config::load()?;

    // Restore original directory
    env::set_current_dir(original_dir)?;

    // Clean up environment
    env::remove_var("STRAINER_API_KEY");
    env::remove_var("STRAINER_TOKENS_PER_MINUTE");

    assert_eq!(config.api.api_key, Some("env-key".to_string()));
    assert_eq!(
        config.api.base_url,
        Some("https://file.api.com".to_string())
    );
    assert_eq!(config.limits.requests_per_minute, Some(60));
    assert_eq!(config.limits.tokens_per_minute, Some(50_000));

    Ok(())
}
