use anyhow::{anyhow, Result};
use dialoguer::{Input, Select};
use reqwest::Client;
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

use crate::providers::config::{AnthropicConfig, MockConfig, OpenAIConfig, ProviderConfig};
use crate::Config;

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

    // Get environment variables first
    let provider_type =
        std::env::var("STRAINER_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
    let model = std::env::var("STRAINER_MODEL");

    // Set provider based on environment variable or default to Anthropic
    config.api.provider_config = match provider_type.to_lowercase().as_str() {
        "openai" => {
            let mut cfg = OpenAIConfig::default();
            if let Ok(model_val) = &model {
                cfg.model = model_val.to_string();
            }

            ProviderConfig::OpenAI(cfg)
        }
        "mock" => ProviderConfig::Mock(MockConfig::default()),
        _ => {
            // In non-interactive mode, use environment variable if set, otherwise use default
            ProviderConfig::Anthropic(AnthropicConfig {
                model: model.unwrap_or_else(|_| "claude-2".to_string()),
                ..AnthropicConfig::default()
            })
        }
    };

    // In non-interactive mode, use environment variable placeholder
    if std::env::var("STRAINER_API_KEY").is_ok() {
        config.api.api_key = Some("${STRAINER_API_KEY}".to_string());
    }

    config
}

/// Create configuration in interactive mode
async fn create_interactive_config() -> Result<Config> {
    let mut config = Config::default();

    println!("Initializing strainer configuration...\n");

    // Provider selection
    let providers = [
        (
            "Anthropic",
            ProviderConfig::Anthropic(AnthropicConfig::default()),
        ),
        ("OpenAI", ProviderConfig::OpenAI(OpenAIConfig::default())),
        (
            "Mock (Testing)",
            ProviderConfig::Mock(MockConfig::default()),
        ),
    ];
    let provider_names: Vec<_> = providers.iter().map(|(name, _)| *name).collect();

    let selected = Select::new()
        .with_prompt("Select API provider")
        .items(&provider_names)
        .default(0)
        .interact()?;

    config.api.provider_config = providers[selected].1.clone();

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
    match &mut config.api.provider_config {
        ProviderConfig::Anthropic(cfg) => {
            let model: String = Input::new()
                .with_prompt("Enter model name")
                .with_initial_text("claude-2")
                .interact_text()?;
            cfg.model = model;

            let max_tokens: String = Input::new()
                .with_prompt("Maximum tokens per response")
                .with_initial_text("100000")
                .interact_text()?;
            cfg.max_tokens = max_tokens.parse()?;
        }
        _ => unreachable!("Only Anthropic provider is supported"),
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
    use tempfile::tempdir;
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_create_non_interactive_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let config = create_non_interactive_config();
        let result = std::fs::write(&config_path, toml::to_string(&config).unwrap());
        assert!(result.is_ok());
        assert!(config_path.exists());
    }

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
            .and(header("x-api-key", "test-key"))
            .and(header("anthropic-version", "2023-06-01"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&mock_server)
            .await;

        let result = test_anthropic_api("test-key", &mock_server.uri()).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("API test failed: Unauthorized"));
    }

    #[tokio::test]
    async fn test_initialize_config_force_overwrite() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        // Create initial config
        std::fs::write(&config_path, "# test config").unwrap();

        let opts = InitOptions {
            config_path: Some(config_path.clone()),
            no_prompt: true,
            force: true,
        };

        let result = initialize_config(opts).await;
        assert!(result.is_ok());
        assert!(config_path.exists());
    }

    #[tokio::test]
    async fn test_initialize_config_existing_no_force() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        // Create initial config
        std::fs::write(&config_path, "# test config").unwrap();

        let opts = InitOptions {
            config_path: Some(config_path.clone()),
            no_prompt: true,
            force: false,
        };

        let result = initialize_config(opts).await;
        assert!(result.is_err());
        assert!(config_path.exists());
    }
}
