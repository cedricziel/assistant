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

### Creating an App Store Connect API Key

1. Go to [App Store Connect → Users and Access → Integrations → App Store Connect API](https://appstoreconnect.apple.com/access/api)
2. Click **+** to create a new key with **Developer** role (or **App Manager** for full upload access)
3. Download the `.p8` file — you can only download it once
4. Note the **Key ID** and **Issuer ID**
5. Base64-encode the `.p8`: `base64 -i AuthKey_XXXXX.p8 | pbcopy`
6. Set `APP_STORE_CONNECT_API_KEY_KEY` to the base64 output and `APP_STORE_CONNECT_API_KEY_IS_BASE64=true`

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

# Submit a build to App Store review
bundle exec fastlane ios release
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

## Updating Provisioning Profiles

When a new capability is added to an app target's entitlements (e.g. App Groups,
Push Notifications, iCloud), the provisioning profile stored in the match repository
must be regenerated to include that capability. Without this, CI will fail with
`Provisioning profile doesn't include the <capability> capability`.

### Steps

1. **Enable the capability** on the App ID in the
   [Apple Developer Portal](https://developer.apple.com/account/resources/identifiers/list).
   For example, for the ShareExtension:
   - Open `com.cedricziel.assistant.ios.ShareExtension`
   - Enable **App Groups** and add `group.com.cedricziel.assistant`

2. **Export credentials** locally:

   ```sh
   export MATCH_GIT_URL=...
   export MATCH_PASSWORD=...
   export APP_STORE_CONNECT_API_KEY_KEY_ID=...
   export APP_STORE_CONNECT_API_KEY_ISSUER_ID=...
   export APP_STORE_CONNECT_API_KEY_KEY=...   # base64-encoded .p8 content
   export APP_STORE_CONNECT_API_KEY_IS_BASE64=true
   ```

3. **Run the refresh lane** from the `app/` directory:

   ```sh
   cd app
   bundle exec fastlane ios refresh_profiles
   ```

   This force-regenerates all App Store provisioning profiles for every bundle ID
   listed in the `Matchfile` and pushes the updated profiles to the match
   certificate repository. CI will pick up the new profiles on the next run.

## macOS Distribution

macOS App Store distribution is intentionally **not automated** in this setup. The app requires a network server, subprocess management, and broad filesystem access — capabilities that conflict with the App Store sandbox (`com.apple.security.app-sandbox`). The macOS app is distributed outside the App Store.
