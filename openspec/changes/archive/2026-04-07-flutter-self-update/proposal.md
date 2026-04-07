## Why

The Flutter desktop app has no mechanism to update itself — users must manually download and install new versions from GitHub. Providing an in-app self-update flow via GitHub Releases removes friction and ensures users stay current with bug fixes and new features.

## What Changes

- Add an update-check service that polls the GitHub Releases API for the latest version
- Display an update banner / dialog when a newer version is available
- Download the release asset for the current platform (macOS `.dmg` / `.app` zip, Linux AppImage, Windows installer)
- Verify the downloaded artifact (checksum or signature) before applying
- Launch the update installer / replace the running binary and restart the app

## Capabilities

### New Capabilities

- `app-self-update`: In-app update checking and installation via GitHub Releases — covers version polling, download, verification, and application of updates for macOS, Linux, and Windows desktop targets.

### Modified Capabilities

<!-- No existing spec-level requirements are changing -->

## Impact

- **Flutter app** (`app/`): new Riverpod providers, update-check service, UI banner/dialog in the app shell
- **Dependencies**: `pub.dev` package for GitHub Releases HTTP calls (`dio` already available); possibly `pub_semver` for version comparison; platform-specific install helpers
- **CI / Release pipeline**: GitHub Actions release workflow must publish per-platform artifacts with a consistent naming convention and a checksum manifest so the update client can verify downloads
- **No Rust backend changes** required — update logic is fully client-side
