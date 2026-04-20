---
name: ios-codesigning
description: >
  iOS code signing and provisioning profile management via fastlane match.
  Use when debugging CI signing failures, adding new app extensions or targets,
  rotating certificates, or troubleshooting match decryption errors.
---

## Architecture

Code signing uses **fastlane match** in git storage mode. Certificates and provisioning
profiles are encrypted and stored in a private git repository. CI and local builds
decrypt the repo using a shared passphrase.

### Key Files

| File                     | Purpose                                                              |
| ------------------------ | -------------------------------------------------------------------- |
| `app/fastlane/Matchfile` | Match config: git URL, storage mode, app identifiers, platform       |
| `app/fastlane/Fastfile`  | Build lanes including `match`, `update_code_signing_settings`, `gym` |
| `app/fastlane/Gymfile`   | Archive settings: scheme, workspace, export method                   |
| `app/fastlane/Appfile`   | Default bundle identifier and team metadata                          |

### Secrets (all stored in 1Password and mirrored to GitHub Actions secrets)

| Purpose               | 1Password item                                      | GitHub Actions secret                 |
| --------------------- | --------------------------------------------------- | ------------------------------------- |
| Match repo URL        | _(in GitHub secret only)_                           | `MATCH_GIT_URL`                       |
| Match encryption pass | `fastlane-ci: match`                                | `MATCH_PASSWORD`                      |
| ASC API key ID        | `AppStore Connect: fastlane-ci` (username field)    | `APP_STORE_CONNECT_API_KEY_KEY_ID`    |
| ASC API issuer ID     | `AppStore Connect: fastlane-ci` (Issue ID field)    | `APP_STORE_CONNECT_API_KEY_ISSUER_ID` |
| ASC API private key   | `AppStore Connect: fastlane-ci` (attached .p8 file) | `APP_STORE_CONNECT_API_KEY_KEY`       |
| Match repo deploy key | _(SSH key)_                                         | `MATCH_DEPLOY_KEY`                    |

## Operations

### Reading credentials from 1Password

The `op` CLI is used to read secrets. Items with special characters in the title
(e.g. colons) must be referenced by **item ID**, not name, when using `op read`.

```sh
# Read match password — use item ID to avoid colon parsing issues
op read 'op://Private/<item-id>/password'

# Or use op item get (works with title but needs --reveal for concealed fields)
op item get "<item-title>" --reveal --fields password
```

**Critical caveat:** `op item get --fields password` without `--reveal` returns a
placeholder string like `[use 'op item get <id> --reveal' to reveal]`, NOT the actual
value. If this placeholder is used as `MATCH_PASSWORD` in a `match --force` run, match
will re-encrypt the entire certificates repo with the placeholder as the new passphrase.
Always use `--reveal` or `op read` with the item ID.

### Running match locally

```sh
cd app

export MATCH_GIT_URL="<certificates repo SSH URL>"
export MATCH_PASSWORD="<from 1Password>"

# Read-only fetch (installs profiles, does not modify the repo)
bundle exec fastlane match appstore --readonly

# Force re-create profiles (modifies and pushes to the certificates repo)
bundle exec fastlane match appstore --force
```

For operations that create or modify profiles, match also needs an App Store Connect
API key. Either set the `APP_STORE_CONNECT_API_KEY_*` env vars, or pass
`api_key_path:<path-to-json>` with a JSON file containing `key_id`, `issuer_id`,
`key` (base64), `is_key_content_base64`, and `in_house` (false for standard accounts).

**`in_house` is required.** If omitted from the JSON file, `sigh` (used internally by
match) will crash with `Cannot determine if team is App Store or Enterprise`. Always
set `"in_house": false` for standard Apple Developer accounts. The env var
`SPACESHIP_CONNECT_API_IN_HOUSE=true` is the alternative for Enterprise accounts.

### Adding a new app extension or target

When a new target (e.g. a share extension, widget, notification service) is added to
the Xcode project, code signing requires these steps:

1. **Register the App ID** on the Apple Developer Portal. The `produce` action does
   not support API-key-only auth; use the Spaceship API directly in a Fastfile lane:

   ```ruby
   lane :register_extension do
     require 'spaceship'
     token = Spaceship::ConnectAPI::Token.create(
       key_id:    ENV["APP_STORE_CONNECT_API_KEY_KEY_ID"],
       issuer_id: ENV["APP_STORE_CONNECT_API_KEY_ISSUER_ID"],
       key:       ENV["APP_STORE_CONNECT_API_KEY_KEY"]
     )
     Spaceship::ConnectAPI.token = token
     bid = "<bundle-identifier>"
     unless Spaceship::ConnectAPI::BundleId.find(bid)
       Spaceship::ConnectAPI::BundleId.create(name: "<display-name>", identifier: bid, platform: "IOS")
     end
   end
   ```

2. **Add the bundle identifier to `Matchfile`** in the `app_identifier` array.

3. **Run `match appstore --force`** to generate and store the provisioning profile
   in the certificates repo.

4. **Add `update_code_signing_settings`** in the Fastfile's `build_ios` lane for
   the new target, specifying the correct `profile_name` and `targets`.

5. **Verify** with `match appstore --readonly` that all profiles install cleanly.

### Troubleshooting decryption failures

`AEAD authentication tag verification failed` means the `MATCH_PASSWORD` does not
match the passphrase used to encrypt the certificates repo.

Diagnosis:

- Check the git log of the certificates repo (`git log --oneline`) to see when it
  was last modified and by whom.
- If a recent `match --force` run re-encrypted with a wrong password, the repo and
  all secrets (GitHub + 1Password) need to be re-synchronized.
- The correct password is whatever successfully decrypts the repo **right now**.
  Test with `match appstore --readonly`.

Recovery if the password is lost:

1. Delete all files in the certificates repo.
2. Run `match nuke appstore` to revoke old profiles on the Developer Portal.
3. Run `match appstore` with a known password to re-create everything.
4. Update both the GitHub secret and the 1Password item.

### CI flow

The release workflow (`.github/workflows/release-please.yml`) runs:

1. `setup_ci` — creates a temporary keychain.
2. `app_store_connect_api_key` — authenticates with ASC.
3. `match(type: "appstore")` — fetches certs/profiles (readonly in CI).
4. `update_code_signing_settings` — switches each target to manual signing with
   the match-installed profile.
5. `flutter build ios --release --no-codesign` — builds the Flutter app.
6. `gym` — archives and exports the IPA. Gym auto-detects provisioning profile
   mappings from the build settings configured in step 4.
7. `pilot` or `deliver` — uploads to TestFlight or App Store.

### Keeping secrets in sync

Whenever the match password changes (intentionally or accidentally via re-encryption):

1. Test the new password locally: `match appstore --readonly`
2. Update the GitHub secret: `gh secret set MATCH_PASSWORD`
3. Update the 1Password item to match.

All three must agree. CI uses the GitHub secret; local runs use 1Password via `op`.
