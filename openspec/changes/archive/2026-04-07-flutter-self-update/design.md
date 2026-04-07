## Context

The Flutter app targets macOS (primary), with potential Windows/Linux desktop support. Currently there is no in-app update mechanism; users must visit GitHub and download releases manually. The project already uses GitHub as the canonical release host (`cedricziel/assistant` or equivalent repo), so leveraging the GitHub Releases API is the natural fit.

The app is built with Flutter 3.x, Riverpod 2.x for state, and `dio` for HTTP. A new, thin update-check service will be added to the Flutter layer only — no changes to the Rust backend are required.

## Goals / Non-Goals

**Goals:**

- Periodic (and on-launch) check against GitHub Releases API for a newer version
- In-app notification (banner or dialog) when an update is available
- One-click download of the correct platform artifact from the release
- Checksum verification of the downloaded artifact before install
- Launch the installer / apply the update and restart the app
- macOS, Linux, and Windows desktop support

**Non-Goals:**

- Silent / automatic updates without user consent
- Delta / binary-diff patching (full artifact download only)
- Updating the embedded Flutter web assets served by the Rust backend
- Rolling back to a previous version

## Decisions

### D1: Use GitHub Releases API directly (no third-party update server)

**Decision**: Poll `https://api.github.com/repos/<owner>/<repo>/releases/latest` on a schedule and on app launch.

**Rationale**: Zero infrastructure cost, version history is already there, no additional hosting needed.

**Alternative considered**: A dedicated update manifest endpoint in the Rust web-ui — rejected because it adds backend complexity for no benefit when GitHub already provides a stable API.

### D2: Use `pub_updater` package (or equivalent thin wrapper) for version checking

**Decision**: Use [`pub_updater`](https://pub.dev/packages/pub_updater) or a small hand-rolled service around `dio` + `pub_semver` for version comparison.

**Rationale**: `pub_semver` gives correct SemVer ordering; `dio` is already a dep. A hand-rolled service avoids a heavy dependency for a small task.

**Alternative considered**: `upgrader` package — provides full UI but is opinionated and heavy; not a good fit for a custom design system.

### D3: Platform-specific artifact download & install strategy

| Platform | Artifact                           | Install strategy                                                           |
| -------- | ---------------------------------- | -------------------------------------------------------------------------- |
| macOS    | `.dmg` or `.zip` containing `.app` | Download → open `.dmg` / extract `.zip` → prompt user to drag or re-launch |
| Linux    | AppImage                           | Download → `chmod +x` → relaunch                                           |
| Windows  | NSIS `.exe` installer              | Download → `Process.run` the installer                                     |

**Rationale**: Each platform has a de-facto standard update artifact; matching it reduces user confusion.

### D4: Checksum verification via SHA-256

**Decision**: The CI release workflow publishes a `checksums.sha256` file alongside each release. The update client downloads it and verifies the artifact hash before launching the installer.

**Rationale**: Prevents corrupted downloads from being applied. Simple to implement with Dart's `crypto` package.

### D5: Riverpod `AsyncNotifier` for update state

**Decision**: `UpdateCheckNotifier extends AsyncNotifier<UpdateInfo?>` — `null` means up-to-date, non-null carries the release metadata.

**Rationale**: Consistent with the rest of the app's state management pattern. UI widgets watch this provider; no global singletons.

## Risks / Trade-offs

- **GitHub API rate limit** → Mitigation: check at most once per app session + once per day (persisted timestamp in shared_preferences). Unauthenticated limit is 60 req/h which is more than sufficient.
- **macOS Gatekeeper / notarization** → The downloaded artifact must be notarized by the release CI or Gatekeeper will block it. This is a CI concern, not app logic, but must be tracked.
- **Auto-update UX on macOS (running app replacement)** → Replacing a running `.app` bundle is tricky. Mitigation: use the `.dmg` flow where the user drags to Applications themselves, or use a small launch-helper script that quits the app, copies the new bundle, and relaunches.
- **Version mismatch between embedded web assets and Rust server** → Non-goal for this change; noted for future work.

## Migration Plan

1. Add `crypto`, `pub_semver`, `path_provider`, `url_launcher` (already likely present) to `pubspec.yaml`
2. Create `app/lib/features/updater/` with service, provider, and UI widgets
3. Add update-check call in `main.dart` post-init (fire-and-forget)
4. Update CI release workflow to publish per-platform artifacts + `checksums.sha256`
5. No database migrations; version state stored in `shared_preferences`
6. Rollback: remove the feature flag / provider — no persistent side effects

## Open Questions

- What is the exact GitHub repo slug to poll? (Needed to hardcode or make configurable in `config.toml`)
- Should the update check be disabled for users running the app from source / `flutter run`? (Recommend yes — check `kReleaseMode`)
- Do we need code-signing / notarization in CI for macOS before this ships? (Blocking for macOS auto-apply; non-blocking for "notify + open browser" fallback)
