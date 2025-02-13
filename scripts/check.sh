#!/usr/bin/env bash
set -e  # Exit on any error

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Track overall success
CHECKS_FAILED=0

echo -e "${BLUE}Running quality checks for Strainer...${NC}\n"

# Check if cargo-tarpaulin is installed and working
if ! cargo tarpaulin --version &> /dev/null; then
    echo -e "${BLUE}Installing or updating cargo-tarpaulin...${NC}"
    cargo install --force cargo-tarpaulin
fi

# Function to run a command and check its status
run_check() {
    local cmd="$1"
    local name="$2"
    echo -e "${BLUE}Running $name...${NC}"
    if $cmd; then
        echo -e "${GREEN}✓ $name passed${NC}\n"
        return 0
    else
        echo -e "${RED}✗ $name failed${NC}\n"
        CHECKS_FAILED=1
        return 1
    fi
}

# Run format check
if ! run_check "cargo fmt -- --check" "Format check"; then
    echo -e "${BLUE}Formatting code...${NC}"
    cargo fmt
    CHECKS_FAILED=1
fi

# Run clippy
if ! run_check "cargo clippy --all-targets --all-features -- -D warnings" "Clippy"; then
    exit 1
fi

# Build binary for integration tests
if ! run_check "cargo build --all-targets --all-features" "Build"; then
    exit 1
fi

# Run tests
if ! run_check "cargo test --all-targets --all-features" "Tests"; then
    exit 1
fi

# Run coverage only if previous checks passed
if [ $CHECKS_FAILED -eq 0 ]; then
    echo -e "${BLUE}Running code coverage...${NC}"
    if ! cargo tarpaulin --config tarpaulin.toml --out Xml --output-dir coverage; then
        echo -e "${RED}✗ Coverage check failed${NC}\n"
        exit 1
    fi
    echo -e "${GREEN}✓ Coverage check passed${NC}\n"
fi

if [ $CHECKS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}All checks completed successfully!${NC}"
    exit 0
else
    echo -e "\n${RED}Some checks failed!${NC}"
    exit 1
fi
