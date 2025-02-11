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
    // Create guards first to ensure cleanup
    let (_dir_guard, _env_guard) = {
        let dir_guard = DirGuard::new()?;
        let env_guard = EnvGuard::new(vec![
            "STRAINER_API_KEY",
            "STRAINER_PROVIDER",
            "STRAINER_BASE_URL",
            "STRAINER_REQUESTS_PER_MINUTE",
            "STRAINER_TOKENS_PER_MINUTE",
        ]);
        (dir_guard, env_guard)
    };

    // Set test environment variables
    env::set_var("STRAINER_API_KEY", "env-key");
    env::set_var("STRAINER_PROVIDER", "mock"); // Changed to match expected value
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");
    env::set_var("STRAINER_REQUESTS_PER_MINUTE", "30");
    env::set_var("STRAINER_TOKENS_PER_MINUTE", "50000");

    // Create an isolated directory for the test
    let dir = tempdir()?;
    env::set_current_dir(dir.path())?;

    // Test direct environment config
    let config = Config::from_env()?;

    // Verify environment values
    assert_eq!(config.api.api_key, Some("env-key".to_string()));
    assert_eq!(config.api.provider, "mock");
    assert_eq!(config.api.base_url, Some("https://env.api.com".to_string()));
    assert_eq!(config.limits.requests_per_minute, Some(30));
    assert_eq!(config.limits.tokens_per_minute, Some(50_000));

    Ok(())
}

#[test]
fn test_load_with_env_override_part1() -> Result<()> {
    // Create guards first to ensure cleanup
    let (_dir_guard, _env_guard) = {
        let dir_guard = DirGuard::new()?;
        let env_guard = EnvGuard::new(vec![
            "STRAINER_API_KEY",
            "STRAINER_PROVIDER",
            "STRAINER_BASE_URL",
        ]);
        (dir_guard, env_guard)
    };

    // Create an isolated directory for the test
    let dir = tempdir()?;
    env::set_current_dir(dir.path())?;

    // Create a config file
    let config_content = r#"
        [api]
        provider = "mock"
        api_key = "file-key"
        base_url = "https://file.api.com"
    "#;
    fs::write("strainer.toml", config_content)?;

    // Set environment variables
    env::set_var("STRAINER_API_KEY", "env-key");
    env::set_var("STRAINER_PROVIDER", "mock");
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");

    // Load config
    let config = Config::load()?;

    // Environment should override file
    assert_eq!(config.api.api_key, Some("env-key".to_string()));
    assert_eq!(config.api.provider, "mock");
    assert_eq!(config.api.base_url, Some("https://env.api.com".to_string()));

    Ok(())
}

#[test]
fn test_load_with_env_override_part2() -> Result<()> {
    // Create guards first to ensure cleanup
    let (_dir_guard, _env_guard) = {
        let dir_guard = DirGuard::new()?;
        let env_guard = EnvGuard::new(vec![
            "STRAINER_API_KEY",
            "STRAINER_PROVIDER",
            "STRAINER_BASE_URL",
        ]);
        (dir_guard, env_guard)
    };

    // Create an isolated directory for the test
    let dir = tempdir()?;
    env::set_current_dir(dir.path())?;

    // Create a config file
    let config_content = r#"
        [api]
        provider = "mock"
        api_key = "file-key"
        base_url = "https://file.api.com"
    "#;
    fs::write("strainer.toml", config_content)?;

    println!("Setting environment variables...");
    env::set_var("STRAINER_API_KEY", "env-key");
    env::set_var("STRAINER_PROVIDER", "mock");
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");

    println!("Loading environment config...");
    let env_config = Config::from_env()?;
    println!(
        "Environment config: provider=\"{}\", api_key={:?}",
        env_config.api.provider, env_config.api.api_key
    );

    println!("Loading final config...");
    let config = Config::load()?;
    println!(
        "Final config: provider=\"{}\", api_key={:?}",
        config.api.provider, config.api.api_key
    );

    // Environment should override file
    assert_eq!(config.api.api_key, Some("env-key".to_string()));
    assert_eq!(config.api.provider, "mock");
    assert_eq!(config.api.base_url, Some("https://env.api.com".to_string()));

    Ok(())
}
