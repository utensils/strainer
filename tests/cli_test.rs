use anyhow::Result;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
#[allow(unused_imports)]
use std::time::Duration;
use tempfile::tempdir;
use tokio::process::Command as TokioCommand;
#[allow(unused_imports)]
use tokio::time::sleep;

// Helper function to create a test binary
fn create_test_binary(dir: &std::path::Path) -> Result<std::path::PathBuf> {
    let test_binary = dir.join("test_process");
    fs::write(
        &test_binary,
        r#"#!/bin/sh
        trap "exit 0" TERM
        while true; do
            sleep 1
        done
        "#,
    )?;
    let mut perms = fs::metadata(&test_binary)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&test_binary, perms)?;
    Ok(test_binary)
}

// Helper function to run the command to avoid rebuilding for each test
async fn run_strainer_command(
    args: &[&str],
    test_dir: &tempfile::TempDir,
) -> Result<std::process::Output> {
    // Get the absolute path to the binary
    let current_dir = env::current_dir()?;
    eprintln!("Current directory: {current_dir:?}");
    let binary_path = current_dir.join("target/debug/strainer");
    eprintln!("Looking for binary at: {binary_path:?}");
    let binary_path = binary_path.canonicalize()?;
    eprintln!("Canonicalized binary path: {binary_path:?}");

    let mut cmd = TokioCommand::new(binary_path);
    cmd.args(args);
    cmd.envs(std::env::vars());
    cmd.current_dir(test_dir.path());
    Ok(cmd.output().await?)
}

fn spawn_strainer_command(
    args: &[&str],
    test_dir: &tempfile::TempDir,
) -> anyhow::Result<tokio::process::Child> {
    // Get the absolute path to the binary
    let binary_path = std::env::current_dir()?
        .join("target/debug/strainer")
        .canonicalize()?;

    let mut cmd = tokio::process::Command::new(binary_path);
    cmd.args(args);
    cmd.envs(std::env::vars());
    cmd.current_dir(test_dir.path());
    Ok(cmd.spawn()?)
}

#[tokio::test]
async fn test_run_command_basic() -> Result<()> {
    let test_dir = tempdir()?;
    let output = run_strainer_command(
        &[
            "run",
            "--api-key",
            "test_key",
            "--api",
            "mock",
            "--",
            "echo",
            "test",
        ],
        &test_dir,
    )
    .await?;

    assert!(output.status.success(), "Command failed: {output:?}");
    Ok(())
}

#[tokio::test]
async fn test_run_command_rate_limits() -> anyhow::Result<()> {
    let test_dir = tempdir()?;
    let output = run_strainer_command(
        &[
            "run",
            "--api-key",
            "test_key",
            "--api",
            "mock",
            "--requests-per-minute",
            "60",
            "--tokens-per-minute",
            "120",
            "--input-tokens-per-minute",
            "100",
            "--",
            "echo",
            "test_rate_limits",
        ],
        &test_dir,
    )
    .await?;

    // Since echo runs and exits successfully, we expect a success status
    assert!(
        output.status.success(),
        "Expected run command to succeed with valid command args"
    );
    Ok(())
}

#[tokio::test]
async fn test_run_command_process_control() -> anyhow::Result<()> {
    let test_dir = tempdir()?;

    // Create a test binary that will run indefinitely
    let test_binary = create_test_binary(test_dir.path())?;

    // Ensure the file is properly created before trying to execute it
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut child = spawn_strainer_command(
        &[
            "run",
            "--api-key",
            "test_key",
            "--api",
            "mock",
            "--",
            test_binary.to_str().unwrap(),
        ],
        &test_dir,
    )?;

    // Give it a short time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Kill the process
    child.kill().await?;
    let status = child.wait().await?;

    // On Unix systems, when a process is killed, it typically exits with a signal status
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert!(
            status.signal().is_some(),
            "Process should have been killed by a signal"
        );
    }

    #[cfg(not(unix))]
    {
        assert!(
            !status.success(),
            "Process should not have exited successfully"
        );
    }

    // Give the system a moment to fully clean up
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(())
}

#[tokio::test]
async fn test_run_command_invalid() -> Result<()> {
    let test_dir = tempdir()?;
    let output = run_strainer_command(
        &["run", "--api-key", "test_key", "--api", "mock"],
        &test_dir,
    )
    .await?;

    // Should fail because no command was provided
    assert!(!output.status.success());
    Ok(())
}

#[tokio::test]
async fn test_watch_command() -> anyhow::Result<()> {
    // Create a temporary directory for our test processes
    let test_dir = tempdir()?;

    // Create our test binary
    let test_binary = create_test_binary(test_dir.path())?;

    // Ensure the file is properly created before trying to execute it
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Start our test process
    let mut child = tokio::process::Command::new(&test_binary)
        .current_dir(test_dir.path())
        .spawn()?;

    let pid = child.id().expect("Failed to get process ID");

    // Give the process a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Run the watch command
    let output = run_strainer_command(
        &[
            "watch",
            "--pid",
            &pid.to_string(),
            "--api-key",
            "test_key",
            "--api",
            "mock",
        ],
        &test_dir,
    )
    .await?;

    // With the implemented watch command, if our test process is running, it should succeed
    assert!(
        output.status.success(),
        "Expected watch command to succeed if process {pid} is running"
    );

    // Clean up our test process
    child.kill().await?;
    child.wait().await?;

    // Give the system a moment to fully clean up
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(())
}
