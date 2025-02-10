use anyhow::{anyhow, Result};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use reqwest::Client;
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

use crate::config::Config;

const ANTHROPIC_TEST_PROMPT: &str = "Say hello";

pub struct InitOptions {
    pub config_path: Option<PathBuf>,
    pub no_prompt: bool,
    pub force: bool,
}

async fn test_anthropic_api(api_key: &str, base_url: &str) -> Result<()> {
    let client = Client::new();
    
    let response = client
        .post(format!("{}/messages", base_url))
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

pub async fn initialize_config(opts: InitOptions) -> Result<()> {
    // Default path if none specified
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("strainer");
    let config_path = opts.config_path.unwrap_or_else(|| config_dir.join("config.toml"));
    
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
    
    let mut config = Config::default();
    
    if !opts.no_prompt {
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
        
        let api_key_value = if api_key.starts_with("${") && api_key.ends_with("}") {
            std::env::var(&api_key[2..api_key.len()-1]).ok()
        } else {
            Some(api_key.clone())
        };
        
        // Test API key if available
        if let Some(key) = api_key_value {
            print!("Testing API key... ");
            match test_anthropic_api(&key, &config.api.base_url.clone().unwrap_or_else(|| "https://api.anthropic.com/v1".to_string())).await {
                Ok(_) => println!("✓ Success"),
                Err(e) => {
                    println!("✗ Failed");
                    return Err(anyhow!("API key validation failed: {}", e));
                }
            }
        }
        
        config.api.api_key = Some(api_key);
        
        // Provider specific settings
        match config.api.provider.as_str() {
            "anthropic" => {
                let model: String = Input::new()
                    .with_prompt("Enter model name")
                    .with_initial_text("claude-2")
                    .interact_text()?;
                
                config.api.provider_specific.insert(
                    "model".to_string(), 
                    serde_json::Value::String(model)
                );
                
                let max_tokens: String = Input::new()
                    .with_prompt("Maximum tokens per response")
                    .with_initial_text("100000")
                    .interact_text()?;
                
                config.api.provider_specific.insert(
                    "max_tokens".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_str_radix(&max_tokens, 10)?)
                );
            }
            _ => {}
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
    }
    
    // Write the config file
    let toml = toml::to_string_pretty(&config)?;
    std::fs::write(&config_path, toml)?;
    
    println!("\nConfiguration created at: {}", config_path.display());
    Ok(())
}