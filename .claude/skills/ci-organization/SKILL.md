---
name: ci-organization
description: >
  CI organization principles and workflow structure for this project. Use whenever modifying,
  adding, or reviewing GitHub Actions workflows — including adding jobs, changing job order,
  updating action versions, modifying cache keys, adding build targets, or debugging CI failures.
---

## Workflow Files

| File                                   | Trigger             | Purpose                                                                  |
| -------------------------------------- | ------------------- | ------------------------------------------------------------------------ |
| `.github/workflows/ci.yml`             | push/PR to `main`   | Check, test, lint, format, visual regression, integration smoke          |
| `.github/workflows/docker.yml`         | `workflow_dispatch` | Manual Docker build from source (dev/tag builds)                         |
| `.github/workflows/release-please.yml` | push to `main`      | Release automation → binary matrix → Docker → OS packages → package repo |

## CI Job Structure (`ci.yml`)

Jobs run in parallel. Order of gates (fast → slow):

1. **format** — `cargo fmt --all -- --check`. No apt deps, no cache needed. Fails fastest.
2. **check** — `cargo check --workspace --all-features`. Requires `protobuf-compiler ffmpeg`.
3. **lint** — `cargo clippy --workspace -- -D warnings` + separate `signal` feature clippy.
4. **test** — `cargo test --workspace`.
5. **visual-regression** — Playwright screenshot tests. `continue-on-error: true`.
6. **integration** — Smoke tests against live Ollama. `continue-on-error: true`, `timeout-minutes: 45`.

## Principles

### Always required before any build

```yaml
- name: Vendor JS dependencies
  run: make vendor
```

Every job that compiles Rust must run `make vendor` first (web-ui templates are embedded).

### Cargo cache key

```yaml
key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
restore-keys: ${{ runner.os }}-cargo-
```

Always hash `**/Cargo.lock`. Cross-compile jobs add the target to the key prefix:
`${{ runner.os }}-${{ matrix.target }}-cargo-`.

### System dependencies (Linux)

```sh
sudo apt-get update -y && sudo apt-get install -y protobuf-compiler ffmpeg
```

`protobuf-compiler` is required for `--all-features` (signal feature uses protobuf). `ffmpeg` is needed for transcription. macOS uses `brew install protobuf`.

### Signal feature linted separately

```yaml
- name: cargo clippy (signal feature)
  run: cargo clippy -p assistant-interface-signal --features signal -- -D warnings
```

Always include this step in the lint job — the signal feature is not covered by `--workspace`.

### Concurrency (cancel stale PR runs, never cancel main)

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}
```

### Soft failures

`visual-regression` and `integration` use `continue-on-error: true` — they are informational and depend on external services (Ollama, Playwright). Do not make them blocking.

### Upload all job artifacts

Every job that produces output files **must** upload them as artifacts using `actions/upload-artifact@v7`. This applies to:

- Test reports (HTML, JUnit, etc.)
- Built binaries
- Diff images / screenshots
- Packages (`.deb`, `.rpm`, `.apk`)

Use `if: always()` for reports so they're available even on failure:

```yaml
- name: Upload test report
  if: always()
  uses: actions/upload-artifact@v7
  with:
    name: my-report
    path: path/to/report/
    retention-days: 14
```

For binaries passed between jobs (e.g. build → docker), use `retention-days: 1` since they are transient.

### Skip on closed PRs

All non-cleanup jobs guard with:

```yaml
if: github.event.action != 'closed'
```

## Release Workflow (`release-please.yml`)

Job dependency chain:

```
release-please → build-binaries (matrix, 6 targets) → build-docker → build-packages → publish-repo
```

- **build-binaries**: matrix of 6 targets (linux gnu/musl x86_64/aarch64, macOS arm64/x86_64). Uses `cross` for cross-compilation targets.
- **build-docker**: downloads pre-built musl artifacts from `build-binaries` — does **not** rebuild from source. Multi-arch (amd64 + arm64), Alpine image via `docker/Dockerfile.assistant-alpine`.
- **build-packages**: produces `.deb`, `.rpm`, `.apk` via `nfpm`.
- **publish-repo**: orphan-commit strategy on `gh-pages` to keep APT/YUM/APK indices small.

## Action Version Pins

Always use these versions when adding new steps:

| Action                       | Version   |
| ---------------------------- | --------- |
| `actions/checkout`           | `@v6`     |
| `actions/cache`              | `@v5`     |
| `actions/upload-artifact`    | `@v7`     |
| `actions/download-artifact`  | `@v8`     |
| `actions/setup-node`         | `@v6`     |
| `actions/github-script`      | `@v8`     |
| `dtolnay/rust-toolchain`     | `@stable` |
| `docker/build-push-action`   | `@v7`     |
| `docker/login-action`        | `@v4`     |
| `docker/metadata-action`     | `@v6`     |
| `docker/setup-buildx-action` | `@v4`     |
| `docker/setup-qemu-action`   | `@v4`     |
