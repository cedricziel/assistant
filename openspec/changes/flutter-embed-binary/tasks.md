## 1. Build System

- [x] 1.1 Add `build-macos-binary` Makefile target: `cargo build --release -p assistant-cli --target aarch64-apple-darwin` and `--target x86_64-apple-darwin`, then `lipo` into a universal binary at `app/macos/Runner/Resources/assistant`
- [x] 1.2 Add `build-macos-bundle` Makefile target that depends on `build-macos-binary` and then runs `cd app && flutter build macos --release`
- [x] 1.3 Add `app/macos/Runner/Resources/` to `.gitignore` (binary should not be committed)
- [x] 1.4 Register `app/macos/Runner/Resources/assistant` as a bundle resource in `app/macos/Runner.xcodeproj` (add to `Copy Bundle Resources` build phase)

## 2. Dart: EmbeddedServerService

- [x] 2.1 Create `app/lib/features/embedded_server/embedded_server_service.dart` with `EmbeddedServerService` class using `dart:io` `Process`
- [x] 2.2 Implement `isAvailable` static method: check if `<bundle>/Contents/Resources/assistant` exists on macOS, return `false` on other platforms
- [x] 2.3 Implement `_findFreePort()` private helper: bind `ServerSocket` to `127.0.0.1:0`, capture port, close socket
- [x] 2.4 Implement `start()`: pick free port, generate UUID v4 token (add `uuid` package), `chmod +x` binary, `Process.start()` with `webui serve --listen 127.0.0.1:<port> --auth-token <token>` args
- [x] 2.5 Implement health-check polling loop: `GET /health` with exponential backoff (200 ms start, 10 attempts max), using `http` package or `dart:io` `HttpClient`
- [x] 2.6 Implement `stop()`: send `ProcessSignal.sigterm`, wait up to 3 s, then `ProcessSignal.sigkill` if still running
- [x] 2.7 Expose `Stream<EmbeddedServerState>` (sealed class: `starting`, `ready(baseUrl, token)`, `error(message)`, `stopped`)

## 3. Dart: Riverpod Provider

- [x] 3.1 Create `app/lib/features/embedded_server/embedded_server_provider.dart` with `EmbeddedServerNotifier extends AsyncNotifier<EmbeddedServerState>`
- [x] 3.2 In `EmbeddedServerNotifier.build()`: check `isAvailable`; if true auto-call `start()` and listen to state stream
- [x] 3.3 Register `AppLifecycleListener` (or `WidgetsBindingObserver`) to call `stop()` on app detach/terminate

## 4. Connection Screen Integration

- [x] 4.1 Add `ServerMode` enum (`embedded`, `remote`) to `app/lib/features/connection/`
- [x] 4.2 Update `connection_provider.dart`: when `embeddedServerProvider` emits `ready`, auto-create and activate `ServerProfile` with embedded URL + token, navigate to `/chat`
- [x] 4.3 Update connection setup screen UI: add mode toggle (Segmented control or radio buttons); show "Embedded (local)" option only when `EmbeddedServerService.isAvailable` is true; hide URL/token fields in embedded mode
- [x] 4.4 When user switches from embedded → remote mode: call `embeddedServerProvider.stop()` before proceeding

## 5. macOS Entitlements

- [x] 5.1 Add `com.apple.security.inherit` entitlement to `app/macos/Runner/DebugProfile.entitlements` and `Release.entitlements` to allow spawning child processes (chosen over `app-sandbox` + child-process permission; `DebugProfile.entitlements` and `Release.entitlements` both updated)
- [x] 5.2 Add `com.apple.security.network.server` entitlement to `Release.entitlements` so the sandboxed child process can bind a loopback port via `ServerSocket.bind()`

## 6. Dependencies

- [x] 6.1 Add `uuid: ^4.x` to `app/pubspec.yaml` (if not already present) for token generation

## 7. Tests

- [x] 7.1 Write unit test for `EmbeddedServerService.isAvailable` (mock `Platform` and file system)
- [x] 7.2 Write unit test for `_findFreePort()` returning a non-zero port
- [x] 7.3 Write unit test for health-check polling: mock HTTP client to succeed on 3rd attempt
- [x] 7.4 Write unit test for health-check polling: mock HTTP client to always fail, verify `error` state emitted
- [x] 7.5 Write widget test for connection screen: verify embedded toggle is shown when `isAvailable = true` and hidden when false

## 8. CI

- [x] 8.1 Update `.github/workflows/flutter.yml` to run `flutter analyze` and `flutter test` — these already run; verify no new failures from added files
- [ ] 8.2 (Optional) Add a `build-macos-bundle` CI job with `runs-on: macos-latest` to validate the full bundle build
