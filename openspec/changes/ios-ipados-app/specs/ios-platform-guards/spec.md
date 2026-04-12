## ADDED Requirements

### Requirement: macOS-only packages do not execute on iOS

The `tray_manager` and `window_manager` packages SHALL NOT be initialised, called, or produce any effect when the app runs on iOS or iPadOS.

#### Scenario: App launches on iOS

- **WHEN** the app starts on an iOS or iPadOS device
- **THEN** no tray icon initialisation occurs
- **THEN** no window manager calls are made
- **THEN** no crash or unhandled exception is thrown from tray or window code

#### Scenario: App launches on macOS

- **WHEN** the app starts on macOS
- **THEN** tray and window manager initialise as before
- **THEN** existing macOS tray behaviour is unchanged

### Requirement: Embedded server code is excluded from the iOS build

All code paths that reference `EmbeddedServerService`, `EmbeddedServerProvider`, or `EmbeddedServerState` SHALL be unreachable and produce no side effects when running on iOS.

#### Scenario: ConnectionScreen renders on iOS

- **WHEN** `ConnectionScreen` is built on an iOS device
- **THEN** no embedded server provider is watched or initialised
- **THEN** the embedded-server status card is never rendered

#### Scenario: Flutter analyze passes for iOS target

- **WHEN** `flutter analyze` is run against the codebase
- **THEN** zero errors and zero warnings are reported
- **THEN** no dead-code or unreachable-code warnings appear for the guard blocks

### Requirement: iOS CI build passes without code signing

The command `flutter build ios --no-codesign` SHALL complete successfully in CI on every pull request that modifies `app/**`.

#### Scenario: CI runs on a PR touching app code

- **WHEN** a pull request is opened or updated with changes under `app/`
- **THEN** the CI workflow runs `flutter build ios --no-codesign`
- **THEN** the step exits with code 0
- **THEN** the build artefact is not uploaded (signing is out of scope for CI)
