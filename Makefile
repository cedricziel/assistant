.PHONY: all build test lint lint-signal lint-flutter format clean check install-hooks run run-mcp run-slack run-mattermost run-matrix run-nextcloud run-signal run-webui run-worker build-signal build-macos-binary build-macos-bundle test-flutter precommit

all: build

build:
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

# ── Flutter quality checks ───────────────────────────────────────────────────

# Run dart_pre_commit (format, analyze, deps, OSV scanning) on the Flutter app.
lint-flutter:
	cd app && flutter pub run dart_pre_commit

# Run Flutter unit and widget tests.
test-flutter:
	cd app && flutter test

# Run all pre-commit checks manually (mirrors .githooks/pre-commit).
precommit:
	cargo fmt --all -- --check
	cd app && flutter pub run dart_pre_commit
	cargo clippy --workspace -- -D warnings
	cargo machete --with-metadata
	cd app && flutter test

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

# Run only the Matrix interface (no interactive REPL)
run-matrix:
	cargo run -p assistant-cli --features matrix -- orchestrator run --interfaces matrix --no-repl

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

# ── macOS app bundle ─────────────────────────────────────────────────────────

# Compile the assistant binary for Apple Silicon and copy it into the Flutter
# app's macOS bundle resources directory.
build-macos-binary:
	rustup target add aarch64-apple-darwin 2>/dev/null || true
	cargo build --release -p assistant-cli --target aarch64-apple-darwin
	mkdir -p app/macos/Runner/Resources
	cp target/aarch64-apple-darwin/release/assistant app/macos/Runner/Resources/assistant
	chmod +x app/macos/Runner/Resources/assistant

# Build the self-contained macOS .app bundle.
# Compiles the Rust binary first, then builds the Flutter macOS app.
build-macos-bundle: build-macos-binary
	cd app && flutter build macos --release

# ── OpenAPI & Flutter client generation ──────────────────────────────────────

# Export the OpenAPI spec to openapi.json (requires no running server).
dump-openapi:
	cargo run -p assistant-cli -- webui serve --print-openapi 2>/dev/null \
	  | python3 -m json.tool --no-ensure-ascii --indent 2 > openapi.json

# Validate the OpenAPI spec with Redocly CLI (requires Node.js / npx).
validate-openapi: openapi.json
	npx --yes @redocly/cli lint openapi.json

# Generate the Dart/Flutter API client from openapi.json.
# Requires: openapi-generator (brew install openapi-generator)
generate-flutter-client: openapi.json
	openapi-generator generate \
	  -i openapi.json \
	  -g dart-dio \
	  -o app/packages/assistant_api \
	  -c app/openapi-generator.yaml \
	  --skip-validate-spec
	cd app && dart format packages/assistant_api/lib packages/assistant_api/test
