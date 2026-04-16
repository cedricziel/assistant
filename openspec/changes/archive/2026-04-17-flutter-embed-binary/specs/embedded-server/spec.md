## ADDED Requirements

### Requirement: Detect embedded binary availability

The system SHALL detect at startup whether the `assistant` binary is present in the macOS app bundle's `Resources` directory and expose this as a runtime capability flag.

#### Scenario: Binary present in bundle

- **WHEN** the app launches on macOS and `assistant` binary exists at `<bundle>/Contents/Resources/assistant`
- **THEN** `EmbeddedServerService.isAvailable` returns `true`

#### Scenario: Binary absent from bundle

- **WHEN** the app launches on a platform where the binary is not bundled (web, or macOS without the resource)
- **THEN** `EmbeddedServerService.isAvailable` returns `false` and the connection screen defaults to remote mode

---

### Requirement: Select a free local port

The system SHALL select an available loopback port before starting the embedded server to avoid conflicts with other running processes.

#### Scenario: Port probe succeeds

- **WHEN** `EmbeddedServerService.start()` is called
- **THEN** a `ServerSocket` is bound to `127.0.0.1:0`, its assigned port is recorded, the socket is closed, and that port is used for `--listen`

> **Note:** A brief TOCTOU race exists between closing the probe socket and the assistant binary binding that port. Another process could claim the port in that window. This is an accepted limitation of the bind-probe approach on loopback; the health-check failure path handles it by emitting `EmbeddedServerError`.

---

### Requirement: Generate ephemeral auth token

The system SHALL generate a cryptographically random UUID v4 auth token per session that is never persisted or displayed to the user.

#### Scenario: Token generated on start

- **WHEN** `EmbeddedServerService.start()` is called
- **THEN** a UUID v4 string is created in memory and passed as `--auth-token` to the child process

> **Note:** Passing the token via CLI argument exposes it in process listings (e.g. `ps`, Activity Monitor) for the brief window before the server starts. This is accepted because the token is ephemeral, localhost-only, and the app targets single-user desktop use. Future hardening could pass the token via an environment variable or a 0600 temp file.

#### Scenario: Token not persisted

- **WHEN** the app exits and restarts
- **THEN** a new token is generated (previous token is discarded)

---

### Requirement: Spawn assistant child process

The system SHALL spawn the bundled `assistant` binary as a child process using `dart:io` `Process.start()`.

#### Scenario: Successful process spawn

- **WHEN** `EmbeddedServerService.start()` is called with a valid binary path, port, and token
- **THEN** the process is started with arguments `webui serve --listen 127.0.0.1:<port> --auth-token <token>` and the process handle is retained

#### Scenario: Binary not executable

- **WHEN** the binary exists but lacks execute permission
- **THEN** `EmbeddedServerService` SHALL set the execute bit via `chmod +x` before spawning, and proceed

---

### Requirement: Health-check polling before ready

The system SHALL poll `GET http://127.0.0.1:<port>/health` with exponential backoff until the server responds 200 or the maximum attempts are exhausted.

#### Scenario: Server starts within timeout

- **WHEN** the child process starts and the health endpoint returns HTTP 200 within 10 attempts (backoff starting at 200 ms, doubling each attempt)
- **THEN** `EmbeddedServerService` emits `EmbeddedServerState.ready` with the base URL and token

#### Scenario: Server fails to start

- **WHEN** 10 health-check attempts all fail or the process exits before becoming ready
- **THEN** `EmbeddedServerService` emits `EmbeddedServerState.error` with a descriptive message

---

### Requirement: Auto-connect after embedded server is ready

The system SHALL automatically create and activate a `ServerProfile` for the embedded server when it becomes ready, bypassing the manual connection setup screen.

#### Scenario: Auto-connect on ready

- **WHEN** `EmbeddedServerState.ready` is emitted
- **THEN** a `ServerProfile` with `baseUrl = "http://127.0.0.1:<port>"` and the ephemeral token is set as the active profile and the app navigates to `/chat`

---

### Requirement: Connection screen embedded mode toggle

The system SHALL offer an "Embedded (local)" option on the connection setup screen that auto-starts the embedded server without requiring URL or token input.

#### Scenario: Embedded option shown when available

- **WHEN** the user opens the connection setup screen and `EmbeddedServerService.isAvailable` is `true`
- **THEN** an "Embedded (local)" mode option is shown and selected by default

#### Scenario: Remote option still available

- **WHEN** the user selects "Remote server" on the connection setup screen
- **THEN** the URL and token fields are shown and the embedded server is not started

---

### Requirement: Graceful shutdown of embedded server

The system SHALL stop the embedded child process when the Flutter app exits or the user explicitly switches to remote mode.

#### Scenario: App exits gracefully

- **WHEN** the macOS app window closes or the app receives SIGTERM
- **THEN** `EmbeddedServerService.stop()` sends SIGTERM to the child process and waits up to 3 seconds for it to exit, then sends SIGKILL if still running

#### Scenario: User switches to remote mode

- **WHEN** the user selects "Remote server" while the embedded server is running
- **THEN** `EmbeddedServerService.stop()` is called before the remote connection attempt proceeds

---

### Requirement: Build pipeline produces self-contained macOS bundle

The build system SHALL provide a single `make build-macos-bundle` target that compiles the Rust binary for macOS, copies it into the Flutter app bundle resources, and then runs `flutter build macos --release`.

#### Scenario: Successful bundle build

- **WHEN** `make build-macos-bundle` is run on a macOS host with Rust and Flutter installed
- **THEN** the `assistant` binary (universal `lipo` merge of `aarch64` and `x86_64`) is placed at `app/macos/Runner/Resources/assistant` and the macOS `.app` is built successfully

#### Scenario: Resources directory registered in Xcode

- **WHEN** the Flutter macOS build runs
- **THEN** `app/macos/Runner/Resources/assistant` is listed as a bundle resource in `Runner.xcodeproj` so Xcode copies it into `<bundle>/Contents/Resources/`
