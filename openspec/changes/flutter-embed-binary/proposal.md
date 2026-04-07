## Why

Users must currently run the Rust `assistant` server separately and manually configure the Flutter app's connection URL and auth token — creating a poor out-of-box experience for desktop users. Bundling the backend binary directly into the macOS app and auto-starting it eliminates this friction entirely.

## What Changes

- The Flutter macOS app bundle will include the compiled Rust `assistant` binary as a bundled resource.
- A new **embedded server mode** in Flutter will spawn the binary as a managed child process on startup.
- The app will auto-generate an ephemeral auth token, start the server on a random free port, and connect without user intervention.
- The connection/setup screen gains an "Embedded (local)" option and auto-selects it when running as a bundled macOS app.
- A `Makefile` target (`make build-macos-bundle`) will cross-compile the Rust binary for macOS and copy it into the Flutter app bundle before `flutter build macos`.

## Capabilities

### New Capabilities

- `embedded-server`: Lifecycle management for the bundled Rust backend — spawn, health-wait, auto-connect, and graceful shutdown of the child process from Flutter.

### Modified Capabilities

- none

## Impact

- **Flutter app** (`app/`): new Dart service + provider for process management; connection flow modified to detect and prefer embedded mode.
- **macOS bundle** (`app/macos/`): `Runner.xcodeproj` / `Podfile` updated to include the binary as a bundle resource; entitlements updated to allow child-process execution.
- **Build system** (`Makefile`, CI): new target to produce a self-contained `.app`; CI workflow updated to build and archive the bundle.
- **Rust binary**: no functional changes required; server must bind to a provided `--listen` address and accept `--auth-token`.
- **No API or protocol changes.**
