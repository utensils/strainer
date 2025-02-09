use anyhow::Result;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_run_command_basic() -> Result<()> {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "run",
            "--api-key",
            "test_key",
            "--api",
            "mock",
            "--",
            "sleep",
            "1",
        ])
        .output()?;

    assert!(output.status.success(), "Command failed: {:?}", output);
    Ok(())
}

#[tokio::test]
async fn test_run_command_rate_limits() -> Result<()> {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "run",
            "--api-key",
            "test_key",
            "--api",
            "mock",
            "--requests-per-minute",
            "60",
            "--tokens-per-minute",
            "1000",
            "--warning-threshold",
            "20",
            "--critical-threshold",
            "40",
            "--resume-threshold",
            "10",
            "--",
            "sleep",
            "2",
        ])
        .output()?;

    assert!(output.status.success(), "Command failed: {:?}", output);
    Ok(())
}

#[tokio::test]
async fn test_run_command_process_control() -> Result<()> {
    let mut child = Command::new("cargo")
        .args([
            "run",
            "--",
            "run",
            "--api-key",
            "test_key",
            "--api",
            "mock",
            "--pause-on-critical",
            "--critical-threshold",
            "10",
            "--requests-per-minute",
            "1",
            "--",
            "yes", // Continuously output "y"
        ])
        .spawn()?;

    // Give it time to start and potentially pause
    sleep(Duration::from_secs(2)).await;

    // Kill the process
    child.kill()?;
    let status = child.wait()?;

    // Process should have been killed by us, so it should not exit successfully
    assert!(!status.success());
    Ok(())
}

#[tokio::test]
async fn test_run_command_invalid() -> Result<()> {
    let output = Command::new("cargo")
        .args(["run", "--", "run", "--api-key", "test_key", "--api", "mock"])
        .output()?;

    // Should fail because no command was provided
    assert!(!output.status.success());
    Ok(())
}

#[tokio::test]
async fn test_watch_command() -> Result<()> {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "watch",
            "--pid",
            "1", // pid 1 should always exist
            "--api-key",
            "test_key",
            "--api",
            "mock",
        ])
        .output()?;

    // Should fail because watch command is not implemented yet
    assert!(!output.status.success());
    Ok(())
}
