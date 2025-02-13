use anyhow::Result;
use clap::Parser;
use strainer::config::Config;
use strainer::providers;
use strainer::providers::config::{AnthropicConfig, MockConfig, OpenAIConfig, ProviderConfig};
use strainer::providers::rate_limiter::RateLimiter;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use strainer::cli::{Cli, Commands};
use strainer::process::ProcessController;
use strainer::{initialize_config, InitOptions};

use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging based on CLI options, but only if not already initialized
    if std::env::var("RUST_LOG").is_err() {
        let filter = if cli.verbose { "debug" } else { &cli.log_level };

        let subscriber = fmt()
            .with_env_filter(EnvFilter::new(filter))
            .with_target(false)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);

        if cli.log_format == "json" {
            let _ = subscriber.json().try_init();
        } else {
            let _ = subscriber.try_init();
        }
    }

    // Handle init command early as it doesn't need config loading
    if let Commands::Init {
        config,
        no_prompt,
        force,
    } = cli.command
    {
        return initialize_config(InitOptions {
            config_path: config,
            no_prompt,
            force,
        })
        .await;
    }

    // Check for empty command vector in Run command
    if let Commands::Run { ref command, .. } = cli.command {
        if command.is_empty() {
            eprintln!("Error: No command specified");
            anyhow::bail!("No command specified");
        }
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

    let cli_config = create_cli_config(&cli.command);
    let mut final_config = base_config;
    final_config.merge(cli_config);
    final_config.validate()?;

    let result = match cli.command {
        Commands::Run { command, .. } => run_command(command, final_config).await,
        Commands::Watch { pid, .. } => watch_process(pid, final_config),
        Commands::Init { .. } => unreachable!(), // Already handled above
    };

    if let Err(ref e) = result {
        eprintln!("{e}");
    }
    result
}

fn create_cli_config(cli: &Commands) -> Config {
    let provider_config = match cli.api() {
        "openai" => ProviderConfig::OpenAI(OpenAIConfig::default()),
        "mock" => ProviderConfig::Mock(MockConfig::default()),
        _ => ProviderConfig::Anthropic(AnthropicConfig::default()),
    };

    Config {
        limits: strainer::config::RateLimits {
            requests_per_minute: cli.requests_per_minute(),
            tokens_per_minute: cli.tokens_per_minute(),
            input_tokens_per_minute: cli.input_tokens_per_minute(),
        },
        thresholds: strainer::config::Thresholds {
            warning: cli.warning_threshold(),
            critical: cli.critical_threshold(),
            resume: cli.resume_threshold(),
        },
        backoff: strainer::config::BackoffConfig {
            min_seconds: cli.min_backoff(),
            max_seconds: cli.max_backoff(),
        },
        process: strainer::config::ProcessConfig {
            pause_on_warning: cli.pause_on_warning(),
            pause_on_critical: cli.pause_on_critical(),
        },
        api: strainer::config::ApiConfig {
            provider_config,
            api_key: cli.api_key(),
            base_url: Some(cli.api_base_url().to_string()),
            parameters: HashMap::default(),
        },
        ..Default::default()
    }
}

async fn run_command(command: Vec<String>, config: Config) -> Result<()> {
    // Check for empty command vector
    if command.is_empty() {
        anyhow::bail!("No command specified");
    }

    // Create provider and rate limiter
    let provider = providers::create_provider(&config.api)?;
    let mut rate_limiter = RateLimiter::new(config.thresholds, config.backoff, provider);

    // Start the process
    let (controller, mut child) = ProcessController::from_command(&command)?;
    info!("Started process with PID {}", child.id());

    // Monitor process and rate limits
    loop {
        // Check if process is still running first
        if let Some(status) = child.try_wait()? {
            info!("Process exited with status {status}");
            // If the process exited with a non-zero status, propagate the error
            if !status.success() {
                anyhow::bail!("Process exited with non-zero status: {status}");
            }
            return Ok(());
        }

        // Process is still running, check rate limits
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

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

fn watch_process(pid: u32, _config: Config) -> Result<()> {
    // SAFETY: Process IDs on Unix systems are always positive and within i32 range
    // If this assumption is violated, we want to panic as it indicates a serious system issue
    #[allow(clippy::cast_possible_wrap)]
    let pid_i32 = pid as i32;
    let controller = ProcessController::new(pid_i32);
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
    use crate::providers::config::MockConfig;
    use std::process::Command;
    use std::time::Duration;
    use strainer::cli::{Cli, Commands};
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
        config.api.provider_config = ProviderConfig::Mock(MockConfig::default());

        let result = run_command(vec!["true".to_string()], config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_command_failure() {
        let mut config = Config::default();
        config.api.provider_config = ProviderConfig::Mock(MockConfig::default());

        let result = run_command(vec!["false".to_string()], config).await;
        assert!(result.is_err()); // The command should fail because 'false' exits with non-zero
    }

    #[tokio::test]
    async fn test_run_command_with_rate_limits() {
        let mut config = Config::default();
        config.api.provider_config = ProviderConfig::Mock(MockConfig::default());
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
        ];

        let cli = Cli::parse_from(args);
        if let Commands::Init {
            config,
            no_prompt,
            force,
        } = cli.command
        {
            let result = strainer::initialize_config(strainer::InitOptions {
                config_path: config,
                no_prompt,
                force,
            })
            .await;
            assert!(result.is_ok());
            assert!(config_path.exists());
        } else {
            panic!("Expected Init command");
        }
    }

    #[tokio::test]
    async fn test_main_run_command() {
        let args = vec!["strainer", "run", "--api", "mock", "--", "true"];
        let cli = Cli::parse_from(args);
        match cli.command {
            Commands::Run { ref command, .. } => {
                let config = create_cli_config(&cli.command);
                let result = run_command(command.clone(), config).await;
                assert!(result.is_ok());
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[tokio::test]
    async fn test_main_watch_command() {
        let child = Command::new("sleep")
            .arg("1")
            .spawn()
            .expect("Failed to start sleep command");

        let pid_str = child.id().to_string();
        let args = vec!["strainer", "watch", "--api", "mock", "--pid", &pid_str];

        let cli = Cli::parse_from(args.clone());
        let pid = match &cli.command {
            Commands::Watch { pid, .. } => *pid,
            _ => panic!("Expected Watch command"),
        };

        let config = Config {
            api: strainer::config::ApiConfig {
                provider_config: ProviderConfig::Mock(MockConfig::default()),
                api_key: None,
                base_url: None,
                parameters: HashMap::default(),
            },
            ..Default::default()
        };

        let result = watch_process(pid, config);
        assert!(result.is_ok());

        let _ = child.wait_with_output();
    }
}
