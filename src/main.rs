mod cli;
mod config;
mod process;
mod rate_limiter;

use anyhow::Result;
use clap::Parser;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    // Initialize logging
    let log_level = cli.log_level.parse().unwrap_or(tracing::Level::INFO);
    if cli.log_format == "json" {
        tracing_subscriber::fmt()
            .json()
            .with_max_level(log_level)
            .init();
    } else {
        tracing_subscriber::fmt().with_max_level(log_level).init();
    }

    info!("Starting strainer...");

    // Load config file
    let _config = config::Config::from_file(&cli.config)?;

    match cli.command {
        cli::Commands::Run {
            command,
            requests_per_minute,
            tokens_per_minute,
            input_tokens_per_minute,
            warning_threshold,
            critical_threshold,
            min_backoff,
            max_backoff,
            api: _,
            api_key: _,
            api_base_url: _,
            pause_on_warning: _,
            pause_on_critical: _,
            resume_threshold,
        } => {
            if command.is_empty() {
                error!("No command specified");
                std::process::exit(1);
            }

            // Create rate limiter
            let _rate_limiter = rate_limiter::RateLimiter::new(
                config::RateLimits {
                    requests_per_minute,
                    tokens_per_minute,
                    input_tokens_per_minute,
                },
                config::Thresholds {
                    warning: warning_threshold,
                    critical: critical_threshold,
                    resume: resume_threshold,
                },
                config::BackoffConfig {
                    min_seconds: min_backoff,
                    max_seconds: max_backoff,
                },
            );

            // Start the process
            let (_controller, mut child) = process::ProcessController::from_command(&command)?;

            // TODO: Implement main rate limiting loop
            let status = child.wait()?;
            if !status.success() {
                error!("Command failed with exit code: {}", status);
                std::process::exit(status.code().unwrap_or(1));
            }
        }

        cli::Commands::Watch {
            pid,
            requests_per_minute,
            tokens_per_minute,
            input_tokens_per_minute,
            warning_threshold,
            critical_threshold,
            min_backoff,
            max_backoff,
            api: _,
            api_key: _,
            api_base_url: _,
            pause_on_warning: _,
            pause_on_critical: _,
            resume_threshold,
        } => {
            // Create rate limiter
            let _rate_limiter = rate_limiter::RateLimiter::new(
                config::RateLimits {
                    requests_per_minute,
                    tokens_per_minute,
                    input_tokens_per_minute,
                },
                config::Thresholds {
                    warning: warning_threshold,
                    critical: critical_threshold,
                    resume: resume_threshold,
                },
                config::BackoffConfig {
                    min_seconds: min_backoff,
                    max_seconds: max_backoff,
                },
            );

            // SAFETY: Process IDs on Unix systems are always positive and within i32 range
            #[allow(clippy::cast_possible_wrap)]
            let controller = process::ProcessController::new(pid as i32);
            if !controller.is_running() {
                error!("Process {} not found", pid);
                std::process::exit(1);
            }

            // TODO: Implement process monitoring loop
        }
    }

    Ok(())
}
