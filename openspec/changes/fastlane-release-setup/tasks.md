## 1. Ruby / Fastlane Bootstrap

- [ ] 1.1 Create `app/Gemfile` with pinned `fastlane` gem (e.g. `gem "fastlane", "~> 2.225"`)
- [ ] 1.2 Run `bundle install` inside `app/` to generate `app/Gemfile.lock` and commit both files
- [ ] 1.3 Add `app/vendor/bundle` to `.gitignore` (avoid committing installed gems)

## 2. Fastlane Configuration Files

- [ ] 2.1 Create `app/fastlane/Appfile` with `app_identifier "com.cedricziel.assistant"`, `apple_id`, and `itc_team_id` placeholders documented as env-var overrideable
- [ ] 2.2 Create `app/fastlane/Matchfile` with `git_url`, `storage_mode "git"`, `type "appstore"`, and `platform ["ios"]`
- [ ] 2.3 Create `app/fastlane/Gymfile` with: `scheme "Runner"`, `workspace "ios/Runner.xcworkspace"`, `export_method "app-store"`, `include_bitcode false`, `output_directory "build/ios/"`
- [ ] 2.4 Create `app/fastlane/Fastfile` with `platform :ios` block containing `beta` and `release` lanes
- [ ] 2.5 Implement `before_all` block in `Fastfile` to load App Store Connect API key from env vars and validate required env vars are set

## 3. iOS Lanes Implementation

- [ ] 3.1 Implement `platform :ios do lane :beta` — calls `match(type: "appstore")`, reads `BUILD_NUMBER`, calls `increment_build_number`, runs `flutter build ios --release --no-codesign`, calls `gym` for iOS, calls `pilot` (upload to TestFlight)
- [ ] 3.2 Implement `platform :ios do lane :release` — same as beta but calls `deliver(submit_for_review: true)` instead of `pilot`
- [ ] 3.3 Test `fastlane ios beta` locally with `MATCH_READONLY=true` and real credentials (dry-run with `pilot(skip_submission: true)`)

## 4. GitHub Actions Workflow

- [ ] 4.1 Create `.github/workflows/release-ios.yml` — triggers on `tags: ["v*"]`, runs on `macos-latest`, installs Flutter via `subosito/flutter-action@v2`, sets `working-directory: app`, runs `bundle install`, runs `bundle exec fastlane ios beta`
- [ ] 4.2 Add a "validate secrets" step to `release-ios.yml` that checks all required env vars and exits with a descriptive error if any are missing
- [ ] 4.3 Set `BUILD_NUMBER: ${{ github.run_number }}` as an env var in the workflow

## 5. Documentation

- [ ] 5.1 Create `app/fastlane/README.md` documenting: all required GitHub Actions secrets (`APP_STORE_CONNECT_API_KEY_ID`, `APP_STORE_CONNECT_API_KEY_ISSUER_ID`, `APP_STORE_CONNECT_API_KEY_CONTENT`, `MATCH_PASSWORD`, `MATCH_GIT_URL`), one-time setup steps (match init, certificate seeding, App Store Connect iOS app registration), and local usage (`fastlane ios beta`)
- [ ] 5.2 Note in README that macOS App Store distribution is intentionally not automated (sandbox constraints)

## 6. Verification

- [ ] 6.1 Confirm `release-ios.yml` parses correctly (`actionlint` or test push to fork)
- [ ] 6.2 Verify `flutter analyze` still passes in `app/` after all changes
- [ ] 6.3 Confirm `Gemfile.lock` is committed and matches `bundle install` on a clean machine
