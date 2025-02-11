# Strainer

[![codecov](https://codecov.io/gh/utensils/strainer/branch/main/graph/badge.svg)](https://codecov.io/gh/utensils/strainer)

A rate limiting and spend management tool for AI APIs. Strainer helps you control and manage your API usage by providing configurable rate limiting, budget controls, and usage tracking.

## Features

- Rate limiting for AI API calls with configurable thresholds
- Spend management and budget controls
- Support for multiple API providers
- Configurable limits and quotas
- Automatic backoff strategies
- Usage tracking and reporting

## Installation

```bash
cargo install strainer
```

## Usage

Basic usage involves creating a configuration file and running strainer with it:

```bash
strainer --config config.toml
```

### Configuration

Strainer uses TOML for configuration and looks for configuration files in the following locations, in order of priority:

1. CLI specified config file (using `--config` flag)
2. User config: `~/.config/strainer/config.toml`
3. System-wide config: `/etc/strainer/config.toml`

Configuration values can also be overridden by environment variables, and finally by CLI arguments which take the highest precedence.

Here's a complete example configuration file with all available options:

```toml
# API Configuration
[api]
# Provider Configuration - Choose one of the following provider sections:

# For Anthropic:
[api.provider]
type = "anthropic"
api_key = "${ANTHROPIC_API_KEY}"  # Can use environment variables
base_url = "https://api.anthropic.com/v1"  # Optional, defaults to official API
model = "claude-2"
max_tokens = 100000
temperature = 0.7

# For OpenAI (example):
# [api.provider]
# type = "openai"
# api_key = "${OPENAI_API_KEY}"
# model = "gpt-4"
# max_tokens = 100000
# temperature = 0.7

# For Mock Provider (testing):
# [api.provider]
# type = "mock"
# delay_ms = 100  # Simulated API delay

# Rate Limits
[limits]
requests_per_minute = 100     # Optional: limit requests per minute
tokens_per_minute = 100000    # Optional: limit tokens per minute
input_tokens_per_minute = 50000  # Optional: limit input tokens per minute

# Threshold Configuration
[thresholds]
warning = 30    # Percentage at which to start showing warnings (default: 30)
critical = 50   # Percentage at which to stop processing (default: 50)
resume = 25     # Percentage at which to resume after hitting critical (default: 25)

# Backoff Configuration
[backoff]
min_seconds = 5   # Minimum backoff time in seconds (default: 5)
max_seconds = 60  # Maximum backoff time in seconds (default: 60)

# Process Configuration
[process]
pause_on_warning = false  # Pause process when warning threshold is reached
pause_on_critical = true  # Pause process when critical threshold is reached (default: true)

# Logging Configuration
[logging]
level = "info"   # Log level: error, warn, info, debug, trace
format = "text"  # Log format: text or json
```

### Environment Variables

All configuration values can be set via environment variables using the `${VAR_NAME}` syntax in the TOML file. For example:

```toml
[api.provider]
type = "anthropic"
api_key = "${ANTHROPIC_API_KEY}"
```

The provider type and other configuration settings can also be set via environment variables:

```bash
STRAINER_PROVIDER_TYPE=anthropic
STRAINER_API_KEY=your-api-key
```

### Thresholds Explained

- `warning`: When usage reaches this percentage, warnings will be logged but processing continues
- `critical`: When usage reaches this percentage, processing stops and maximum backoff is applied
- `resume`: After hitting critical, processing resumes when usage drops below this percentage

### Rate Limits

Rate limits can be configured for:
- Requests per minute
- Total tokens per minute (input + output)
- Input tokens per minute

If any limit is omitted, that particular limit won't be enforced.

Example with only request limiting:
```toml
[limits]
requests_per_minute = 100
```

### Backoff Strategy

When limits are approached, Strainer implements an automatic backoff strategy:
- Below warning threshold: Uses minimum backoff time
- At warning threshold: Uses minimum backoff time with warnings
- At critical threshold: Uses maximum backoff time and pauses processing
- Below resume threshold: Resumes processing with minimum backoff

## Development

### Quality Checks

We provide a convenient script to run all quality checks:

```bash
./scripts/check.sh
```

This script will:
1. Check and fix code formatting
2. Run clippy lints with all features enabled
3. Run all tests (basic and extended)
4. Generate code coverage reports

The script requires `cargo-tarpaulin` for coverage reporting and will install it if not present.

### Testing

The project uses a two-tier testing strategy:

1. Basic Tests (No Mocking Required)
```bash
cargo test
```
This runs:
- Unit tests
- Basic integration tests
- Property tests for configuration validation

2. Extended Tests (With Mocking Support)
```bash
cargo test --features testing
```
This runs all tests, including:
- All unit tests
- Extended integration tests with mocks
- Property-based tests with mocked providers
- Test utilities for custom test scenarios

### Test Coverage Requirements

- Core rate limiting logic: 100% coverage
- Process control code: 100% coverage
- Overall minimum: 90% coverage
- All PRs must include tests
- Integration tests must cover CLI workflows

## License

This project is licensed under the MIT License - see the LICENSE file for details.