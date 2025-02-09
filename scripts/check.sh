#!/bin/bash
set -e  # Exit on any error

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Running quality checks for Strainer...${NC}\n"

# Check if cargo-tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo -e "${RED}cargo-tarpaulin is not installed. Installing...${NC}"
    cargo install cargo-tarpaulin
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
        return 1
    fi
}

# Run all checks
run_check "cargo fmt -- --check" "Format check" || \
    (echo -e "${BLUE}Formatting code...${NC}" && cargo fmt)

run_check "cargo clippy --all-targets --all-features -- -D warnings" "Clippy" || exit 1

run_check "cargo test --all-targets --all-features" "Tests" || exit 1

echo -e "${BLUE}Running code coverage...${NC}"
cargo tarpaulin --all-features --out Xml --output-dir coverage

echo -e "\n${GREEN}All checks completed successfully!${NC}"
