## Context

The Flutter app communicates with the assistant backend via HTTP (Dio) and SSE streaming. Currently, network failures are detected reactively — only when a request fails with a `DioException`. The app has reconnect logic in `ChatProvider.attemptReconnect()` but it's only triggered on `AppLifecycleState.resumed`. On macOS desktop, where the app stays in the foreground during Wi-Fi drops (lid close, network switch), this trigger never fires.

The app already uses Riverpod for state management, has a `nav_shell.dart` wrapping all routes, and the chat provider tracks `_needsReconnect` state with a `_currentRunId` for SSE event-log replay.

## Goals / Non-Goals

**Goals:**

- Provide immediate visual feedback when network is unavailable
- Automatically trigger SSE reconnection when connectivity is restored
- Prevent user from initiating actions that will fail (voice recording, file upload) while offline
- Work across all target platforms (macOS, iOS, web)

**Non-Goals:**

- Server reachability probing (captive portals, VPN tunnels)
- Offline message queuing or draft persistence
- Custom web implementation (browsers provide `navigator.onLine` natively, which `connectivity_plus` wraps)

## Decisions

### 1. Use `connectivity_plus` over raw platform channels

**Choice**: `connectivity_plus` package (Flutter Favorites, maintained by the plus_plugins team)

**Alternatives considered**:

- Raw `NWPathMonitor` (macOS/iOS) + platform channel: More control, but requires native code in both platforms and ongoing maintenance. Not worth it for transport-level detection.
- `internet_connection_checker`: Does active HTTP probing (pings Google DNS). Heavier, adds latency, requires an external endpoint. Overkill — our DioException path already handles server-unreachable cases.

**Rationale**: `connectivity_plus` is lightweight, well-maintained, covers all our platforms, and only reports transport availability — which is exactly what we need as a fast signal.

### 2. Riverpod StreamProvider for connectivity state

**Choice**: A `connectivityProvider` (StreamProvider) wrapping `Connectivity().onConnectivityChanged`, with a derived `isOnlineProvider` (Provider<bool>).

**Rationale**: Fits the existing Riverpod architecture. StreamProvider auto-disposes when nothing watches it. The derived boolean makes consumption trivial for guards.

### 3. Offline banner in NavShell (not per-screen)

**Choice**: A single `MaterialBanner` or slim bar at the top of `NavShell`, visible across all routes.

**Alternatives considered**:

- Per-screen banners: Duplicated logic, inconsistent placement.
- SnackBar: Dismissible, can be missed, doesn't persist.

**Rationale**: Consistent, always visible, single integration point. Disappears automatically when connectivity returns.

### 4. Reconnect trigger: connectivity restoration + existing app-resume

**Choice**: Watch `connectivityProvider` transitions from `none` → any connected state inside `AgentEventListener` (which already watches lifecycle) and call `chatProvider.attemptReconnect()`.

**Rationale**: Reuses existing reconnect machinery. The `attemptReconnect()` method already handles the run-ID-based event replay. Adding a second trigger (connectivity change) alongside the existing lifecycle trigger covers both mobile (backgrounding) and desktop (network drop without backgrounding) scenarios.

### 5. Guard pattern for voice/upload

**Choice**: Read `isOnlineProvider` before starting voice recording or file upload. Show a brief toast/snackbar ("No internet connection") and abort. Don't disable the button permanently — just guard the action.

**Rationale**: Disabling buttons confuses users who don't notice the offline banner. A guard with feedback is clearer and self-resolving.

## Risks / Trade-offs

- **[False positives on web]** `connectivity_plus` on web uses `navigator.onLine` which can report "online" behind captive portals. → Mitigation: This is a known limitation; our DioException path is the fallback. The banner is additive UX, not the only failure path.

- **[Platform inconsistency]** iOS may report `ConnectivityResult.wifi` while actually on a captive portal with no internet. → Mitigation: Same as above — we treat this as transport awareness, not server reachability.

- **[Stream disposal race]** If `connectivityProvider` triggers reconnect while `ChatProvider` is mid-dispose (route change), we could get a use-after-dispose. → Mitigation: Guard `attemptReconnect()` with a mounted/active check (already partially in place via `_needsReconnect` flag).

- **[Battery on mobile]** `connectivity_plus` uses OS-level callbacks, not polling — negligible battery impact.
