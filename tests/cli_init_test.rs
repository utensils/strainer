use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_init_command_creates_config() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");

    let mut cmd = Command::cargo_bin("strainer")?;
    cmd.arg("init")
        .arg("--no-prompt")
        .arg("--config")
        .arg(config_path.as_os_str());

    cmd.assert().success();

    assert!(config_path.exists());
    Ok(())
}

#[test]
fn test_init_command_fails_on_existing_config() -> anyhow::Result<()> {
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

#[test]
fn test_init_command_force_override() -> anyhow::Result<()> {
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
    assert!(config_content.contains("provider"));
    Ok(())
}

#[test]
fn test_init_with_env_vars() -> anyhow::Result<()> {
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

#[test]
fn test_init_command_help() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin("strainer")?;
    cmd.arg("init").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialize a new configuration"));

    Ok(())
}
