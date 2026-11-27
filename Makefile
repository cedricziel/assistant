.PHONY: all build build-web build-release test lint lint-flutter format clean check install-hooks run run-mcp run-slack run-mattermost run-matrix run-nextcloud run-signal run-webui run-worker build-macos-binary build-macos-bundle test-flutter precommit icons coverage update-goldens

all: build

# Build the Flutter web app that `assistant-web-ui` embeds. Cargo never runs
# Flutter itself; without this step the binary embeds a placeholder page.
build-web:
	cd app && flutter pub get && flutter build web --release

build: build-web
	cargo build --workspace

build-release: build-web
	cargo build --workspace --release

test:
	cargo test --workspace

test-integration:
	cargo test -p assistant-integration-tests --test smoke -- --ignored --nocapture --test-threads=1

# Run cargo-llvm-cov over the workspace and apply the per-crate coverage
# gate from coverage.toml. Requires `cargo install cargo-llvm-cov` and
# the `llvm-tools-preview` rustup component. The gate is currently
# report-only for every crate during the rollout to the 80% floor;
# crates promote to enforcing by being removed from coverage.toml's
# [report_only].crates list. See openspec/changes/workspace-test-coverage-floor/.
coverage:
	cargo llvm-cov \
		--workspace \
		--json \
		--output-path coverage.json \
		--ignore-filename-regex '/build\.rs$$|/target/|\.pb\.rs$$|/app/packages/assistant_api/'
	./tools/check_coverage.sh

lint:
	cargo clippy --workspace -- -D warnings

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
# Also enforces theme token discipline via scripts/check_no_raw_colors.sh
# (#266) — fails when a file outside lib/shared/theme/ uses a raw Color(0x...)
# literal or Colors.X named colour without an allow-list entry.
lint-flutter:
	cd app && flutter pub run dart_pre_commit
	cd app && ./scripts/check_no_raw_colors.sh
	cd app && ./scripts/check_facade_imports.sh

# Golden baselines are generated on Linux and compared there (CI). Elsewhere
# (macOS dev machines) text renders differently, so goldens are skipped.
GOLDEN_FLAGS := $(if $(or $(CI),$(filter Linux,$(shell uname))),,--exclude-tags golden)

# Run Flutter unit and widget tests for the app and the assistant_ui package.
test-flutter:
	cd app && flutter test $(GOLDEN_FLAGS)
	cd app/packages/assistant_ui && flutter test $(GOLDEN_FLAGS)
	cd app/widgetbook && flutter test

# Regenerate golden baselines on Linux, in the Flutter image pinned to the
# version in app/pubspec.yaml, so they match what CI compares against.
FLUTTER_VERSION := $(shell sed -n 's/^  flutter: //p' app/pubspec.yaml)
update-goldens:
	docker run --rm -v "$(CURDIR)":/work -w /work/app \
	  ghcr.io/cirruslabs/flutter:$(FLUTTER_VERSION) sh -c ' \
	  set -e; \
	  trap "chown -R $(shell id -u):$(shell id -g) /work/app" EXIT; \
	  flutter pub get; \
	  (cd packages/assistant_ui && flutter test --update-goldens --tags golden); \
	  if grep -rqF "@Tags(" test; then flutter test --update-goldens --tags golden; fi'

# Run all pre-commit checks manually (mirrors .githooks/pre-commit).
precommit:
	cargo fmt --all -- --check
	cd app && flutter pub run dart_pre_commit
	cargo clippy --workspace -- -D warnings
	cargo machete --with-metadata
	$(MAKE) test-flutter

# Run the interactive REPL (Slack/Mattermost start in background if configured)
run:
	cargo run -p assistant-cli -- orchestrator run

# Run the MCP server over stdio (replaces the standalone mcp-server binary)
run-mcp:
	cargo run -p assistant-cli -- mcp

# Run only the Slack interface (no interactive REPL)
run-slack:
	cargo run -p assistant-cli -- orchestrator run --interfaces slack --no-repl

# Run only the Mattermost interface (no interactive REPL)
run-mattermost:
	cargo run -p assistant-cli -- orchestrator run --interfaces mattermost --no-repl

# Run only the Matrix interface (no interactive REPL)
run-matrix:
	cargo run -p assistant-cli -- orchestrator run --interfaces matrix --no-repl

# Run only the Nextcloud Talk interface (no interactive REPL)
run-nextcloud:
	cargo run -p assistant-cli -- orchestrator run --interfaces nextcloud --no-repl

# Run only the Signal interface (no interactive REPL)
run-signal:
	cargo run -p assistant-cli -- orchestrator run --interfaces signal --no-repl

# Run the web UI from the unified assistant binary
run-webui:
	cargo run -p assistant-cli -- webui serve --auth-token changeme --listen 127.0.0.1:8080

# Run a dedicated turn worker process
run-worker:
	cargo run -p assistant-cli -- worker --interface any --id local-worker

# ── macOS app bundle ─────────────────────────────────────────────────────────

# Compile the assistant binary for Apple Silicon and copy it into the Flutter
# app's macOS bundle resources directory.
build-macos-binary: build-web
	rustup target add aarch64-apple-darwin x86_64-apple-darwin 2>/dev/null || true
	cargo build --release -p assistant-cli --target aarch64-apple-darwin
	mkdir -p app/macos/Runner/Resources
	cp target/aarch64-apple-darwin/release/assistant app/macos/Runner/Resources/assistant
	chmod +x app/macos/Runner/Resources/assistant

# Build the self-contained macOS .app bundle.
# Compiles the Rust binary first, then builds the Flutter macOS app. Flutter's
# build hooks run cargo from the pub cache, where rustup falls back to the
# default toolchain; point them at the toolchain this repository pins.
build-macos-bundle: build-macos-binary
	cd app && RUSTUP_TOOLCHAIN=$$(rustup show active-toolchain | awk '{print $$1}') \
		flutter build macos --release

# ── OpenAPI & Flutter client generation ──────────────────────────────────────

# Export the OpenAPI spec to openapi.json (requires no running server).
dump-openapi:
	cargo run -p assistant-cli -- webui serve --print-openapi 2>/dev/null \
	  | python3 -m json.tool --no-ensure-ascii --indent 2 > openapi.json

# Validate the OpenAPI spec with Redocly CLI (requires Node.js / npx).
validate-openapi: openapi.json
	npx --yes @redocly/cli lint openapi.json

# Generate the Dart/Flutter API client from openapi.json.
# The generator runs from a pinned docker image so every machine and CI produce
# byte-identical output; a different version reformats hundreds of files.
# `dart format` is pinned to the package's SDK floor so the style does not
# depend on whether a package_config.json exists yet.
# Requires: docker
OPENAPI_GENERATOR_VERSION := 7.22.0

# The client is generated from a copy of the spec with info.version pinned,
# so a release-please version bump leaves the generated code unchanged.
generate-flutter-client: openapi.json
	mkdir -p target
	jq '.info.version = "0.0.0"' openapi.json > target/openapi-client.json
	docker run --rm --user "$$(id -u):$$(id -g)" -v "$(CURDIR):/local" \
	  openapitools/openapi-generator-cli:v$(OPENAPI_GENERATOR_VERSION) generate \
	  -i /local/target/openapi-client.json \
	  -g dart-dio \
	  -o /local/app/packages/assistant_api \
	  -c /local/app/openapi-generator.yaml \
	  --skip-validate-spec
	cd app && dart format --language-version=2.18 packages/assistant_api/lib packages/assistant_api/test
	cd app/packages/assistant_api && dart run build_runner build

# Lint the OpenAPI spec with Spectral. Enforces rules in openapi-spectral.yaml,
# notably: every secured operation must document a 401 response that uses
# the ErrorBody schema. Requires: @stoplight/spectral-cli (npm i -g).
lint-openapi: openapi.json openapi-spectral.yaml
	spectral lint openapi.json --ruleset openapi-spectral.yaml --fail-severity=error

# Regenerate every app icon from the two vector masters:
#   app/icon_source.svg  full-bleed artwork (background and "ai" monogram)
#   app/icon_glyph.svg   the bare monogram in black, for the macOS menu bar
#
# rsvg-convert renders app/icon_source.svg to app/icon_source_maskable.png
# (full bleed, the monogram sits inside the 80% safe zone). ImageMagick then
# derives app/icon_source.png (rounded corners), the iOS icons (opaque, no
# alpha channel, as the App Store requires), the macOS icons (824px rounded
# tile on a transparent 1024px canvas), the tray template image and the white
# web notification badge.
# flutter_launcher_icons writes the 16px favicon and standard PWA variants.
# `-strip` and `-define png:exclude-chunk=time,tIME` are required for
# reproducibility — without them magick embeds a timestamp in PNG metadata
# and `make icons` is no longer idempotent.
#
# Requires librsvg and ImageMagick (macOS: `brew install librsvg imagemagick`;
# Linux: `apt-get install librsvg2-bin imagemagick`). Linux fallback for
# `magick`: substitute `convert` (older ImageMagick CLI) — same flags apply.
#
# `-define png:color-type=6` forces RGBA output so the maskable variants
# always have an explicit alpha channel — without it magick strips alpha
# when the source has no transparent pixels, and downstream readers (e.g.
# the `image` Dart package's `Pixel.a` accessor) report alpha as 0 for
# alpha-less PNGs, breaking the opaque-border test.
ICON_REPRO_FLAGS := -strip -define png:exclude-chunk=time,tIME -depth 8
ICON_MAGICK_FLAGS := $(ICON_REPRO_FLAGS) -define png:color-type=6
ICON_IOS_DIR := app/ios/Runner/Assets.xcassets/AppIcon.appiconset
ICON_MACOS_DIR := app/macos/Runner/Assets.xcassets/AppIcon.appiconset
# <name>:<pixels> for every file in the iOS AppIcon set.
ICON_IOS_SIZES := 20x20@1x:20 20x20@2x:40 20x20@3x:60 29x29@1x:29 29x29@2x:58 \
	29x29@3x:87 40x40@1x:40 40x40@2x:80 40x40@3x:120 60x60@2x:120 60x60@3x:180 \
	76x76@1x:76 76x76@2x:152 83.5x83.5@2x:167 1024x1024@1x:1024
ICON_LAUNCH_DIR := app/ios/Runner/Assets.xcassets/LaunchImage.imageset
ICON_LAUNCH_SIZES := LaunchImage:200 LaunchImage@2x:400 LaunchImage@3x:600
ICON_MACOS_SIZES := 16 32 64 128 256 512
# Clips the current image to a rounded square: $(1) = side, $(2) = corner radius.
icon_rounded_mask = \( -size $(1)x$(1) xc:none -fill white -draw "roundrectangle 0,0 $(1),$(1) $(2),$(2)" \) -compose DstIn -composite -compose Over
icons:
	rsvg-convert -w 1024 -h 1024 app/icon_source.svg | magick - $(ICON_MAGICK_FLAGS) app/icon_source_maskable.png
	magick app/icon_source_maskable.png $(call icon_rounded_mask,1024,229) $(ICON_MAGICK_FLAGS) app/icon_source.png
	for e in $(ICON_IOS_SIZES); do n=$${e%:*}; px=$${e#*:}; \
		magick app/icon_source_maskable.png -alpha off -resize $${px}x$${px} $(ICON_REPRO_FLAGS) "$(ICON_IOS_DIR)/Icon-App-$$n.png" || exit 1; done
	for e in $(ICON_LAUNCH_SIZES); do n=$${e%:*}; px=$${e#*:}; \
		magick app/icon_source.png -resize $${px}x$${px} $(ICON_MAGICK_FLAGS) "$(ICON_LAUNCH_DIR)/$$n.png" || exit 1; done
	magick app/icon_source_maskable.png -resize 824x824 $(call icon_rounded_mask,824,185) \
		-background none -gravity center -extent 1024x1024 $(ICON_MAGICK_FLAGS) $(ICON_MACOS_DIR)/app_icon_1024.png
	for px in $(ICON_MACOS_SIZES); do \
		magick $(ICON_MACOS_DIR)/app_icon_1024.png -resize $${px}x$${px} $(ICON_MAGICK_FLAGS) $(ICON_MACOS_DIR)/app_icon_$$px.png || exit 1; done
	rsvg-convert -w 64 -h 64 app/icon_glyph.svg | magick - $(ICON_MAGICK_FLAGS) app/assets/tray_icon.png
	rsvg-convert -w 96 -h 96 app/icon_glyph.svg | magick - -fill white -colorize 100 $(ICON_MAGICK_FLAGS) app/web/icons/badge-96.png
	cd app && dart run flutter_launcher_icons
	magick app/icon_source.png $(ICON_MAGICK_FLAGS) -resize 32x32 app/web/favicon-32.png
	magick app/icon_source_maskable.png $(ICON_MAGICK_FLAGS) -resize 192x192 app/web/icons/Icon-maskable-192.png
	magick app/icon_source_maskable.png $(ICON_MAGICK_FLAGS) -resize 512x512 app/web/icons/Icon-maskable-512.png
