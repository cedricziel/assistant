## Context

The assistant Flutter app targets iOS and macOS. Publishing to TestFlight and the App Store today is entirely manual: build in Xcode, manage signing certificates locally, upload via Transporter. This is fragile and not repeatable in CI.

Fastlane is the industry standard for automating Apple platform releases. It handles code signing (via `match`), building (via `gym`), and uploading (via `pilot`/`deliver`). We will add a shared `Gemfile`-driven Fastlane setup under `app/fastlane/` covering both platforms, and back it with two GitHub Actions workflows that trigger on version tags.

**Current State:**

- iOS bundle ID: `com.cedricziel.assistant.ios` (correct, registered)
- macOS bundle ID: `com.example.assistantApp` (placeholder — needs correction before App Store submission)
- No code-signing automation
- No CI release pipeline

## Goals / Non-Goals

**Goals:**

- Automate iOS build → sign → TestFlight upload with `fastlane ios beta`
- Add `fastlane ios release` lane for direct App Store submission
- Manage certificates and provisioning profiles centrally with `fastlane match` (git storage)
- Integrate into GitHub Actions on `v*` tags
- Document required secrets and one-time setup

**Non-Goals:**

- macOS App Store / TestFlight distribution — the app runs a network server, manages subprocesses, and accesses the filesystem broadly; these capabilities conflict with the App Store sandbox. macOS distribution will remain outside the App Store.
- Android / Play Store automation (out of scope)
- Web build distribution
- Automated screenshot generation (can be added later)
- Auto-incrementing semantic versions (version in `pubspec.yaml` is the source of truth)

## Decisions

### D1: iOS-only `Fastfile` under `app/fastlane/`

The `Fastfile` contains only `platform :ios` lanes. No `platform :mac` block is added. This keeps the configuration minimal and avoids carrying dead macOS lanes that can't be used until the sandbox issue is resolved.

_Alternative considered_: Stub out macOS lanes now. Rejected — dead code creates maintenance burden and false expectations.

### D2: `fastlane match` with App Store Connect API key (not legacy username/password)

Use `MATCH_APP_STORE_CONNECT_API_KEY_*` env vars (key ID, issuer ID, p8 content). Apple's legacy username/password auth is being phased out and doesn't support 2FA-free CI.

Match storage: private git repository (`MATCH_GIT_URL`). Certificates are encrypted with `MATCH_PASSWORD`.

_Alternative considered_: Manual certificate management per machine. Rejected — doesn't scale to CI.

### D3: macOS bundle ID fix is independent of Fastlane scope

The macOS bundle ID was already corrected from `com.example.assistantApp` to `com.cedricziel.assistant.macos` as a housekeeping task. It is not required for iOS Fastlane setup and is not part of the Fastlane lane or CI pipeline work.

### D4: Build number derived from CI run number

`increment_build_number` in the `Fastfile` will use `ENV["BUILD_NUMBER"]` (set from `${{ github.run_number }}` in Actions). The marketing version comes from `pubspec.yaml` (parsed and passed as `version_number`). This keeps a single source of truth for the human-readable version while ensuring a monotonically increasing build number.

### D5: `flutter build` wraps Xcode build (not direct `xcodebuild`)

Fastlane's `gym` action will call `flutter build ios --release --no-codesign` (or `macos`) first as a pre-step, then pick up the derived `.xcarchive` for signing and packaging. This ensures Flutter's build system (Dart compilation, asset bundling) runs before the Xcode archive step.

_Alternative considered_: Let `gym` invoke Xcode directly without `flutter build`. Rejected — skips Dart compilation, produces broken app.

## Risks / Trade-offs

- **Match git repo must be private and accessible from CI** → Mitigation: document required deploy key or PAT setup in `app/fastlane/README.md`.
- **Flutter version must be pinned in CI** → Mitigation: use `subosito/flutter-action@v2` with `flutter-version` from `.flutter-version` file or `pubspec.yaml` SDK constraint.
- **macOS App Store distribution deferred** → The macOS app needs significant entitlement changes (remove `app-sandbox`, adjust network/filesystem permissions) before it can target the App Store. This is a deliberate out-of-scope decision; revisit when a sandboxed macOS variant is planned.

## Migration Plan

1. Register `com.cedricziel.assistant.macos` in App Store Connect (manual, one-time).
2. Update macOS `project.pbxproj` and `AppInfo.xcconfig` with new bundle ID.
3. Create a private match git repository; run `fastlane match init` locally to populate `Matchfile`.
4. Run `fastlane match appstore --platform ios` and `fastlane match appstore --platform macos` locally once to seed certificates.
5. Add required secrets to GitHub repository settings.
6. Merge this change; first tagged release triggers CI workflows.

**Rollback**: Remove the new workflow files and revert the macOS bundle ID change. No production systems are affected.

## Open Questions

- Should `fastlane macos beta` target TestFlight for macOS (supported since Xcode 13) or direct App Store submission? → Assume TestFlight for now, matching iOS.
- Where should the match git repository live (personal GitHub org vs. separate org)? → Document as `MATCH_GIT_URL` env var; operator decides.
