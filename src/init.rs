use anyhow::{anyhow, Result};
use dialoguer::{Input, Select};
use reqwest::Client;
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

use super::Config;

const ANTHROPIC_TEST_PROMPT: &str = "Say hello";

pub struct InitOptions {
    pub config_path: Option<PathBuf>,
    pub no_prompt: bool,
    pub force: bool,
}

/// Test the Anthropic API connection with the provided credentials
///
/// # Arguments
/// * `api_key` - The API key to test
/// * `base_url` - The base URL of the Anthropic API
///
/// # Errors
/// Returns an error if:
/// * The API request fails to send
/// * The API returns a non-success status code
async fn test_anthropic_api(api_key: &str, base_url: &str) -> Result<()> {
    let client = Client::new();

    let response = client
        .post(format!("{base_url}/messages"))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&json!({
            "model": "claude-2",
            "max_tokens": 10,
            "messages": [{
                "role": "user",
                "content": ANTHROPIC_TEST_PROMPT
            }]
        }))
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        return Err(anyhow!("API test failed: {}", error));
    }

    Ok(())
}

/// Initialize the configuration file for the Strainer tool
///
/// # Arguments
/// * `opts` - The initialization options
///
/// # Errors
/// Returns an error if:
/// * The configuration file already exists and `force` is not set
/// * Failed to create the configuration directory
/// * Failed to write the configuration file
/// * API validation fails when testing credentials
///
/// # Panics
/// This function will panic if:
/// * Converting the `max_tokens` value to a JSON number fails
pub async fn initialize_config(opts: InitOptions) -> Result<()> {
    // Default path if none specified
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("strainer");
    let config_path = opts
        .config_path
        .unwrap_or_else(|| config_dir.join("config.toml"));

    // Check if config exists
    if config_path.exists() && !opts.force {
        return Err(anyhow!(
            "Config file already exists at {}. Use --force to overwrite.",
            config_path.display()
        ));
    }

    // Create config directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let config = if opts.no_prompt {
        create_non_interactive_config()
    } else {
        create_interactive_config().await?
    };

    // Write the config file
    let toml = toml::to_string_pretty(&config)?;
    std::fs::write(&config_path, toml)?;

    println!("\nConfiguration created at: {}", config_path.display());
    Ok(())
}

/// Create configuration in non-interactive mode
fn create_non_interactive_config() -> Config {
    let mut config = Config::default();

    // In non-interactive mode, check for environment variables
    if std::env::var("STRAINER_API_KEY").is_ok() {
        config.api.api_key = Some("${STRAINER_API_KEY}".to_string());
    }

    // Include any other environment-based settings
    if let Ok(model) = std::env::var("STRAINER_MODEL") {
        config
            .api
            .provider_specific
            .insert("model".to_string(), serde_json::Value::String(model));
    }

    config
}

/// Create configuration in interactive mode
async fn create_interactive_config() -> Result<Config> {
    let mut config = Config::default();

    println!("Initializing strainer configuration...\n");

    // Provider selection
    let providers = vec!["anthropic"];
    let provider = Select::new()
        .with_prompt("Select API provider")
        .items(&providers)
        .default(0)
        .interact()?;
    config.api.provider = providers[provider].to_string();

    // API key
    let api_key: String = Input::new()
        .with_prompt("Enter API key (or environment variable name)")
        .with_initial_text("${ANTHROPIC_API_KEY}")
        .interact_text()?;

    let api_key_value = if api_key.starts_with("${") && api_key.ends_with('}') {
        std::env::var(&api_key[2..api_key.len() - 1]).ok()
    } else {
        Some(api_key.clone())
    };

    // Test API key if available
    if let Some(key) = api_key_value {
        print!("Testing API key... ");
        match test_anthropic_api(
            &key,
            &config
                .api
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string()),
        )
        .await
        {
            Ok(()) => println!("✓ Success"),
            Err(e) => {
                println!("✗ Failed");
                return Err(anyhow!("API key validation failed: {}", e));
            }
        }
    }

    config.api.api_key = Some(api_key);

    // Provider specific settings
    if config.api.provider.as_str() == "anthropic" {
        let model: String = Input::new()
            .with_prompt("Enter model name")
            .with_initial_text("claude-2")
            .interact_text()?;

        config
            .api
            .provider_specific
            .insert("model".to_string(), serde_json::Value::String(model));

        let max_tokens: String = Input::new()
            .with_prompt("Maximum tokens per response")
            .with_initial_text("100000")
            .interact_text()?;

        let max_tokens_num = max_tokens.parse::<u32>()?;
        config.api.provider_specific.insert(
            "max_tokens".to_string(),
            serde_json::Value::Number(serde_json::Number::from(max_tokens_num)),
        );
    }

    // Rate limits
    let rpm: String = Input::new()
        .with_prompt("Requests per minute (leave empty for no limit)")
        .allow_empty(true)
        .interact_text()?;

    if !rpm.is_empty() {
        config.limits.requests_per_minute = Some(rpm.parse()?);
    }

    let tpm: String = Input::new()
        .with_prompt("Tokens per minute (leave empty for no limit)")
        .allow_empty(true)
        .interact_text()?;

    if !tpm.is_empty() {
        config.limits.tokens_per_minute = Some(tpm.parse()?);
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_anthropic_api_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .and(header("x-api-key", "test-key"))
            .and(header("anthropic-version", "2023-06-01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test",
                "content": "Hello"
            })))
            .mount(&mock_server)
            .await;

        let result = test_anthropic_api("test-key", &mock_server.uri()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_anthropic_api_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .and(header("x-api-key", "invalid-key"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "Invalid API key"
            })))
            .mount(&mock_server)
            .await;

        let result = test_anthropic_api("invalid-key", &mock_server.uri()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API test failed"));
    }

    #[tokio::test]
    async fn test_initialize_config_non_interactive() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        env::set_var("STRAINER_API_KEY", "test-key");
        env::set_var("STRAINER_MODEL", "test-model");

        let result = initialize_config(InitOptions {
            config_path: Some(config_path.clone()),
            no_prompt: true,
            force: false,
        })
        .await;

        assert!(result.is_ok());
        assert!(config_path.exists());

        let config_content = std::fs::read_to_string(config_path).unwrap();
        assert!(config_content.contains("${STRAINER_API_KEY}"));
        assert!(config_content.contains("test-model"));

        env::remove_var("STRAINER_API_KEY");
        env::remove_var("STRAINER_MODEL");
    }

    #[tokio::test]
    async fn test_initialize_config_force_overwrite() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Create initial config
        std::fs::write(&config_path, "test").unwrap();

        let result = initialize_config(InitOptions {
            config_path: Some(config_path.clone()),
            no_prompt: true,
            force: true,
        })
        .await;

        assert!(result.is_ok());
        assert!(config_path.exists());
        assert_ne!(std::fs::read_to_string(config_path).unwrap(), "test");
    }

    #[tokio::test]
    async fn test_initialize_config_existing_no_force() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Create initial config
        std::fs::write(&config_path, "test").unwrap();

        let result = initialize_config(InitOptions {
            config_path: Some(config_path),
            no_prompt: true,
            force: false,
        })
        .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Config file already exists"));
    }

    #[test]
    fn test_create_non_interactive_config() {
        env::set_var("STRAINER_API_KEY", "test-key");
        env::set_var("STRAINER_MODEL", "test-model");

        let config = create_non_interactive_config();

        assert_eq!(config.api.api_key, Some("${STRAINER_API_KEY}".to_string()));
        assert_eq!(
            config
                .api
                .provider_specific
                .get("model")
                .and_then(|v| v.as_str()),
            Some("test-model")
        );

        env::remove_var("STRAINER_API_KEY");
        env::remove_var("STRAINER_MODEL");
    }
}
