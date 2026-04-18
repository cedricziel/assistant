## ADDED Requirements

### Requirement: iOS release workflow triggers on version tags

The system SHALL provide a `release-ios-testflight` job in `.github/workflows/release-please.yml` that runs on `macos-latest` when a release is created, installs Flutter and Ruby dependencies, and invokes `fastlane ios beta`.

#### Scenario: Tagged commit triggers iOS workflow

- **WHEN** release-please creates a new release on `main`
- **THEN** the `release-ios-testflight` job starts and runs `fastlane ios beta`

#### Scenario: Job does not run without a release

- **WHEN** a commit is pushed to `main` without triggering a release
- **THEN** the `release-ios-testflight` job does NOT start

### Requirement: Required secrets documented and validated

The system SHALL document in `app/fastlane/README.md` the complete list of GitHub Actions secrets required, and the workflow SHALL fail fast (before building) if any required secret is absent.

#### Scenario: Missing secret causes early failure

- **WHEN** `APP_STORE_CONNECT_API_KEY_KEY` is not set as a GitHub secret
- **THEN** the workflow step that validates secrets exits with a non-zero code and a human-readable error message before Flutter or Fastlane runs

### Requirement: Flutter version pinned in release workflow

The system SHALL specify a Flutter version via `subosito/flutter-action@v2` in the release workflow to ensure reproducible builds.

#### Scenario: Flutter SDK installed at correct version

- **WHEN** the release workflow runs
- **THEN** `flutter --version` reports the version pinned in the workflow file

### Requirement: Gemfile locks Fastlane version

The system SHALL provide `app/Gemfile` specifying a pinned `fastlane` gem version and `app/Gemfile.lock` committed to the repository so that CI and local environments use identical Fastlane versions.

#### Scenario: Bundler uses locked Fastlane version

- **WHEN** `bundle install` runs in `app/`
- **THEN** the installed `fastlane` version matches the version in `Gemfile.lock`

### Requirement: CI workflow runs from app/ directory

The system SHALL set the working directory for all Fastlane and Flutter steps to `app/` so that relative paths in `Fastfile`, `Gymfile`, and `Matchfile` resolve correctly.

#### Scenario: Fastlane invoked from app/ directory

- **WHEN** the GitHub Actions workflow runs `bundle exec fastlane ios beta`
- **THEN** the working directory is `app/` and `app/fastlane/Fastfile` is found without additional path arguments
