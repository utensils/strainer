use std::str::FromStr;
use strainer::providers::config::ProviderConfig;

#[test]
fn test_old_to_new_config_migration() {
    // Test migration from old string-based format to new enum-based format
    let old_config = r#"
        [api]
        provider = "anthropic"
        [api.provider_specific]
        model = "claude-2"
        max_tokens = 1000
        temperature = "0.7"
    "#;

    let new_config = r#"
        [api]
        type = "anthropic"
        model = "claude-2"
        max_tokens = 1000
        parameters = { temperature = "0.7" }
    "#;

    // Parse both configs
    let old: toml::Value = toml::from_str(old_config).unwrap();
    let new: toml::Value = toml::from_str(new_config).unwrap();

    // Verify old config can be converted to ProviderConfig
    let provider = old
        .get("api")
        .and_then(|api| api.get("provider"))
        .and_then(|p| p.as_str())
        .unwrap();
    let provider_config = ProviderConfig::from_str(provider).unwrap();

    match provider_config {
        ProviderConfig::Anthropic(config) => {
            assert_eq!(config.model, "claude-2");
            assert_eq!(config.max_tokens, 1000);
        }
        _ => panic!("Expected Anthropic provider"),
    }

    // Verify new config format
    let api = new.get("api").unwrap();
    assert_eq!(api.get("type").unwrap().as_str().unwrap(), "anthropic");
    assert_eq!(api.get("model").unwrap().as_str().unwrap(), "claude-2");
    assert_eq!(api.get("max_tokens").unwrap().as_integer().unwrap(), 1000);
}

#[test]
fn test_openai_config_migration() {
    // Test migration for OpenAI config
    let old_config = r#"
        [api]
        provider = "openai"
        [api.provider_specific]
        model = "gpt-4"
        max_tokens = 2000
        temperature = 0.7
    "#;

    let new_config = r#"
        [api]
        type = "openai"
        model = "gpt-4"
        max_tokens = 2000
        temperature = 0.7
    "#;

    // Parse both configs
    let old: toml::Value = toml::from_str(old_config).unwrap();
    let new: toml::Value = toml::from_str(new_config).unwrap();

    // Verify old config can be converted
    let provider = old
        .get("api")
        .and_then(|api| api.get("provider"))
        .and_then(|p| p.as_str())
        .unwrap();
    let provider_config = ProviderConfig::from_str(provider).unwrap();

    match provider_config {
        ProviderConfig::OpenAI(config) => {
            assert_eq!(config.model, "gpt-4");
            assert_eq!(config.max_tokens, 2000);
            assert_eq!(config.temperature, 0.7);
        }
        _ => panic!("Expected OpenAI provider"),
    }

    // Verify new config format
    let api = new.get("api").unwrap();
    assert_eq!(api.get("type").unwrap().as_str().unwrap(), "openai");
    assert_eq!(api.get("model").unwrap().as_str().unwrap(), "gpt-4");
    assert_eq!(api.get("max_tokens").unwrap().as_integer().unwrap(), 2000);
    assert_eq!(api.get("temperature").unwrap().as_float().unwrap(), 0.7);
}

#[test]
fn test_config_parameters_migration() {
    // Test migration of additional parameters
    let new_config = r#"
        [api]
        type = "anthropic"
        model = "claude-2"
        max_tokens = 1000
        parameters = { custom_param = "value", another_param = "42" }
    "#;

    // Parse new config
    let new: toml::Value = toml::from_str(new_config).unwrap();

    // Verify parameters are correctly structured in new format
    let new_params = new
        .get("api")
        .and_then(|api| api.get("parameters"))
        .unwrap()
        .as_table()
        .unwrap();

    assert!(new_params.contains_key("custom_param"));
    assert!(new_params.contains_key("another_param"));
    assert_eq!(
        new_params.get("custom_param").unwrap().as_str().unwrap(),
        "value"
    );
    assert_eq!(
        new_params.get("another_param").unwrap().as_str().unwrap(),
        "42"
    );
}
