## ADDED Requirements

### Requirement: PWA icons use the app icon

The system SHALL provide PWA icon files at `app/web/icons/Icon-192.png`, `app/web/icons/Icon-512.png`, `app/web/icons/Icon-maskable-192.png`, and `app/web/icons/Icon-maskable-512.png` that are derived from the official app icon (sourced from `app/macos/Runner/Assets.xcassets/AppIcon.appiconset/app_icon_1024.png`), replacing the default Flutter template icons.

#### Scenario: PWA icon files are not the default Flutter template

- **WHEN** the web build artifacts are inspected
- **THEN** `app/web/icons/Icon-192.png` and `app/web/icons/Icon-512.png` SHALL contain the app's actual icon, not the Flutter default blue logo

#### Scenario: Maskable icons have adequate safe-zone padding

- **WHEN** the maskable icon variants are rendered on a platform that applies a mask shape (e.g. Android circle, squircle)
- **THEN** the app icon logo SHALL remain fully visible within the inner 80% safe zone of the image

### Requirement: Favicon uses the app icon

The system SHALL provide `app/web/favicon.png` derived from the official app icon, replacing the default Flutter placeholder favicon.

#### Scenario: Favicon is not the Flutter default

- **WHEN** the web app is loaded in a browser
- **THEN** the browser tab SHALL display the actual app icon, not the Flutter default blue icon

### Requirement: PWA manifest uses brand colors

The system SHALL set `background_color` and `theme_color` in `app/web/manifest.json` to the app's brand palette instead of Flutter's default `#0175C2`.

#### Scenario: Installed PWA splash screen shows brand color

- **WHEN** a user launches the installed PWA on a mobile device
- **THEN** the splash screen background color SHALL match the app's brand color, not the Flutter default blue

#### Scenario: Browser toolbar reflects brand theme

- **WHEN** the PWA is open in a supporting browser (e.g. Chrome on Android)
- **THEN** the browser toolbar SHALL display the app's theme color
