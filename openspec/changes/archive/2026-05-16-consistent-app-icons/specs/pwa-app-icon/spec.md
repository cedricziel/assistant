## MODIFIED Requirements

### Requirement: Favicon uses the app icon

The system SHALL provide `app/web/favicon.png` (16×16, the default legacy fallback) AND `app/web/favicon-32.png` (32×32, the modern tab-icon size) both derived from `app/icon_source.png` (the canonical brand source). `app/web/index.html` SHALL declare both via `<link rel="icon">` tags so modern browsers select the sharper variant for tab rendering. The PWA `Icon-192.png` covers Apple touch icon and Android/desktop high-DPI uses.

#### Scenario: Favicon files exist at both resolutions

- **WHEN** the web build artifacts are inspected
- **THEN** `app/web/favicon.png` SHALL exist and be 16×16 pixels
- **AND** `app/web/favicon-32.png` SHALL exist and be 32×32 pixels

#### Scenario: index.html declares both favicon sizes

- **WHEN** `app/web/index.html` is parsed
- **THEN** it SHALL contain a `<link rel="icon">` referencing `favicon-32.png` with `sizes="32x32"`
- **AND** it SHALL also reference the original `favicon.png` for legacy fallback

#### Scenario: Tab icon is recognizable on modern browsers

- **WHEN** the web app is loaded in a modern browser (Chrome, Firefox, Safari, Edge) on any DPI
- **THEN** the browser tab SHALL display the brand icon at sufficient resolution to be recognizable as the same logo shown on the macOS dock and iOS home screen

### Requirement: PWA icons use the app icon

The system SHALL provide PWA icon files at `app/web/icons/Icon-192.png` and `app/web/icons/Icon-512.png` that are derived from `app/icon_source.png` (the canonical brand source), replacing the default Flutter template icons.

#### Scenario: PWA icon files are not the default Flutter template

- **WHEN** the web build artifacts are inspected
- **THEN** `app/web/icons/Icon-192.png` and `app/web/icons/Icon-512.png` SHALL contain the app's actual icon, not the Flutter default blue logo

#### Scenario: Standard PWA icons are reproducible from the canonical source

- **WHEN** a developer runs `make icons` against the unchanged `app/icon_source.png`
- **THEN** `app/web/icons/Icon-192.png` and `app/web/icons/Icon-512.png` SHALL be regenerated deterministically
- **AND** re-running `make icons` SHALL produce no diff in git

### Requirement: PWA manifest uses brand colors

The system SHALL set `background_color` and `theme_color` in `app/web/manifest.json` to the app's brand palette instead of Flutter's default `#0175C2`.

#### Scenario: Installed PWA splash screen shows brand color

- **WHEN** a user launches the installed PWA on a mobile device
- **THEN** the splash screen background color SHALL match the app's brand color, not the Flutter default blue

#### Scenario: Browser toolbar reflects brand theme

- **WHEN** the PWA is open in a supporting browser (e.g. Chrome on Android)
- **THEN** the browser toolbar SHALL display the app's theme color

## ADDED Requirements

### Requirement: Maskable PWA icons fill the safe zone with opaque background

The system SHALL provide maskable PWA icon files at `app/web/icons/Icon-maskable-192.png` and `app/web/icons/Icon-maskable-512.png` that are derived from a dedicated maskable source `app/icon_source_maskable.png`. The maskable variants MUST present an opaque background filling the entire square (including all four corners), with the brand logo contained within the inner 80% safe zone.

#### Scenario: Maskable icons have an opaque outer border

- **GIVEN** either `app/web/icons/Icon-maskable-192.png` or `app/web/icons/Icon-maskable-512.png`
- **WHEN** every pixel in the outer 10% border of the image is inspected
- **THEN** every such pixel SHALL have alpha at the image's maximum channel value (fully opaque)

#### Scenario: Maskable icons survive Android mask shapes

- **WHEN** an installed PWA is rendered on a launcher that applies a circular, squircle, or teardrop mask (Android)
- **THEN** the brand logo SHALL remain fully visible within the masked region
- **AND** no transparent or partially-transparent area SHALL appear at the masked edges

#### Scenario: Maskable variants are not byte-identical to the standard variants

- **WHEN** the four PWA icon files are inspected
- **THEN** `Icon-maskable-192.png` SHALL differ from `Icon-192.png`
- **AND** `Icon-maskable-512.png` SHALL differ from `Icon-512.png`

### Requirement: App icon source-of-truth is a named, discoverable file

The system SHALL maintain `app/icon_source.png` and `app/icon_source_maskable.png` as the canonical source files for all generated web/PWA icon assets. The `flutter_launcher_icons` configuration in `app/pubspec.yaml` SHALL reference `app/icon_source.png` (not a file inside a platform asset catalog).

#### Scenario: Source files exist at documented paths

- **WHEN** the repository is freshly cloned
- **THEN** `app/icon_source.png` SHALL exist as a 1024×1024 PNG containing the full-bleed brand artwork
- **AND** `app/icon_source_maskable.png` SHALL exist as a 1024×1024 PNG with an opaque background and logo within the inner safe zone

#### Scenario: pubspec generator config points at the canonical source

- **WHEN** `app/pubspec.yaml` is inspected
- **THEN** the `flutter_launcher_icons.web.image_path` field SHALL equal `icon_source.png` (relative to `app/`)
- **AND** SHALL NOT reference any path inside `macos/`, `ios/`, or `android/`

### Requirement: A single command regenerates derived web icon assets

The system SHALL provide a `make icons` target (in the repository root `Makefile`) that regenerates all derived web and PWA icon assets from the canonical source files in one invocation. The target SHALL produce no git diff when run against unchanged source files.

#### Scenario: `make icons` is idempotent against unchanged sources

- **GIVEN** a clean working tree
- **WHEN** the developer runs `make icons`
- **THEN** `git status --porcelain` SHALL report no changes to any file under `app/web/`

#### Scenario: `make icons` regenerates after a source update

- **GIVEN** `app/icon_source.png` has been replaced with new brand artwork
- **WHEN** the developer runs `make icons`
- **THEN** `app/web/favicon.png`, `app/web/favicon-32.png`, `app/web/icons/Icon-192.png`, and `app/web/icons/Icon-512.png` SHALL be updated on disk to reflect the new source
