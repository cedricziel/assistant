## ADDED Requirements

### Requirement: iOS release workflow triggers on version tags

The system SHALL provide `.github/workflows/release-ios.yml` that triggers on tags matching `v*`, runs on `macos-latest`, installs Flutter and Ruby dependencies, and invokes `fastlane ios beta`.

#### Scenario: Tagged commit triggers iOS workflow

- **WHEN** a git tag starting with `v` is pushed (e.g. `v0.2.0`)
- **THEN** the `release-ios` GitHub Actions workflow starts and runs `fastlane ios beta`

#### Scenario: Workflow does not trigger on branch pushes

- **WHEN** a commit is pushed to a branch (not a tag)
- **THEN** the `release-ios` workflow does NOT start

### Requirement: Required secrets documented and validated

The system SHALL document in `app/fastlane/README.md` the complete list of GitHub Actions secrets required, and the workflow SHALL fail fast (before building) if any required secret is absent.

#### Scenario: Missing secret causes early failure

- **WHEN** `APP_STORE_CONNECT_API_KEY_CONTENT` is not set as a GitHub secret
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
