use std::{env, fs, path::PathBuf};

use anyhow::Result;
use serde_json::json;
use tempfile::tempdir;

use strainer::config::Config;

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
        let original_dir = env::current_dir()?;
        Ok(Self { original_dir })
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        if let Err(e) = env::set_current_dir(&self.original_dir) {
            eprintln!("Error restoring original directory: {e}");
        }
    }
}

#[test]
fn test_config_from_env() -> Result<()> {
    // Create a directory guard to restore the working directory
    let _dir_guard = DirGuard::new()?;

    // Create a guard to clean up environment variables on test completion
    let _env_guard = EnvGuard::new(vec![
        "STRAINER_API_KEY",
        "STRAINER_PROVIDER",
        "STRAINER_BASE_URL",
        "STRAINER_REQUESTS_PER_MINUTE",
        "STRAINER_TOKENS_PER_MINUTE",
    ]);

    // Clean up any existing environment variables first
    for var in &[
        "STRAINER_API_KEY",
        "STRAINER_PROVIDER",
        "STRAINER_BASE_URL",
        "STRAINER_REQUESTS_PER_MINUTE",
        "STRAINER_TOKENS_PER_MINUTE",
    ] {
        env::remove_var(var);
    }

    // Create an isolated directory and set it for Config::load
    let dir = tempdir()?;
    env::set_current_dir(dir.path())?;

    // Set test environment variables
    env::set_var("STRAINER_API_KEY", "env-key");
    env::set_var("STRAINER_PROVIDER", "anthropic");
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");
    env::set_var("STRAINER_REQUESTS_PER_MINUTE", "30");
    env::set_var("STRAINER_TOKENS_PER_MINUTE", "50000");

    // Test direct environment config
    let config = Config::from_env()?;

    // Verify environment values
    assert_eq!(config.api.api_key, Some("env-key".to_string()));
    assert_eq!(config.api.provider, "anthropic");
    assert_eq!(config.api.base_url, Some("https://env.api.com".to_string()));
    assert_eq!(config.limits.requests_per_minute, Some(30));
    assert_eq!(config.limits.tokens_per_minute, Some(50_000));

    Ok(())
}

fn setup_test_env() -> Result<(tempfile::TempDir, EnvGuard, DirGuard)> {
    // Create a directory guard first to ensure we can restore the original directory
    let dir_guard = DirGuard::new()?;

    // Create a temporary directory for the test
    let temp_dir = tempdir()?;
    let temp_dir_path = temp_dir.path().to_path_buf();
    let config_path = temp_dir_path.join("strainer.toml");

    // Create a guard to clean up environment variables on test completion
    let env_guard = EnvGuard::new(vec![
        "STRAINER_API_KEY",
        "STRAINER_PROVIDER",
        "STRAINER_BASE_URL",
        "STRAINER_TOKENS_PER_MINUTE",
        "STRAINER_REQUESTS_PER_MINUTE",
    ]);

    // Clean up any existing variables
    for var in &[
        "STRAINER_API_KEY",
        "STRAINER_PROVIDER",
        "STRAINER_BASE_URL",
        "STRAINER_TOKENS_PER_MINUTE",
        "STRAINER_REQUESTS_PER_MINUTE",
    ] {
        env::remove_var(var);
    }

    // Write config file
    let config_content = r#"
        [api]
        provider = "mock"
        api_key = "file-key"
        base_url = "https://file.api.com"
        
        [limits]
        requests_per_minute = 30
    "#;

    fs::write(&config_path, config_content)?;

    // Change to the temporary directory
    env::set_current_dir(&temp_dir_path)?;

    Ok((temp_dir, env_guard, dir_guard))
}

#[test]
fn test_load_with_env_override_part1() -> Result<()> {
    let (temp_dir, _env_guard, _dir_guard) = setup_test_env()?;
    let _temp_dir = temp_dir; // Keep temp_dir in scope

    eprintln!("Current directory: {}", env::current_dir()?.display());
    eprintln!(
        "Config file exists: {}",
        PathBuf::from("strainer.toml").exists()
    );

    // Load initial config from file to verify
    eprintln!("Loading initial file config...");
    let file_config = Config::from_file(&PathBuf::from("strainer.toml"))?;
    eprintln!(
        "File config: provider={:?}, api_key={:?}",
        file_config.api.provider, file_config.api.api_key
    );
    assert_eq!(file_config.api.provider, "mock");
    assert_eq!(file_config.api.api_key, Some("file-key".to_string()));
    assert_eq!(
        file_config.api.base_url,
        Some("https://file.api.com".to_string())
    );

    Ok(())
}

#[test]
fn test_load_with_env_override_part2() -> Result<()> {
    let (temp_dir, _env_guard, _dir_guard) = setup_test_env()?;
    let _temp_dir = temp_dir; // Keep temp_dir in scope

    // Now set environment variables - ensure we maintain mock provider
    eprintln!("Setting environment variables...");
    env::set_var("STRAINER_PROVIDER", "mock"); // Keep mock provider
    env::set_var("STRAINER_API_KEY", "env-key");
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");
    env::set_var("STRAINER_TOKENS_PER_MINUTE", "50000");
    env::set_var("STRAINER_REQUESTS_PER_MINUTE", "30");

    // Load environment config to verify environment values are set
    eprintln!("Loading environment config...");
    let env_config = Config::from_env()?;
    eprintln!(
        "Environment config: provider={:?}, api_key={:?}",
        env_config.api.provider, env_config.api.api_key
    );
    assert_eq!(env_config.api.provider, "mock"); // Should be mock
    assert_eq!(env_config.api.api_key, Some("env-key".to_string()));
    assert_eq!(
        env_config.api.base_url,
        Some("https://env.api.com".to_string())
    );

    // Load final config which should prefer environment values
    eprintln!("Loading final config...");
    let config = Config::load()?;
    eprintln!(
        "Final config: provider={:?}, api_key={:?}",
        config.api.provider, config.api.api_key
    );

    // Environment variables should override file values
    assert_eq!(config.api.provider, "mock"); // Should still be mock
    assert_eq!(config.api.api_key, Some("env-key".to_string())); // env overrides file
    assert_eq!(config.api.base_url, Some("https://env.api.com".to_string())); // env overrides file
    assert_eq!(config.limits.requests_per_minute, Some(30)); // env overrides file
    assert_eq!(config.limits.tokens_per_minute, Some(50_000)); // env provides this value

    // Reset env vars and load again - should get file values
    env::remove_var("STRAINER_PROVIDER");
    env::remove_var("STRAINER_API_KEY");
    env::remove_var("STRAINER_BASE_URL");
    env::remove_var("STRAINER_TOKENS_PER_MINUTE");

    Ok(())
}
