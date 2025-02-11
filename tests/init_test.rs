//! Tests for the initialization module
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

// Integration tests for the init command
#[tokio::test]
async fn test_init_command_creates_config() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");

    let mut cmd = Command::cargo_bin("strainer")?;
    cmd.arg("init")
        .arg("--no-prompt")
        .arg("--config")
        .arg(config_path.as_os_str());

    cmd.assert().success();

    let config_content = fs::read_to_string(&config_path)?;
    // Check for new format content
    assert!(config_content.contains("type = \"anthropic\""));
    assert!(config_path.exists());
    Ok(())
}

#[tokio::test]
async fn test_init_command_fails_on_existing_config() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");

    // Create initial config
    fs::write(&config_path, "# existing config")?;

    let mut cmd = Command::cargo_bin("strainer")?;
    cmd.arg("init")
        .arg("--no-prompt")
        .arg("--config")
        .arg(config_path.as_os_str());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    Ok(())
}

#[tokio::test]
async fn test_init_command_force_override() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");

    // Create initial config
    fs::write(&config_path, "# existing config")?;

    let mut cmd = Command::cargo_bin("strainer")?;
    cmd.arg("init")
        .arg("--no-prompt")
        .arg("--force")
        .arg("--config")
        .arg(config_path.as_os_str());

    cmd.assert().success();

    let config_content = fs::read_to_string(config_path)?;
    assert!(config_content.contains("type = \"anthropic\""));
    Ok(())
}

#[tokio::test]
async fn test_init_with_env_vars() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");

    // Set environment variables
    std::env::set_var("STRAINER_API_KEY", "test-key");
    std::env::set_var("STRAINER_MODEL", "claude-3");

    let mut cmd = Command::cargo_bin("strainer")?;
    cmd.arg("init")
        .arg("--no-prompt")
        .arg("--config")
        .arg(config_path.as_os_str())
        .env("STRAINER_API_KEY", "test-key")
        .env("STRAINER_MODEL", "claude-3");

    cmd.assert().success();

    let config_content = fs::read_to_string(config_path)?;
    assert!(config_content.contains("${STRAINER_API_KEY}"));
    assert!(config_content.contains("claude-3"));
    Ok(())
}

#[tokio::test]
async fn test_anthropic_api_validation() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(header("x-api-key", "test-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "test",
            "type": "message",
            "role": "assistant",
            "content": "Hello"
        })))
        .mount(&mock_server)
        .await;

    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");

    let mut cmd = Command::cargo_bin("strainer")?;
    cmd.arg("init")
        .arg("--no-prompt")
        .arg("--config")
        .arg(config_path.as_os_str())
        .env("STRAINER_BASE_URL", mock_server.uri())
        .env("STRAINER_API_KEY", "test-key");

    cmd.assert().success();

    let config_content = fs::read_to_string(config_path)?;
    assert!(config_content.contains("type = \"anthropic\""));
    assert!(config_content.contains("model = \"claude-2\""));
    Ok(())
}

#[tokio::test]
async fn test_openai_provider_config() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");

    let mut cmd = Command::cargo_bin("strainer")?;
    cmd.arg("init")
        .arg("--no-prompt")
        .arg("--config")
        .arg(config_path.as_os_str())
        .env("STRAINER_PROVIDER", "openai")
        .env("STRAINER_API_KEY", "test-key")
        .env("STRAINER_MODEL", "gpt-4");

    cmd.assert().success();

    let config_content = fs::read_to_string(config_path)?;
    assert!(config_content.contains("type = \"openai\""));
    assert!(config_content.contains("model = \"gpt-4\""));
    assert!(config_content.contains("temperature = 0.7"));
    Ok(())
}

#[tokio::test]
async fn test_mock_provider_config() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");

    let mut cmd = Command::cargo_bin("strainer")?;
    cmd.arg("init")
        .arg("--no-prompt")
        .arg("--config")
        .arg(config_path.as_os_str())
        .env("STRAINER_PROVIDER", "mock");

    cmd.assert().success();

    let config_content = fs::read_to_string(config_path)?;
    assert!(config_content.contains("type = \"mock\""));
    Ok(())
}

// Test fixtures
#[allow(dead_code)]
pub mod fixtures {
    use serde_json::json;

    #[must_use]
    pub fn anthropic_success_response() -> serde_json::Value {
        json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": "Hello",
            "model": "claude-2",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 5,
                "output_tokens": 2
            }
        })
    }

    #[must_use]
    pub fn anthropic_error_response() -> serde_json::Value {
        json!({
            "error": {
                "type": "authentication_error",
                "message": "Invalid API key"
            }
        })
    }

    #[must_use]
    pub fn sample_config_toml() -> String {
        r#"
[api]
api_key = "${ANTHROPIC_API_KEY}"
base_url = "https://api.anthropic.com/v1"

[api.provider]
type = "anthropic"
model = "claude-2"
max_tokens = 100000

[limits]
requests_per_minute = 60
tokens_per_minute = 100000
"#
        .to_string()
    }

    #[must_use]
    pub fn sample_openai_config_toml() -> String {
        r#"
[api]
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1"

[api.provider]
type = "openai"
model = "gpt-4"
max_tokens = 2000
temperature = 0.7

[limits]
requests_per_minute = 60
tokens_per_minute = 100000
"#
        .to_string()
    }

    #[must_use]
    pub fn sample_mock_config_toml() -> String {
        r#"
[api]
api_key = "mock-key"
base_url = "http://localhost:8080"

[api.provider]
type = "mock"

[limits]
requests_per_minute = 60
tokens_per_minute = 100000
"#
        .to_string()
    }
}
