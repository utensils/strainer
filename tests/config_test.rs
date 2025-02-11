use std::{env, fs, path::PathBuf};

use anyhow::Result;
use tempfile::tempdir;

use strainer::config::Config;
use strainer::providers::config::{OpenAIConfig, ProviderConfig};

#[test]
fn test_config_from_file() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");

    let config_content = r#"
        [api]
        api_key = "test-key"
        base_url = "https://custom.api.com"
        
        [api.provider_config]
        type = "anthropic"
        model = "claude-2"
        max_tokens = 1000
        
        [limits]
        requests_per_minute = 60
        tokens_per_minute = 100000
    "#;

    fs::write(&config_path, config_content)?;

    let config = Config::builder().from_file(&config_path)?.build()?;

    assert_eq!(config.api.api_key, Some("test-key".to_string()));
    assert_eq!(
        config.api.base_url,
        Some("https://custom.api.com".to_string())
    );

    match &config.api.provider_config {
        ProviderConfig::Anthropic(cfg) => {
            assert_eq!(cfg.model, "claude-2");
            assert_eq!(cfg.max_tokens, 1000);
        }
        _ => panic!("Expected Anthropic provider"),
    }
    assert_eq!(config.limits.requests_per_minute, Some(60));
    assert_eq!(config.limits.tokens_per_minute, Some(100000));

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
            "STRAINER_PROVIDER_TYPE",
            "STRAINER_BASE_URL",
            "STRAINER_REQUESTS_PER_MINUTE",
            "STRAINER_TOKENS_PER_MINUTE",
            "STRAINER_MODEL",
            "STRAINER_MAX_TOKENS",
            "STRAINER_TEMPERATURE",
        ]);
        (dir_guard, env_guard)
    };

    // Set test environment variables
    env::set_var("STRAINER_API_KEY", "env-key");
    env::set_var("STRAINER_PROVIDER_TYPE", "mock");
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");
    env::set_var("STRAINER_REQUESTS_PER_MINUTE", "30");
    env::set_var("STRAINER_TOKENS_PER_MINUTE", "50000");

    // Create an isolated directory for the test
    let dir = tempdir()?;
    env::set_current_dir(dir.path())?;

    let config = Config::builder().from_env()?.build()?;

    assert_eq!(config.api.api_key, Some("env-key".to_string()));
    assert_eq!(config.api.base_url, Some("https://env.api.com".to_string()));

    match &config.api.provider_config {
        ProviderConfig::Mock(_) => {}
        _ => panic!("Expected Mock provider"),
    }
    assert_eq!(config.limits.requests_per_minute, Some(30));
    assert_eq!(config.limits.tokens_per_minute, Some(50000));

    Ok(())
}

#[test]
fn test_config_merge_env_over_file() -> Result<()> {
    // Create guards first to ensure cleanup
    let (_dir_guard, _env_guard) = {
        let dir_guard = DirGuard::new()?;
        let env_guard = EnvGuard::new(vec![
            "STRAINER_API_KEY",
            "STRAINER_PROVIDER_TYPE",
            "STRAINER_BASE_URL",
            "STRAINER_MODEL",
            "STRAINER_MAX_TOKENS",
        ]);
        (dir_guard, env_guard)
    };

    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let config_content = r#"
        [api]
        api_key = "file-key"
        base_url = "https://file.api.com"
        
        [api.provider_config]
        type = "anthropic"
        model = "claude-2"
        max_tokens = 1000
    "#;
    fs::write(&config_path, config_content)?;

    // Set environment variables that should override the file
    env::set_var("STRAINER_API_KEY", "env-key");
    env::set_var("STRAINER_PROVIDER_TYPE", "mock");
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");

    // Create an isolated directory for the test
    env::set_current_dir(dir.path())?;

    let config = Config::builder()
        .from_file(&config_path)?
        .from_env()?
        .build()?;

    // Environment should override file
    assert_eq!(config.api.api_key, Some("env-key".to_string()));
    assert_eq!(config.api.base_url, Some("https://env.api.com".to_string()));
    match &config.api.provider_config {
        ProviderConfig::Mock(_) => {}
        _ => panic!("Expected Mock provider after environment override"),
    }

    Ok(())
}

#[test]
fn test_provider_config_anthropic() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let config_content = r#"
        [api]
        api_key = "test-key"
        
        [api.provider_config]
        type = "anthropic"
        model = "claude-2"
        max_tokens = 1000
    "#;
    fs::write(&config_path, config_content)?;

    let config = Config::builder().from_file(&config_path)?.build()?;

    match &config.api.provider_config {
        ProviderConfig::Anthropic(cfg) => {
            assert_eq!(cfg.model, "claude-2");
            assert_eq!(cfg.max_tokens, 1000);
            assert!(cfg.parameters.is_empty());
        }
        _ => panic!("Expected Anthropic provider"),
    }

    Ok(())
}

#[test]
fn test_provider_config_openai() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let config_content = r#"
        [api]
        api_key = "test-key"
        
        [api.provider_config]
        type = "openai"
        model = "gpt-4"
        max_tokens = 2000
        temperature = 0.7
    "#;
    fs::write(&config_path, config_content)?;

    let config = Config::builder().from_file(&config_path)?.build()?;

    match &config.api.provider_config {
        ProviderConfig::OpenAI(cfg) => {
            assert_eq!(cfg.model, "gpt-4");
            assert_eq!(cfg.max_tokens, 2000);
            assert_eq!(cfg.temperature, 0.7);
            assert!(cfg.parameters.is_empty());
        }
        _ => panic!("Expected OpenAI provider"),
    }

    Ok(())
}

#[test]
fn test_provider_config_mock() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let config_content = r#"
        [api]
        api_key = "test-key"
        
        [api.provider_config]
        type = "mock"
    "#;
    fs::write(&config_path, config_content)?;

    let config = Config::builder().from_file(&config_path)?.build()?;

    match &config.api.provider_config {
        ProviderConfig::Mock(cfg) => {
            assert!(cfg.parameters.is_empty());
        }
        _ => panic!("Expected Mock provider"),
    }

    Ok(())
}

#[test]
fn test_provider_config_validation() -> Result<()> {
    // Test invalid Anthropic config
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let config_content = r#"
        [api]
        api_key = "test-key"
        
        [api.provider_config]
        type = "anthropic"
        model = ""  # Invalid: empty model
        max_tokens = 0  # Invalid: zero tokens
    "#;
    fs::write(&config_path, config_content)?;

    let result = Config::builder().from_file(&config_path)?.build();
    assert!(result.is_err());

    // Test invalid OpenAI config
    let config_content = r#"
        [api]
        api_key = "test-key"
        
        [api.provider_config]
        type = "openai"
        model = "gpt-4"
        max_tokens = 2000
        temperature = 2.5  # Invalid: temperature too high
    "#;
    fs::write(&config_path, config_content)?;

    let result = Config::builder().from_file(&config_path)?.build();
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_builder_methods() -> Result<()> {
    let config = Config::builder()
        .with_api_key("test-key".to_string())
        .with_base_url("https://api.openai.com/v1".to_string())
        .with_provider_config(ProviderConfig::OpenAI(OpenAIConfig {
            model: "gpt-4".to_string(),
            max_tokens: 2000,
            temperature: 0.7,
            parameters: Default::default(),
        }))
        .with_requests_per_minute(60)
        .with_tokens_per_minute(40000)
        .with_warning_threshold(80)
        .with_critical_threshold(90)
        .with_resume_threshold(70)
        .with_pause_on_warning(true)
        .with_pause_on_critical(true)
        .build()?;

    assert_eq!(config.api.api_key, Some("test-key".to_string()));
    assert_eq!(
        config.api.base_url,
        Some("https://api.openai.com/v1".to_string())
    );
    match &config.api.provider_config {
        ProviderConfig::OpenAI(cfg) => {
            assert_eq!(cfg.model, "gpt-4");
            assert_eq!(cfg.max_tokens, 2000);
            assert_eq!(cfg.temperature, 0.7);
        }
        _ => panic!("Expected OpenAI provider"),
    }
    assert_eq!(config.limits.requests_per_minute, Some(60));
    assert_eq!(config.limits.tokens_per_minute, Some(40000));
    assert_eq!(config.thresholds.warning, 80);
    assert_eq!(config.thresholds.critical, 90);
    assert_eq!(config.thresholds.resume, 70);
    assert!(config.process.pause_on_warning);
    assert!(config.process.pause_on_critical);

    Ok(())
}
