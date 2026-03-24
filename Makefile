.PHONY: all build test lint lint-signal format clean check install-hooks vendor run run-mcp run-slack run-mattermost run-nextcloud run-signal run-webui run-worker build-signal

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
	cargo test -p assistant-integration-tests --test smoke -- --ignored --nocapture --test-threads=1

lint:
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
	cargo run -p assistant-cli -- orchestrator run

# Run the MCP server over stdio (replaces the standalone mcp-server binary)
run-mcp:
	cargo run -p assistant-cli -- mcp

# Run only the Slack interface (no interactive REPL)
run-slack:
	cargo run -p assistant-cli --features slack -- orchestrator run --interfaces slack --no-repl

# Run only the Mattermost interface (no interactive REPL)
run-mattermost:
	cargo run -p assistant-cli --features mattermost -- orchestrator run --interfaces mattermost --no-repl

# Run only the Nextcloud Talk interface (no interactive REPL)
run-nextcloud:
	cargo run -p assistant-cli --features nextcloud -- orchestrator run --interfaces nextcloud --no-repl

# Run only the Signal interface (no interactive REPL)
run-signal:
	cargo run -p assistant-cli --features signal -- orchestrator run --interfaces signal --no-repl

# Run the web UI from the unified assistant binary
run-webui:
	cargo run -p assistant-cli -- webui serve --auth-token changeme --listen 127.0.0.1:8080

# Run a dedicated turn worker process
run-worker:
	cargo run -p assistant-cli -- worker --interface any --id local-worker

# Build the Signal interface binary with the presage integration.
# Requires presage git deps to be resolvable (see crates/interface-signal/README.md).
build-signal:
	cargo build -p assistant-interface-signal --features signal
