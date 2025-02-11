use std::collections::HashMap;
use strainer::providers::config::{AnthropicConfig, MockConfig, OpenAIConfig, ProviderConfig};

#[test]
fn test_anthropic_config_validation() {
    // Test valid configurations
    let valid_configs = vec![
        AnthropicConfig {
            model: "claude-2".to_string(),
            max_tokens: 1000,
            parameters: HashMap::new(),
        },
        AnthropicConfig {
            model: "claude-instant-1".to_string(),
            max_tokens: 500,
            parameters: {
                let mut params = HashMap::new();
                params.insert("temperature".to_string(), "0.7".to_string());
                params
            },
        },
    ];

    for config in valid_configs {
        let provider_config = ProviderConfig::Anthropic(config);
        assert!(
            provider_config.validate().is_ok(),
            "Valid Anthropic config should pass validation"
        );
    }

    // Test invalid configurations
    let invalid_configs = vec![
        AnthropicConfig {
            model: "".to_string(),
            max_tokens: 1000,
            parameters: HashMap::new(),
        },
        AnthropicConfig {
            model: "claude-2".to_string(),
            max_tokens: 0,
            parameters: HashMap::new(),
        },
    ];

    for config in invalid_configs {
        let provider_config = ProviderConfig::Anthropic(config);
        assert!(
            provider_config.validate().is_err(),
            "Invalid Anthropic config should fail validation"
        );
    }
}

#[test]
fn test_openai_config_validation() {
    // Test valid configurations
    let valid_configs = vec![
        OpenAIConfig {
            model: "gpt-4".to_string(),
            max_tokens: 2000,
            temperature: 0.7,
            parameters: HashMap::new(),
        },
        OpenAIConfig {
            model: "gpt-3.5-turbo".to_string(),
            max_tokens: 1000,
            temperature: 0.0,
            parameters: {
                let mut params = HashMap::new();
                params.insert("presence_penalty".to_string(), "0.5".to_string());
                params
            },
        },
    ];

    for config in valid_configs {
        let provider_config = ProviderConfig::OpenAI(config);
        assert!(
            provider_config.validate().is_ok(),
            "Valid OpenAI config should pass validation"
        );
    }

    // Test invalid configurations
    let invalid_configs = vec![
        OpenAIConfig {
            model: "".to_string(),
            max_tokens: 2000,
            temperature: 0.7,
            parameters: HashMap::new(),
        },
        OpenAIConfig {
            model: "gpt-4".to_string(),
            max_tokens: 0,
            temperature: 0.7,
            parameters: HashMap::new(),
        },
        OpenAIConfig {
            model: "gpt-4".to_string(),
            max_tokens: 2000,
            temperature: 2.5,
            parameters: HashMap::new(),
        },
    ];

    for config in invalid_configs {
        let provider_config = ProviderConfig::OpenAI(config);
        assert!(
            provider_config.validate().is_err(),
            "Invalid OpenAI config should fail validation"
        );
    }
}

#[test]
fn test_mock_config_validation() {
    // Test various mock configurations
    let configs = vec![
        MockConfig {
            parameters: HashMap::new(),
        },
        MockConfig {
            parameters: {
                let mut params = HashMap::new();
                params.insert("test_key".to_string(), "test_value".to_string());
                params
            },
        },
    ];

    for config in configs {
        let provider_config = ProviderConfig::Mock(config);
        assert!(
            provider_config.validate().is_ok(),
            "Mock config should always pass validation"
        );
    }
}

#[test]
fn test_provider_config_serialization() {
    use serde_json;

    // Test Anthropic serialization
    let anthropic_config = ProviderConfig::Anthropic(AnthropicConfig::default());
    let json = serde_json::to_string(&anthropic_config).unwrap();
    let deserialized: ProviderConfig = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, ProviderConfig::Anthropic(_)));

    // Test OpenAI serialization
    let openai_config = ProviderConfig::OpenAI(OpenAIConfig::default());
    let json = serde_json::to_string(&openai_config).unwrap();
    let deserialized: ProviderConfig = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, ProviderConfig::OpenAI(_)));

    // Test Mock serialization
    let mock_config = ProviderConfig::Mock(MockConfig::default());
    let json = serde_json::to_string(&mock_config).unwrap();
    let deserialized: ProviderConfig = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, ProviderConfig::Mock(_)));
}
