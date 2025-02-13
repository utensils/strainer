use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::fs;
use strainer::config::Config;
use strainer::init::{initialize_config, InitOptions};
use strainer::providers::config::{OpenAIConfig, ProviderConfig};
use tempfile::tempdir;

mod common;
use common::{DirGuard, EnvGuard};

#[test]
fn test_config_from_file() -> Result<()> {
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
        api_key = "test-key"
        base_url = "https://custom.api.com"
        type = "anthropic"
        model = "claude-2"
        max_tokens = 1000
        
        [limits]
        requests_per_minute = 60
        tokens_per_minute = 100000
        input_tokens_per_minute = 50000

        [thresholds]
        warning = 80
        critical = 90
        resume = 70

        [backoff]
        min_seconds = 1
        max_seconds = 60

        [process]
        pause_on_warning = false
        pause_on_critical = true

        [logging]
        level = "info"
        format = "text"
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
    assert_eq!(config.limits.tokens_per_minute, Some(100_000));
    assert_eq!(config.limits.input_tokens_per_minute, Some(50_000));
    assert_eq!(config.thresholds.warning, 80);
    assert_eq!(config.thresholds.critical, 90);
    assert_eq!(config.thresholds.resume, 70);
    assert_eq!(config.backoff.min_seconds, 1);
    assert_eq!(config.backoff.max_seconds, 60);
    assert!(!config.process.pause_on_warning);
    assert!(config.process.pause_on_critical);
    assert_eq!(config.logging.level, "info");
    assert_eq!(config.logging.format, "text");

    Ok(())
}

#[test]
fn test_config_from_env() -> Result<()> {
    // Create guards first to ensure cleanup
    let (_dir_guard, _env_guard) = {
        let dir_guard = DirGuard::new()?;
        let env_guard = EnvGuard::new(vec![
            "STRAINER_PROVIDER_TYPE",
            "STRAINER_BASE_URL",
            "STRAINER_API_KEY",
        ]);
        (dir_guard, env_guard)
    };

    // Create an isolated directory for the test
    let dir = tempdir()?;
    env::set_current_dir(dir.path())?;

    // Set test environment variables after guards are created
    env::set_var("STRAINER_PROVIDER_TYPE", "mock");
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");
    env::set_var("STRAINER_API_KEY", "env-key");

    // Debug: Print environment variables
    println!("API_KEY: {:?}", env::var("STRAINER_API_KEY"));
    println!("PROVIDER_TYPE: {:?}", env::var("STRAINER_PROVIDER_TYPE"));
    println!("BASE_URL: {:?}", env::var("STRAINER_BASE_URL"));

    let config = Config::builder().from_env()?.build()?;

    // Debug: Print config values
    println!("Config API_KEY: {:?}", config.api.api_key);
    println!("Config BASE_URL: {:?}", config.api.base_url);
    println!("Config PROVIDER: {:?}", config.api.provider_config);

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
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let config_content = r#"
        [api]
        api_key = "file-key"
        base_url = "https://file.api.com"
        provider = "anthropic"

        [provider.anthropic]
        model = "claude-2"
        max_tokens = 1000
    "#;
    fs::write(&config_path, config_content)?;

    // Create guards after tempdir to ensure proper cleanup order
    let dir_guard = DirGuard::new()?;
    let env_guard = EnvGuard::new(vec![
        "STRAINER_PROVIDER_TYPE",
        "STRAINER_BASE_URL",
        "STRAINER_API_KEY",
    ]);

    // Set environment variables
    env::set_var("STRAINER_API_KEY", "env-key");
    env::set_var("STRAINER_PROVIDER_TYPE", "mock");
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");

    // Change to the temp directory
    env::set_current_dir(dir.path())?;

    // Load and verify config
    let config = Config::builder().from_file(&config_path)?.build()?;

    assert_eq!(config.api.api_key, Some("env-key".to_string()));
    assert_eq!(config.api.base_url, Some("https://env.api.com".to_string()));
    match &config.api.provider_config {
        ProviderConfig::Mock(_) => {}
        _ => panic!("Expected Mock provider"),
    }

    // Guards will be dropped first, restoring the environment,
    // then tempdir will be dropped
    drop(dir_guard);
    drop(env_guard);
    Ok(())
}

#[test]
fn test_provider_config_anthropic() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let config_content = r#"
        [api]
        api_key = "test-key"
        type = "anthropic"
        model = "claude-2"
        max_tokens = 1000

        [limits]
        requests_per_minute = 60
        tokens_per_minute = 100000
        input_tokens_per_minute = 50000

        [thresholds]
        warning = 80
        critical = 90
        resume = 70

        [backoff]
        min_seconds = 1
        max_seconds = 60

        [process]
        pause_on_warning = false
        pause_on_critical = true

        [logging]
        level = "info"
        format = "text"
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
        type = "openai"
        model = "gpt-4"
        max_tokens = 2000

        [limits]
        requests_per_minute = 60
        tokens_per_minute = 100000
        input_tokens_per_minute = 50000

        [thresholds]
        warning = 80
        critical = 90
        resume = 70

        [backoff]
        min_seconds = 1
        max_seconds = 60

        [process]
        pause_on_warning = false
        pause_on_critical = true

        [logging]
        level = "info"
        format = "text"
    "#;
    fs::write(&config_path, config_content)?;

    let config = Config::builder().from_file(&config_path)?.build()?;

    match &config.api.provider_config {
        ProviderConfig::OpenAI(cfg) => {
            assert_eq!(cfg.model, "gpt-4");
            assert_eq!(cfg.max_tokens, 2000);
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
        type = "mock"

        [limits]
        requests_per_minute = 60
        tokens_per_minute = 100000
        input_tokens_per_minute = 50000

        [thresholds]
        warning = 80
        critical = 90
        resume = 70

        [backoff]
        min_seconds = 1
        max_seconds = 60

        [process]
        pause_on_warning = false
        pause_on_critical = true

        [logging]
        level = "info"
        format = "text"
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
fn test_invalid_anthropic_config_validation() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let config_content = r#"
        [api]
        api_key = "test-key"
        type = "anthropic"
        model = ""  # Invalid: empty model
        max_tokens = 0  # Invalid: zero tokens

        [limits]
        requests_per_minute = 60
        tokens_per_minute = 100000
        input_tokens_per_minute = 50000

        [thresholds]
        warning = 80
        critical = 90
        resume = 70

        [backoff]
        min_seconds = 1
        max_seconds = 60

        [process]
        pause_on_warning = false
        pause_on_critical = true

        [logging]
        level = "info"
        format = "text"
    "#;
    fs::write(&config_path, config_content)?;

    let result = Config::builder().from_file(&config_path)?.build();
    assert!(
        result.is_err(),
        "Build should fail due to invalid Anthropic config"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("model must not be empty")
            || err.contains("max_tokens must be greater than 0"),
        "Error should mention invalid model or max_tokens"
    );

    Ok(())
}

#[test]
fn test_invalid_openai_config_validation() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let config_content = r#"
        [api]
        api_key = "test-key"
        type = "openai"
        model = ""  # Invalid: empty model
        max_tokens = 0  # Invalid: zero tokens

        [limits]
        requests_per_minute = 60
        tokens_per_minute = 100000
        input_tokens_per_minute = 50000

        [thresholds]
        warning = 80
        critical = 90
        resume = 70

        [backoff]
        min_seconds = 1
        max_seconds = 60

        [process]
        pause_on_warning = false
        pause_on_critical = true

        [logging]
        level = "info"
        format = "text"
    "#;
    fs::write(&config_path, config_content)?;

    let result = Config::builder().from_file(&config_path)?.build();
    assert!(
        result.is_err(),
        "Build should fail due to invalid OpenAI config"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("model must not be empty")
            || err.contains("max_tokens must be greater than 0"),
        "Error should mention invalid model or max_tokens"
    );

    Ok(())
}

#[test]
fn test_valid_anthropic_config_validation() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let config_content = r#"
        [api]
        api_key = "test-key"
        type = "anthropic"
        model = "claude-2"
        max_tokens = 1000

        [limits]
        requests_per_minute = 60
        tokens_per_minute = 100000
        input_tokens_per_minute = 50000

        [thresholds]
        warning = 80
        critical = 90
        resume = 70

        [backoff]
        min_seconds = 1
        max_seconds = 60

        [process]
        pause_on_warning = false
        pause_on_critical = true

        [logging]
        level = "info"
        format = "text"
    "#;
    fs::write(&config_path, config_content)?;

    let result = Config::builder().from_file(&config_path)?.build();
    assert!(
        result.is_ok(),
        "Build should succeed with valid Anthropic config"
    );

    Ok(())
}

#[test]
fn test_valid_openai_config_validation() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let config_content = r#"
        [api]
        api_key = "test-key"
        type = "openai"
        model = "gpt-4"
        max_tokens = 2000

        [limits]
        requests_per_minute = 60
        tokens_per_minute = 100000
        input_tokens_per_minute = 50000

        [thresholds]
        warning = 80
        critical = 90
        resume = 70

        [backoff]
        min_seconds = 1
        max_seconds = 60

        [process]
        pause_on_warning = false
        pause_on_critical = true

        [logging]
        level = "info"
        format = "text"
    "#;
    fs::write(&config_path, config_content)?;

    let result = Config::builder().from_file(&config_path)?.build();
    assert!(
        result.is_ok(),
        "Build should succeed with valid OpenAI config"
    );

    Ok(())
}

#[test]
fn test_mock_config_validation() -> Result<()> {
    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let config_content = r#"
        [api]
        type = "mock"

        [limits]
        requests_per_minute = 60
        tokens_per_minute = 100000
        input_tokens_per_minute = 50000

        [thresholds]
        warning = 80
        critical = 90
        resume = 70

        [backoff]
        min_seconds = 1
        max_seconds = 60

        [process]
        pause_on_warning = false
        pause_on_critical = true

        [logging]
        level = "info"
        format = "text"
    "#;
    fs::write(&config_path, config_content)?;

    let result = Config::builder().from_file(&config_path)?.build();
    assert!(result.is_ok(), "Build should succeed with Mock config");

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

            parameters: HashMap::default(),
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

#[test]
fn test_load_with_env_override() -> Result<()> {
    // Create guards first to ensure cleanup
    let (_dir_guard, _env_guard) = {
        let dir_guard = DirGuard::new()?;
        let env_guard = EnvGuard::new(vec![
            "STRAINER_PROVIDER_TYPE",
            "STRAINER_BASE_URL",
            "STRAINER_API_KEY",
        ]);
        (dir_guard, env_guard)
    };

    // Create an isolated directory for the test
    let dir = tempdir()?;
    env::set_current_dir(dir.path())?;

    // Set test environment variables after guards are created
    env::set_var("STRAINER_PROVIDER_TYPE", "mock");
    env::set_var("STRAINER_BASE_URL", "https://env.api.com");
    env::set_var("STRAINER_API_KEY", "env-key");

    // Load config from environment
    let config = Config::builder().from_env()?.build()?;

    // Environment should override defaults
    match config.api.provider_config {
        ProviderConfig::Mock(_) => {}
        _ => panic!("Expected Mock provider"),
    }
    assert_eq!(config.api.api_key, Some("env-key".to_string()));
    assert_eq!(config.api.base_url, Some("https://env.api.com".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_initialize_config_non_interactive() {
    let _dir_guard = DirGuard::new().unwrap();
    let _env_guard = EnvGuard::new(vec!["STRAINER_API_KEY", "STRAINER_MODEL"]);

    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let opts = InitOptions {
        config_path: Some(config_path.clone()),
        no_prompt: true,
        force: false,
    };

    env::set_var("STRAINER_API_KEY", "test-key");
    env::set_var("STRAINER_MODEL", "claude-3");

    let result = initialize_config(opts).await;
    assert!(result.is_ok());

    let config_str = fs::read_to_string(&config_path).unwrap();
    let config: Config = toml::from_str(&config_str).unwrap();

    match &config.api.provider_config {
        ProviderConfig::Anthropic(cfg) => {
            assert_eq!(cfg.model, "claude-3");
        }
        _ => panic!("Expected Anthropic provider"),
    }
    // When writing to file, we use the environment variable placeholder
    assert_eq!(config.api.api_key, Some("${STRAINER_API_KEY}".to_string()));
}

#[test]
fn test_config_merge() {
    let mut base = Config::default();
    let override_config = Config::default();

    // Merge configs
    base.merge(override_config);

    // Verify the merge
    assert_eq!(base.api.api_key, None);
}
