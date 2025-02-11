use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;
use strainer::config::Config;
use strainer::providers;
use strainer::providers::rate_limiter::RateLimiter;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use crate::cli::{Cli, Commands};

mod cli;
mod init;
mod process;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging based on CLI options
    let filter = if cli.verbose { "debug" } else { &cli.log_level };

    let subscriber = fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    if cli.log_format == "json" {
        subscriber.json().init();
    } else {
        subscriber.init();
    }

    // Handle init command early as it doesn't need config loading
    if let Commands::Init {
        config,
        no_prompt,
        force,
    } = cli.command
    {
        return init::initialize_config(init::InitOptions {
            config_path: config,
            no_prompt,
            force,
        })
        .await;
    }

    // Load configuration from file and CLI args
    let base_config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            // Allow load to fail if using CLI args
            if cli.command.api_key().is_none() {
                return Err(e);
            }
            Config::default()
        }
    };
    let cli_config = Config {
        limits: strainer::config::RateLimits {
            requests_per_minute: cli.command.requests_per_minute(),
            tokens_per_minute: cli.command.tokens_per_minute(),
            input_tokens_per_minute: cli.command.input_tokens_per_minute(),
        },
        thresholds: strainer::config::Thresholds {
            warning: cli.command.warning_threshold(),
            critical: cli.command.critical_threshold(),
            resume: cli.command.resume_threshold(),
        },
        backoff: strainer::config::BackoffConfig {
            min_seconds: cli.command.min_backoff(),
            max_seconds: cli.command.max_backoff(),
        },
        process: strainer::config::ProcessConfig {
            pause_on_warning: cli.command.pause_on_warning(),
            pause_on_critical: cli.command.pause_on_critical(),
        },
        api: strainer::config::ApiConfig {
            provider: cli.command.api().to_string(),
            api_key: cli.command.api_key(),
            base_url: Some(cli.command.api_base_url().to_string()),
            provider_specific: HashMap::default(),
        },
        ..Default::default()
    };
    let mut final_config = base_config;
    final_config.merge(cli_config);
    final_config.validate()?;

    match cli.command {
        Commands::Run { command, .. } => run_command(command, final_config).await,
        Commands::Watch { pid, .. } => watch_process(pid, final_config),
        Commands::Init { .. } => unreachable!(), // Already handled above
    }
}

async fn run_command(command: Vec<String>, config: Config) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("No command specified");
    }

    // Create provider and rate limiter
    let provider = providers::create_provider(&config.api)?;
    let mut rate_limiter =
        RateLimiter::new(config.limits, config.thresholds, config.backoff, provider);

    // Start the process
    let (controller, mut child) = process::ProcessController::from_command(&command)?;
    info!("Started process with PID {}", child.id());

    // Monitor process and rate limits
    loop {
        let (proceed, backoff) = rate_limiter.check_limits()?;

        if !proceed {
            if config.process.pause_on_critical {
                info!("Rate limit critical threshold reached, pausing process");
                controller.pause()?;
            }
            tokio::time::sleep(backoff).await;
            if config.process.pause_on_critical {
                info!("Resuming process after backoff");
                controller.resume()?;
            }
            continue;
        }

        // Check if process is still running
        match child.try_wait()? {
            Some(status) => {
                info!("Process exited with status {}", status);
                return Ok(());
            }
            None => tokio::time::sleep(std::time::Duration::from_secs(1)).await,
        }
    }
}

fn watch_process(pid: u32, _config: Config) -> Result<()> {
    // SAFETY: Process IDs on Unix systems are always positive and within i32 range
    // If this assumption is violated, we want to panic as it indicates a serious system issue
    #[allow(clippy::cast_possible_wrap)]
    let pid_i32 = pid as i32;
    let controller = process::ProcessController::new(pid_i32);
    if controller.is_running() {
        println!("Process {pid} is running");
        Ok(())
    } else {
        anyhow::bail!("Process {} is not running", pid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::Duration;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_run_command_empty() {
        let result = run_command(vec![], Config::default()).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No command specified"));
    }

    #[tokio::test]
    async fn test_run_command_success() {
        let mut config = Config::default();
        config.api.provider = "mock".to_string();

        let result = run_command(vec!["true".to_string()], config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_command_failure() {
        let mut config = Config::default();
        config.api.provider = "mock".to_string();

        let result = run_command(vec!["false".to_string()], config).await;
        assert!(result.is_ok()); // The command runs successfully but exits with non-zero
    }

    #[tokio::test]
    async fn test_run_command_with_rate_limits() {
        let mut config = Config::default();
        config.api.provider = "mock".to_string();
        config.limits.requests_per_minute = Some(1);
        config.thresholds.critical = 50;
        config.process.pause_on_critical = true;
        config.backoff.min_seconds = 1;
        config.backoff.max_seconds = 2;

        // Start a long-running process that we can control
        let mut child = Command::new("sleep")
            .arg("10")
            .spawn()
            .expect("Failed to start sleep command");

        // Run the command in a separate task so we can kill it after our test
        let config_clone = config.clone();
        let handle = tokio::spawn(async move {
            run_command(vec!["sleep".to_string(), "10".to_string()], config_clone).await
        });

        // Give it some time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Kill the process
        child.kill().expect("Failed to kill process");
        let _ = child.wait();

        // Wait for our command to finish
        let result = handle.await.expect("Task panicked");
        assert!(result.is_ok());
    }

    #[test]
    fn test_watch_process_not_running() {
        let result = watch_process(1, Config::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not running"));
    }

    #[test]
    fn test_watch_process_running() {
        let child = Command::new("sleep")
            .arg("1")
            .spawn()
            .expect("Failed to start sleep command");

        let result = watch_process(child.id(), Config::default());
        assert!(result.is_ok());

        let _ = child.wait_with_output();
    }

    #[tokio::test]
    async fn test_main_init_command() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let args = vec![
            "strainer",
            "init",
            "--config",
            config_path.to_str().unwrap(),
            "--no-prompt",
            "--force",
        ];

        let result = Cli::try_parse_from(args).map(|cli| cli.command);
        assert!(matches!(
            result.unwrap(),
            Commands::Init {
                config: Some(_),
                no_prompt: true,
                force: true
            }
        ));
    }

    #[tokio::test]
    async fn test_main_run_command() {
        let args = vec!["strainer", "run", "--", "true"];

        let result = Cli::try_parse_from(args).map(|cli| cli.command);
        assert!(matches!(result.unwrap(), Commands::Run { .. }));
    }

    #[tokio::test]
    async fn test_main_watch_command() {
        let args = vec!["strainer", "watch", "--pid", "1234"];

        let result = Cli::try_parse_from(args).map(|cli| cli.command);
        assert!(matches!(result.unwrap(), Commands::Watch { pid: 1234, .. }));
    }
}
