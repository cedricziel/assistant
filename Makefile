.PHONY: all build test lint format clean check

all: build

build:
	cargo build --workspace

build-release:
	cargo build --workspace --release

test:
	cargo test --workspace

test-integration:
	cargo test -p assistant-integration-tests --test smoke -- --ignored --nocapture

lint:
	cargo clippy --workspace --all-features -- -D warnings

format:
	cargo fmt --all

check:
	cargo check --workspace --all-features

clean:
	cargo clean

# Run the CLI interface
run:
	cargo run -p assistant-cli

# Run only the MCP server
run-mcp:
	cargo run -p mcp-server

# Build with Signal feature enabled
build-signal:
	cargo build --workspace --features signal
