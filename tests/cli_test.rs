use anyhow::Result;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;
use tempfile::tempdir;
use tokio::process::Command as TokioCommand;

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
    let binary_path = env::var("CARGO_BIN_EXE_strainer").unwrap_or_else(|_| {
        // Fallback for running tests directly without cargo
        let generic_path = env::current_dir()
            .unwrap()
            .join("target/debug/strainer");
        let platform_path = env::current_dir()
            .unwrap()
            .join("target/x86_64-unknown-linux-gnu/debug/strainer");
            
        if platform_path.exists() {
            platform_path.display().to_string()
        } else if generic_path.exists() {
            generic_path.display().to_string()
        } else {
            panic!("Could not find strainer binary in either {:?} or {:?}", generic_path, platform_path);
        }
    });

    let mut cmd = TokioCommand::new(binary_path);
    cmd.args(args);
    cmd.envs(std::env::vars());
    cmd.current_dir(test_dir.path());
    cmd.stderr(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    Ok(cmd.output().await?)
}

fn spawn_strainer_command(
    args: &[&str],
    test_dir: &tempfile::TempDir,
) -> anyhow::Result<tokio::process::Child> {
    let binary_path = env::var("CARGO_BIN_EXE_strainer").unwrap_or_else(|_| {
        // Fallback for running tests directly without cargo
        let generic_path = env::current_dir()
            .unwrap()
            .join("target/debug/strainer");
        let platform_path = env::current_dir()
            .unwrap()
            .join("target/x86_64-unknown-linux-gnu/debug/strainer");
            
        if platform_path.exists() {
            platform_path.display().to_string()
        } else if generic_path.exists() {
            generic_path.display().to_string()
        } else {
            panic!("Could not find strainer binary in either {:?} or {:?}", generic_path, platform_path);
        }
    });

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
            "--requests-per-minute",
            "100",
            "--tokens-per-minute",
            "1000",
            "--input-tokens-per-minute",
            "500",
            "--warning-threshold",
            "30",
            "--critical-threshold",
            "50",
            "--resume-threshold",
            "25",
            "--min-backoff",
            "1",
            "--max-backoff",
            "5",
            "--",
            "true",
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
            "true",
        ],
        &test_dir,
    )
    .await?;

    assert!(
        output.status.success(),
        "Expected run command to succeed with valid command args"
    );
    Ok(())
}

#[tokio::test]
async fn test_run_command_invalid() -> Result<()> {
    let test_dir = tempdir()?;

    // Set RUST_LOG to prevent tracing initialization in the binary
    std::env::set_var("RUST_LOG", "error");

    let output = run_strainer_command(&["run", "--"], &test_dir).await?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error: No command specified"),
        "Expected error message not found in stderr: {stderr}"
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

    // Give it some time to start
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
async fn test_watch_command() -> anyhow::Result<()> {
    let test_dir = tempdir()?;
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
