# Strainer

A rate limiting and spend management tool for AI APIs.

## Features

- Rate limiting for AI API calls
- Spend management and budget controls
- Support for multiple API providers
- Configurable limits and quotas

## Installation

```bash
cargo install strainer
```

## Usage

```bash
strainer --config config.toml
```

## Configuration

Configuration is done via TOML files. Example configuration:

```toml
# Coming soon
```

## Development

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