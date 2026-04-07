## ADDED Requirements

### Requirement: Version check on launch

The app SHALL check for a newer release via the GitHub Releases API when it starts in release mode (`kReleaseMode == true`). The check SHALL be performed asynchronously without blocking the UI.

#### Scenario: Newer version available

- **WHEN** the app launches and the GitHub API returns a release tag greater than the current app version
- **THEN** the app displays an update-available notification (banner or dialog) with the new version number and release notes

#### Scenario: Already up-to-date

- **WHEN** the app launches and the latest release tag equals the current app version
- **THEN** no notification is shown and the app proceeds normally

#### Scenario: API unreachable

- **WHEN** the GitHub Releases API cannot be reached (network error or timeout)
- **THEN** the update check fails silently and no error is shown to the user

#### Scenario: Running in debug/profile mode

- **WHEN** the app is running with `kReleaseMode == false`
- **THEN** the update check is skipped entirely

### Requirement: Respect check frequency limit

The app SHALL not perform more than one GitHub API check per calendar day per device to avoid rate-limit exhaustion.

#### Scenario: Check already performed today

- **WHEN** the app launches and the stored last-check timestamp is from the current calendar day
- **THEN** the update check is skipped for this session

#### Scenario: First launch of the day

- **WHEN** the app launches and no stored timestamp exists or the stored timestamp is from a previous calendar day
- **THEN** the update check proceeds and the timestamp is updated on completion

### Requirement: Update available notification

When a newer version is detected the app SHALL surface a non-blocking in-app notification that allows the user to act or dismiss.

#### Scenario: User initiates download

- **WHEN** user taps / clicks the "Download update" action in the notification
- **THEN** the app begins downloading the platform-correct release artifact and shows download progress

#### Scenario: User dismisses notification

- **WHEN** user dismisses the update notification
- **THEN** the notification is hidden for the remainder of the session and no download begins

### Requirement: Platform-correct artifact selection

The update client SHALL download the release asset that matches the current platform.

#### Scenario: macOS artifact

- **WHEN** the platform is macOS and a download is initiated
- **THEN** the artifact whose filename ends in `.dmg` or `-macos.zip` is downloaded

#### Scenario: Linux artifact

- **WHEN** the platform is Linux and a download is initiated
- **THEN** the artifact whose filename ends in `.AppImage` is downloaded

#### Scenario: Windows artifact

- **WHEN** the platform is Windows and a download is initiated
- **THEN** the artifact whose filename ends in `.exe` or `-windows.zip` is downloaded

#### Scenario: No matching artifact

- **WHEN** no release asset matches the current platform
- **THEN** the app opens the GitHub release page in the default browser as a fallback

### Requirement: Checksum verification before install

The app SHALL verify the SHA-256 checksum of the downloaded artifact against the `checksums.sha256` file published in the same release before launching the installer.

#### Scenario: Checksum matches

- **WHEN** the SHA-256 hash of the downloaded file matches the corresponding entry in `checksums.sha256`
- **THEN** the app proceeds to launch the installer

#### Scenario: Checksum mismatch

- **WHEN** the SHA-256 hash of the downloaded file does not match
- **THEN** the downloaded file is deleted, an error is shown to the user, and the installer is NOT launched

#### Scenario: Checksums file missing from release

- **WHEN** the release does not include a `checksums.sha256` asset
- **THEN** the app falls back to opening the release page in the browser and does not attempt an in-app install

### Requirement: Installer launch and app restart

After successful checksum verification the app SHALL launch the platform installer and exit.

#### Scenario: macOS DMG install

- **WHEN** the downloaded artifact is a `.dmg` and checksum passes
- **THEN** the app opens the `.dmg` with the default handler (`open`) and quits

#### Scenario: macOS ZIP install

- **WHEN** the downloaded artifact is a `.zip` and checksum passes
- **THEN** the app extracts the `.app` bundle, prompts the user to replace the current app, and quits

#### Scenario: Linux AppImage install

- **WHEN** the downloaded artifact is an `.AppImage` and checksum passes
- **THEN** the file is made executable (`chmod +x`) and launched; the old process exits

#### Scenario: Windows installer

- **WHEN** the downloaded artifact is a `.exe` installer and checksum passes
- **THEN** the installer is launched via `Process.run` and the app exits
