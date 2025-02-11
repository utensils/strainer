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
    /// Initialize a new configuration
    Init {
        /// Path to create the config file
        #[arg(long)]
        config: Option<PathBuf>,

        /// Don't prompt for input, use defaults
        #[arg(long)]
        no_prompt: bool,

        /// Force overwrite if config file exists
        #[arg(long)]
        force: bool,
    },

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
    pub const fn requests_per_minute(&self) -> Option<u32> {
        match self {
            Self::Run {
                requests_per_minute,
                ..
            }
            | Self::Watch {
                requests_per_minute,
                ..
            } => *requests_per_minute,
            Self::Init { .. } => None,
        }
    }

    pub const fn tokens_per_minute(&self) -> Option<u32> {
        match self {
            Self::Run {
                tokens_per_minute, ..
            }
            | Self::Watch {
                tokens_per_minute, ..
            } => *tokens_per_minute,
            Self::Init { .. } => None,
        }
    }

    pub const fn input_tokens_per_minute(&self) -> Option<u32> {
        match self {
            Self::Run {
                input_tokens_per_minute,
                ..
            }
            | Self::Watch {
                input_tokens_per_minute,
                ..
            } => *input_tokens_per_minute,
            Self::Init { .. } => None,
        }
    }

    pub const fn warning_threshold(&self) -> u8 {
        match self {
            Self::Run {
                warning_threshold, ..
            }
            | Self::Watch {
                warning_threshold, ..
            } => *warning_threshold,
            Self::Init { .. } => 30, // Default value
        }
    }

    pub const fn critical_threshold(&self) -> u8 {
        match self {
            Self::Run {
                critical_threshold, ..
            }
            | Self::Watch {
                critical_threshold, ..
            } => *critical_threshold,
            Self::Init { .. } => 50, // Default value
        }
    }

    pub const fn resume_threshold(&self) -> u8 {
        match self {
            Self::Run {
                resume_threshold, ..
            }
            | Self::Watch {
                resume_threshold, ..
            } => *resume_threshold,
            Self::Init { .. } => 25, // Default value
        }
    }

    pub const fn min_backoff(&self) -> u32 {
        match self {
            Self::Run { min_backoff, .. } | Self::Watch { min_backoff, .. } => *min_backoff,
            Self::Init { .. } => 5, // Default value
        }
    }

    pub const fn max_backoff(&self) -> u32 {
        match self {
            Self::Run { max_backoff, .. } | Self::Watch { max_backoff, .. } => *max_backoff,
            Self::Init { .. } => 60, // Default value
        }
    }

    pub fn api(&self) -> &str {
        match self {
            Self::Run { api, .. } | Self::Watch { api, .. } => api,
            Self::Init { .. } => "anthropic", // Default value
        }
    }

    pub fn api_key(&self) -> Option<String> {
        match self {
            Self::Run { api_key, .. } | Self::Watch { api_key, .. } => api_key.clone(),
            Self::Init { .. } => None,
        }
    }

    pub fn api_base_url(&self) -> &str {
        match self {
            Self::Run { api_base_url, .. } | Self::Watch { api_base_url, .. } => api_base_url,
            Self::Init { .. } => "https://api.anthropic.com/v1", // Default value
        }
    }

    pub const fn pause_on_warning(&self) -> bool {
        match self {
            Self::Run {
                pause_on_warning, ..
            }
            | Self::Watch {
                pause_on_warning, ..
            } => *pause_on_warning,
            Self::Init { .. } => false, // Default value
        }
    }

    pub const fn pause_on_critical(&self) -> bool {
        match self {
            Self::Run {
                pause_on_critical, ..
            }
            | Self::Watch {
                pause_on_critical, ..
            } => *pause_on_critical,
            Self::Init { .. } => true, // Default value
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_defaults() {
        let cli = Cli::try_parse_from(["strainer"]).unwrap_err();
        assert!(cli.to_string().contains("Usage: strainer"));
    }

    #[test]
    fn test_cli_init_command() {
        let cli = Cli::try_parse_from(["strainer", "init"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Init {
                config: None,
                no_prompt: false,
                force: false
            }
        ));
    }

    #[test]
    fn test_cli_init_with_options() {
        let cli = Cli::try_parse_from([
            "strainer",
            "init",
            "--config",
            "test.toml",
            "--no-prompt",
            "--force",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Commands::Init {
                config: Some(_),
                no_prompt: true,
                force: true
            }
        ));
    }

    #[test]
    fn test_cli_run_command() {
        let cli = Cli::try_parse_from(["strainer", "run", "--", "echo", "test"]).unwrap();
        if let Commands::Run { command, .. } = cli.command {
            assert_eq!(command, vec!["echo", "test"]);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_cli_run_with_options() {
        let cli = Cli::try_parse_from([
            "strainer",
            "run",
            "--requests-per-minute",
            "100",
            "--tokens-per-minute",
            "1000",
            "--input-tokens-per-minute",
            "500",
            "--warning-threshold",
            "40",
            "--critical-threshold",
            "80",
            "--min-backoff",
            "10",
            "--max-backoff",
            "120",
            "--api",
            "test-provider",
            "--api-key",
            "test-key",
            "--api-base-url",
            "http://test.local",
            "--pause-on-warning",
            "--",
            "echo",
            "test",
        ])
        .unwrap();

        if let Commands::Run {
            requests_per_minute,
            tokens_per_minute,
            input_tokens_per_minute,
            warning_threshold,
            critical_threshold,
            min_backoff,
            max_backoff,
            api,
            api_key,
            api_base_url,
            pause_on_warning,
            command,
            ..
        } = cli.command
        {
            assert_eq!(requests_per_minute, Some(100));
            assert_eq!(tokens_per_minute, Some(1000));
            assert_eq!(input_tokens_per_minute, Some(500));
            assert_eq!(warning_threshold, 40);
            assert_eq!(critical_threshold, 80);
            assert_eq!(min_backoff, 10);
            assert_eq!(max_backoff, 120);
            assert_eq!(api, "test-provider");
            assert_eq!(api_key, Some("test-key".to_string()));
            assert_eq!(api_base_url, "http://test.local");
            assert!(pause_on_warning);
            assert_eq!(command, vec!["echo", "test"]);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_cli_watch_command() {
        let cli = Cli::try_parse_from(["strainer", "watch", "--pid", "1234"]).unwrap();
        if let Commands::Watch { pid, .. } = cli.command {
            assert_eq!(pid, 1234);
        } else {
            panic!("Expected Watch command");
        }
    }

    #[test]
    fn test_commands_accessors() {
        test_run_command_accessors();
        test_init_command_accessors();
    }

    #[test]
    fn test_run_command_accessors() {
        let run_cmd = Commands::Run {
            requests_per_minute: Some(100),
            tokens_per_minute: Some(1000),
            input_tokens_per_minute: Some(500),
            warning_threshold: 40,
            critical_threshold: 80,
            min_backoff: 10,
            max_backoff: 120,
            api: "test-provider".to_string(),
            api_key: Some("test-key".to_string()),
            api_base_url: "http://test.local".to_string(),
            pause_on_warning: true,
            pause_on_critical: true,
            resume_threshold: 20,
            command: vec!["test".to_string()],
        };

        assert_eq!(run_cmd.requests_per_minute(), Some(100));
        assert_eq!(run_cmd.tokens_per_minute(), Some(1000));
        assert_eq!(run_cmd.input_tokens_per_minute(), Some(500));
        assert_eq!(run_cmd.warning_threshold(), 40);
        assert_eq!(run_cmd.critical_threshold(), 80);
        assert_eq!(run_cmd.min_backoff(), 10);
        assert_eq!(run_cmd.max_backoff(), 120);
        assert_eq!(run_cmd.api(), "test-provider");
        assert_eq!(run_cmd.api_key(), Some("test-key".to_string()));
        assert_eq!(run_cmd.api_base_url(), "http://test.local");
        assert!(run_cmd.pause_on_warning());
        assert!(run_cmd.pause_on_critical());
        assert_eq!(run_cmd.resume_threshold(), 20);
    }

    #[test]
    fn test_init_command_accessors() {
        let init_cmd = Commands::Init {
            config: None,
            no_prompt: false,
            force: false,
        };

        assert_eq!(init_cmd.requests_per_minute(), None);
        assert_eq!(init_cmd.tokens_per_minute(), None);
        assert_eq!(init_cmd.input_tokens_per_minute(), None);
        assert_eq!(init_cmd.warning_threshold(), 30);
        assert_eq!(init_cmd.critical_threshold(), 50);
        assert_eq!(init_cmd.min_backoff(), 5);
        assert_eq!(init_cmd.max_backoff(), 60);
        assert_eq!(init_cmd.api(), "anthropic");
        assert_eq!(init_cmd.api_key(), None);
        assert_eq!(init_cmd.api_base_url(), "https://api.anthropic.com/v1");
        assert!(!init_cmd.pause_on_warning());
        assert!(init_cmd.pause_on_critical());
        assert_eq!(init_cmd.resume_threshold(), 25);
    }
}
