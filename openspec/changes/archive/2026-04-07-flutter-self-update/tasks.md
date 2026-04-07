## 1. Dependencies & Configuration

- [x] 1.1 Add `pub_semver`, `crypto`, `shared_preferences`, and `path_provider` to `app/pubspec.yaml` (verify `dio` and `url_launcher` are already present)
- [x] 1.2 Add `const String kAppVersion = '<version>';` to `app/lib/core/version.dart` (or read from `pubspec.yaml` via `package_info_plus`)
- [x] 1.3 Add `package_info_plus` to `pubspec.yaml` for runtime version retrieval
- [x] 1.4 Hardcode (or make configurable) the GitHub repo slug constant (`kGitHubOwner`, `kGitHubRepo`) in `app/lib/core/constants.dart`

## 2. Update Check Service

- [x] 2.1 Create `app/lib/features/updater/update_service.dart` with `UpdateService` class that wraps the GitHub Releases API call using `dio`
- [x] 2.2 Implement `UpdateService.fetchLatestRelease()` returning `ReleaseInfo?` (version tag, release notes, assets list, html_url)
- [x] 2.3 Implement `UpdateService.isNewerVersion(String latest, String current)` using `pub_semver` `Version.parse` comparison
- [x] 2.4 Implement last-check timestamp persistence in `shared_preferences` (`updater.last_check_date`) and skip-if-same-day logic

## 3. Riverpod State

- [x] 3.1 Create `app/lib/features/updater/update_provider.dart` with `UpdateCheckNotifier extends AsyncNotifier<UpdateInfo?>` (`null` = up-to-date)
- [x] 3.2 Expose `updateCheckProvider` as a global `AsyncNotifierProvider`
- [x] 3.3 Guard check behind `kReleaseMode` — return `null` immediately in debug/profile builds

## 4. Update Notification UI

- [x] 4.1 Create `app/lib/features/updater/update_banner.dart` — a dismissible `MaterialBanner` or custom snackbar widget
- [x] 4.2 Wire the banner into the app shell (e.g., `ScaffoldMessenger` in `app_router.dart` or main scaffold) watching `updateCheckProvider`
- [x] 4.3 Show version number and one-line release notes in the banner; include "Download" and "Dismiss" actions

## 5. Artifact Download

- [x] 5.1 Create `app/lib/features/updater/artifact_downloader.dart` with `ArtifactDownloader` that selects the correct asset for `Platform.operatingSystem`
- [x] 5.2 Implement platform matching logic (`.dmg`/`-macos.zip` → macOS, `.AppImage` → Linux, `.exe`/`-windows.zip` → Windows)
- [x] 5.3 Implement fallback: open `html_url` via `url_launcher` when no platform asset is found or when `checksums.sha256` is absent
- [x] 5.4 Stream download progress via `dio` `onReceiveProgress` callback and expose via a `DownloadProgressNotifier`
- [x] 5.5 Save the downloaded file to a temp directory via `path_provider`

## 6. Checksum Verification

- [x] 6.1 Download `checksums.sha256` asset from the release and parse its `<hash>  <filename>` lines
- [x] 6.2 Compute SHA-256 of the downloaded artifact using `dart:crypto` and compare against the parsed entry
- [x] 6.3 Delete the artifact and surface an error dialog on mismatch; proceed on match

## 7. Installer Launch

- [x] 7.1 Create `app/lib/features/updater/installer_launcher.dart` with platform-specific install logic
- [x] 7.2 macOS `.dmg`: call `Process.run('open', [path])` then `exit(0)`
- [x] 7.3 macOS `.zip`: extract with `dart:io` `ZipDecoder` (or `archive` package), prompt user to confirm replace, then `exit(0)`
- [x] 7.4 Linux `.AppImage`: `File(path).chmod(0x755)` then `Process.start(path, [])` and `exit(0)`
- [x] 7.5 Windows `.exe`: `Process.run(path, [])` then `exit(0)`

## 8. Integration & Wiring

- [x] 8.1 Trigger `updateCheckProvider` initialization in `main.dart` after app startup (fire-and-forget, wrapped in try/catch)
- [x] 8.2 Connect "Download update" banner action to `ArtifactDownloader` → `ArtifactVerifier` → `InstallerLauncher` pipeline
- [x] 8.3 Show a progress dialog during download with percentage indicator and cancel button

## 9. CI Release Workflow

- [x] 9.1 Update `.github/workflows/release.yml` (or create one) to build macOS `.dmg`, Linux `.AppImage`, and Windows `.exe` artifacts
- [x] 9.2 Add a post-build step to generate `checksums.sha256` (`shasum -a 256 <artifacts>`) and upload it as a release asset
- [x] 9.3 Ensure macOS artifact is code-signed and notarized (or document as a known gap with browser-fallback behaviour)

## 10. Tests

- [x] 10.1 Unit test `UpdateService.isNewerVersion()` with equal, older, and newer version strings
- [x] 10.2 Unit test platform asset selection logic in `ArtifactDownloader` for all three platforms + no-match fallback
- [x] 10.3 Unit test checksum verification: match, mismatch, and missing-checksums-file scenarios
- [x] 10.4 Widget test `UpdateBanner` renders version string and fires correct callbacks on "Download" and "Dismiss"
- [x] 10.5 Unit test skip-if-same-day frequency logic in `UpdateService`
