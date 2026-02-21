.PHONY: all build test lint lint-signal format clean check

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
	# The `signal` feature introduces presage/libsignal git deps that conflict
	# with other workspace crates at the crate.io version of curve25519-dalek.
	# Run `make lint-signal` separately after resolving those deps.
	cargo clippy --workspace -- -D warnings

lint-signal:
	cargo clippy -p assistant-interface-signal --features signal -- -D warnings

format:
	cargo fmt --all

check:
	cargo check --workspace

clean:
	cargo clean

# Run the CLI interface
run:
	cargo run -p assistant-cli

# Run only the MCP server
run-mcp:
	cargo run -p mcp-server

# Build the Signal interface binary with the presage integration.
# Requires presage git deps to be resolvable (see crates/interface-signal/README.md).
build-signal:
	cargo build -p assistant-interface-signal --features signal
