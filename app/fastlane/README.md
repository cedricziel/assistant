# Fastlane Release Automation

Automated iOS releases to TestFlight and the App Store via Fastlane.

## Required GitHub Actions Secrets

Set these in **Settings → Secrets and variables → Actions** in the GitHub repository:

| Secret                                | Description                                                          |
| ------------------------------------- | -------------------------------------------------------------------- |
| `APP_STORE_CONNECT_API_KEY_KEY_ID`    | Key ID from App Store Connect API key (e.g. `ABC123`)                |
| `APP_STORE_CONNECT_API_KEY_ISSUER_ID` | Issuer ID from App Store Connect (UUID format)                       |
| `APP_STORE_CONNECT_API_KEY_KEY`       | Contents of the `.p8` private key file (base64-encoded or raw)       |
| `APP_STORE_CONNECT_API_KEY_IS_BASE64` | `true` if the key content above is base64-encoded, `false` otherwise |
| `MATCH_GIT_URL`                       | SSH or HTTPS URL of the private git repo that stores certificates    |
| `MATCH_PASSWORD`                      | Passphrase used to encrypt/decrypt certificates in the match repo    |
| `MATCH_DEPLOY_KEY`                    | SSH private key with read access to the match certificate repo       |

### Optional App Review Secrets

`.github/workflows/app-store.yml` passes these to `ios submit`. Without them, App Store Connect keeps the review details it already has.

| Secret                                                                                                  | Description                                                                                      |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `REVIEW_CONTACT_FIRST_NAME`, `REVIEW_CONTACT_LAST_NAME`, `REVIEW_CONTACT_PHONE`, `REVIEW_CONTACT_EMAIL` | The person App Review contacts. Set all four, or none is sent.                                   |
| `REVIEW_DEMO_URL`, `REVIEW_DEMO_USER`, `REVIEW_DEMO_PASSWORD`                                           | The server and demo account App Review signs in with. Set all three, or no demo account is sent. |

Set the demo secrets whenever you set the contact. Sending the contact without a demo account clears "Sign-in required", and App Review needs a server to sign in to.

### Creating an App Store Connect API Key

1. Go to [App Store Connect → Users and Access → Integrations → App Store Connect API](https://appstoreconnect.apple.com/access/api)
2. Click **+** to create a new key with **Developer** role (or **App Manager** for full upload access)
3. Download the `.p8` file — you can only download it once
4. Note the **Key ID** and **Issuer ID**
5. Base64-encode the `.p8`: `base64 -i AuthKey_XXXXX.p8 | pbcopy`
6. Set `APP_STORE_CONNECT_API_KEY_KEY` to the base64 output and `APP_STORE_CONNECT_API_KEY_IS_BASE64=true`

## Release Flow

1. release-please cuts every release as a GitHub pre-release. `release-please.yml` still builds and ships every artifact, and uploads the iOS build to TestFlight for internal testers, with the release notes as plain text in "What to Test".
2. Promoting the pre-release to a full release in the GitHub UI starts `app-store.yml`. It runs `ios submit`, which attaches the version's existing TestFlight build (no rebuild), sets "What's New" from the release notes, and submits it for review. A version already waiting for review, in review or released is left alone. An older version still waiting for review is pulled back so the new one takes its place.
3. After App Review approves the version, publish it by hand in App Store Connect.

To submit any tag, run `app-store.yml` from the Actions tab with `tag` and `dry_run`. `dry_run` defaults to `true`: it updates the version and its notes without submitting.

GitHub's "Latest" release moves only when a release is promoted. The desktop app's updater polls `/releases/latest`, so it sees a new version only after promotion.

## One-Time Setup

### 1. Register the iOS app in App Store Connect

Ensure `com.cedricziel.assistant.ios` is registered as an App ID in the [Apple Developer portal](https://developer.apple.com/account/resources/identifiers/list) and has a corresponding app record in App Store Connect.

### 2. Initialize the match certificate repository

Create a **private** git repository to store certificates, then run locally:

```sh
cd app
export MATCH_GIT_URL=<your-private-repo-url>
export MATCH_PASSWORD=<your-passphrase>
bundle exec fastlane match init
bundle exec fastlane match appstore --platform ios
```

This creates and stores the App Store distribution certificate and provisioning profile.

### 3. Add the match repo deploy key to CI

If using SSH for `MATCH_GIT_URL`, add a deploy key with read access to the match repository and store the private key as a GitHub Actions secret (e.g. `MATCH_DEPLOY_KEY`). Update the workflow to load it with `webfactory/ssh-agent`.

## Local Usage

```sh
cd app

# Upload a build to TestFlight
bundle exec fastlane ios beta

# Send a version's TestFlight build to App Review (dry run: update notes only)
bundle exec fastlane ios submit version:X.Y.Z dry_run:true
```

Required env vars for local runs:

```sh
export APP_STORE_CONNECT_API_KEY_KEY_ID=...
export APP_STORE_CONNECT_API_KEY_ISSUER_ID=...
export APP_STORE_CONNECT_API_KEY_KEY=...     # base64 .p8 content
export APP_STORE_CONNECT_API_KEY_IS_BASE64=true
export MATCH_GIT_URL=...
export MATCH_PASSWORD=...
```

## macOS Distribution

macOS App Store distribution is intentionally **not automated** in this setup. The app requires a network server, subprocess management, and broad filesystem access — capabilities that conflict with the App Store sandbox (`com.apple.security.app-sandbox`). The macOS app is distributed outside the App Store.
