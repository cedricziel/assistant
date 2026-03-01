.PHONY: all build test lint lint-signal format clean check install-hooks vendor

all: build

vendor:
	@crates/web-ui/vendor.sh

build: vendor
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

install-hooks:
	git config core.hooksPath .githooks

# Run the interactive REPL (Slack/Mattermost start in background if configured)
run:
	cargo run -p assistant-cli

# Run the MCP server over stdio (replaces the standalone mcp-server binary)
run-mcp:
	cargo run -p assistant-cli -- mcp

# Run only the Slack interface (no interactive REPL)
run-slack:
	cargo run -p assistant-cli --features slack -- slack

# Run only the Mattermost interface (no interactive REPL)
run-mattermost:
	cargo run -p assistant-cli --features mattermost -- mattermost

# Build the Signal interface binary with the presage integration.
# Requires presage git deps to be resolvable (see crates/interface-signal/README.md).
build-signal:
	cargo build -p assistant-interface-signal --features signal
