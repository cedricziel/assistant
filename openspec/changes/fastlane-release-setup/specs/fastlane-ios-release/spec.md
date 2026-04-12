## ADDED Requirements

### Requirement: Fastlane iOS beta lane uploads to TestFlight

The system SHALL provide a `fastlane ios beta` lane that builds the iOS app in release mode, signs it with a distribution certificate managed by `fastlane match`, and uploads the resulting IPA to TestFlight via the App Store Connect API.

#### Scenario: Successful beta upload

- **WHEN** `fastlane ios beta` is invoked with valid `APP_STORE_CONNECT_API_KEY_*` and `MATCH_*` environment variables set
- **THEN** an IPA is produced, signed with the App Store distribution certificate, and uploaded to TestFlight for bundle ID `com.cedricziel.assistant.ios`

#### Scenario: Missing API key halts lane

- **WHEN** `APP_STORE_CONNECT_API_KEY_CONTENT` is not set
- **THEN** the lane fails with a descriptive error before attempting to build or upload

### Requirement: Fastlane iOS release lane submits to App Store

The system SHALL provide a `fastlane ios release` lane that performs the same build and signing steps as the beta lane but submits the build to App Store review via `deliver`.

#### Scenario: Successful App Store submission

- **WHEN** `fastlane ios release` is invoked with valid credentials and the build has already been uploaded to TestFlight
- **THEN** the build is submitted for App Store review

### Requirement: Match manages iOS code signing

The system SHALL use `fastlane match appstore` to fetch or create the App Store distribution certificate and provisioning profile for `com.cedricziel.assistant.ios` and install them into the keychain before building.

#### Scenario: Certificate fetched from match repo

- **WHEN** `fastlane match appstore` runs and the certificate exists in the match git repository
- **THEN** the certificate and profile are installed into the macOS keychain without requiring Keychain Access or manual import

#### Scenario: Readonly mode in CI

- **WHEN** the `MATCH_READONLY` environment variable is `true`
- **THEN** match fetches existing certificates without attempting to create or revoke anything

### Requirement: Build number set from environment

The system SHALL read `BUILD_NUMBER` from the environment and pass it to `increment_build_number` before archiving, so each CI build has a unique, monotonically increasing build number.

#### Scenario: Build number propagated to Xcode

- **WHEN** `BUILD_NUMBER=500` is set in the environment
- **THEN** the resulting IPA has `CFBundleVersion` equal to `500`

### Requirement: Gymfile defines iOS build settings

The system SHALL provide `app/fastlane/Gymfile` (or inline `gym` parameters) specifying: scheme `Runner`, export method `app-store`, output directory `build/ios/`, and `include_bitcode false`.

#### Scenario: gym uses correct scheme

- **WHEN** `gym` runs for the iOS platform
- **THEN** it targets the `Runner` scheme in `app/ios/Runner.xcworkspace`
