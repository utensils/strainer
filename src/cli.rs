use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to config file
    #[arg(long, default_value = "~/.config/strainer/config.toml")]
    pub config: PathBuf,

    /// Set log level
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Set log format
    #[arg(long, default_value = "text")]
    pub log_format: String,

    /// Increase verbosity
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a command with rate limiting
    Run {
        /// Maximum requests per minute
        #[arg(long)]
        requests_per_minute: Option<u32>,

        /// Maximum tokens per minute
        #[arg(long)]
        tokens_per_minute: Option<u32>,

        /// Maximum input tokens per minute
        #[arg(long)]
        input_tokens_per_minute: Option<u32>,

        /// Percentage at which to start warning
        #[arg(long, default_value = "30")]
        warning_threshold: u8,

        /// Percentage at which to pause process
        #[arg(long, default_value = "50")]
        critical_threshold: u8,

        /// Minimum backoff time in seconds
        #[arg(long, default_value = "5")]
        min_backoff: u32,

        /// Maximum backoff time in seconds
        #[arg(long, default_value = "60")]
        max_backoff: u32,

        /// API provider
        #[arg(long, default_value = "anthropic")]
        api: String,

        /// API key
        #[arg(long)]
        api_key: Option<String>,

        /// API base URL
        #[arg(long, default_value = "https://api.anthropic.com/v1")]
        api_base_url: String,

        /// Pause process at warning threshold
        #[arg(long)]
        pause_on_warning: bool,

        /// Pause process at critical threshold
        #[arg(long, default_value = "true")]
        pause_on_critical: bool,

        /// Resume process below this usage percentage
        #[arg(long, default_value = "25")]
        resume_threshold: u8,

        /// Command to run
        #[arg(last = true)]
        command: Vec<String>,
    },

    /// Watch an existing process
    Watch {
        /// Process ID to watch
        #[arg(long)]
        pid: u32,

        // Include all the same options as Run except for command
        /// Maximum requests per minute
        #[arg(long)]
        requests_per_minute: Option<u32>,

        /// Maximum tokens per minute
        #[arg(long)]
        tokens_per_minute: Option<u32>,

        /// Maximum input tokens per minute
        #[arg(long)]
        input_tokens_per_minute: Option<u32>,

        /// Percentage at which to start warning
        #[arg(long, default_value = "30")]
        warning_threshold: u8,

        /// Percentage at which to pause process
        #[arg(long, default_value = "50")]
        critical_threshold: u8,

        /// Minimum backoff time in seconds
        #[arg(long, default_value = "5")]
        min_backoff: u32,

        /// Maximum backoff time in seconds
        #[arg(long, default_value = "60")]
        max_backoff: u32,

        /// API provider
        #[arg(long, default_value = "anthropic")]
        api: String,

        /// API key
        #[arg(long)]
        api_key: Option<String>,

        /// API base URL
        #[arg(long, default_value = "https://api.anthropic.com/v1")]
        api_base_url: String,

        /// Pause process at warning threshold
        #[arg(long)]
        pause_on_warning: bool,

        /// Pause process at critical threshold
        #[arg(long, default_value = "true")]
        pause_on_critical: bool,

        /// Resume process below this usage percentage
        #[arg(long, default_value = "25")]
        resume_threshold: u8,
    },
}

impl Commands {
    pub fn requests_per_minute(&self) -> Option<u32> {
        match self {
            Commands::Run {
                requests_per_minute,
                ..
            } => *requests_per_minute,
            Commands::Watch {
                requests_per_minute,
                ..
            } => *requests_per_minute,
        }
    }

    pub fn tokens_per_minute(&self) -> Option<u32> {
        match self {
            Commands::Run {
                tokens_per_minute, ..
            } => *tokens_per_minute,
            Commands::Watch {
                tokens_per_minute, ..
            } => *tokens_per_minute,
        }
    }

    pub fn input_tokens_per_minute(&self) -> Option<u32> {
        match self {
            Commands::Run {
                input_tokens_per_minute,
                ..
            } => *input_tokens_per_minute,
            Commands::Watch {
                input_tokens_per_minute,
                ..
            } => *input_tokens_per_minute,
        }
    }

    pub fn warning_threshold(&self) -> u8 {
        match self {
            Commands::Run {
                warning_threshold, ..
            } => *warning_threshold,
            Commands::Watch {
                warning_threshold, ..
            } => *warning_threshold,
        }
    }

    pub fn critical_threshold(&self) -> u8 {
        match self {
            Commands::Run {
                critical_threshold, ..
            } => *critical_threshold,
            Commands::Watch {
                critical_threshold, ..
            } => *critical_threshold,
        }
    }

    pub fn resume_threshold(&self) -> u8 {
        match self {
            Commands::Run {
                resume_threshold, ..
            } => *resume_threshold,
            Commands::Watch {
                resume_threshold, ..
            } => *resume_threshold,
        }
    }

    pub fn min_backoff(&self) -> u32 {
        match self {
            Commands::Run { min_backoff, .. } => *min_backoff,
            Commands::Watch { min_backoff, .. } => *min_backoff,
        }
    }

    pub fn max_backoff(&self) -> u32 {
        match self {
            Commands::Run { max_backoff, .. } => *max_backoff,
            Commands::Watch { max_backoff, .. } => *max_backoff,
        }
    }

    pub fn api(&self) -> &str {
        match self {
            Commands::Run { api, .. } => api,
            Commands::Watch { api, .. } => api,
        }
    }

    pub fn api_key(&self) -> Option<String> {
        match self {
            Commands::Run { api_key, .. } => api_key.clone(),
            Commands::Watch { api_key, .. } => api_key.clone(),
        }
    }

    pub fn api_base_url(&self) -> &str {
        match self {
            Commands::Run { api_base_url, .. } => api_base_url,
            Commands::Watch { api_base_url, .. } => api_base_url,
        }
    }

    pub fn pause_on_warning(&self) -> bool {
        match self {
            Commands::Run {
                pause_on_warning, ..
            } => *pause_on_warning,
            Commands::Watch {
                pause_on_warning, ..
            } => *pause_on_warning,
        }
    }

    pub fn pause_on_critical(&self) -> bool {
        match self {
            Commands::Run {
                pause_on_critical, ..
            } => *pause_on_critical,
            Commands::Watch {
                pause_on_critical, ..
            } => *pause_on_critical,
        }
    }
}
