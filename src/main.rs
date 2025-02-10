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
    if let Commands::Init { config, no_prompt, force } = cli.command {
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
        Commands::Watch { pid: _pid, .. } => watch_process(final_config),
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

fn watch_process(_config: Config) -> Result<()> {
    // TODO: Implement watch process command
    todo!("Watch process not yet implemented")
}