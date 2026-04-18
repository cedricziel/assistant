## Why

Publishing the assistant iOS app to TestFlight and the App Store today requires entirely manual steps — building in Xcode, managing signing, uploading via Transporter — which is error-prone and blocks CI-driven releases. Setting up Fastlane automates code signing, build numbering, and upload so any engineer (or CI runner) can ship a release with a single command.

The macOS app is explicitly out of scope for App Store distribution at this time: the app operates a network server, manages subprocesses, and accesses the filesystem broadly — capabilities that conflict with App Store sandbox requirements. The macOS target will be distributed outside the App Store.

## What Changes

- Add `Fastfile` + `Appfile` configuration for the iOS Fastlane lanes.
- Add `Matchfile` for centralized certificate and provisioning-profile management via `fastlane match` (App Store Connect API key auth, certificates stored in a private git repo).
- Add a `Gymfile` to define default iOS build settings.
- Add a CI workflow (`.github/workflows/release-ios.yml`) that triggers on a version tag, runs `fastlane ios beta`, and uploads to TestFlight.
- Document the one-time secrets setup required (App Store Connect API key, Match passphrase) in `app/fastlane/README.md`.

## Capabilities

### New Capabilities

- `fastlane-ios-release`: Fastlane lanes for building, signing, and uploading the iOS target to TestFlight and App Store.
- `ci-release-pipeline`: GitHub Actions workflow that invokes Fastlane on tagged commits to automate end-to-end iOS releases.

### Modified Capabilities

<!-- No existing spec-level capabilities are changing. -->

## Impact

- `app/ios/` — Fastlane directory added; Xcode project signing settings updated to use `match`-managed profiles.
- `app/pubspec.yaml` — version/build-number management integrated with Fastlane `increment_build_number`.
- `.github/workflows/` — one new release workflow file added (`release-ios.yml`).
- Secrets required in GitHub Actions: `APP_STORE_CONNECT_API_KEY_ID`, `APP_STORE_CONNECT_API_KEY_ISSUER_ID`, `APP_STORE_CONNECT_API_KEY_CONTENT`, `MATCH_PASSWORD`, `MATCH_GIT_URL`.
- No changes to Rust crates, backend code, or the macOS target.
