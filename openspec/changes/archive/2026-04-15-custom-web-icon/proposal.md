## Why

The PWA (Progressive Web App) currently ships with Flutter's default placeholder icons instead of the app's actual icon, resulting in a generic blue Flutter logo appearing on home screens and in browser UI when users install the app. Tracked in [cedricziel/assistant#443](https://github.com/cedricziel/assistant/issues/443).

## What Changes

- Replace the default Flutter PWA icons (`Icon-192.png`, `Icon-512.png`, `Icon-maskable-192.png`, `Icon-maskable-512.png`) under `app/web/icons/` with the actual app icon derived from the macOS `app_icon_1024.png` source asset
- Replace `app/web/favicon.png` with the app's actual favicon
- Update `app/web/manifest.json` colors (`background_color`, `theme_color`) to match the app's brand palette rather than Flutter's default blue

## Capabilities

### New Capabilities

- `pwa-app-icon`: Correct PWA icon set (192×192, 512×512, maskable variants, and favicon) using the real app icon asset, ensuring the installed PWA displays the proper icon on all platforms and home screens

### Modified Capabilities

<!-- No existing spec-level capability requirements are changing -->

## Impact

- `app/web/icons/` — four PNG files replaced
- `app/web/favicon.png` — replaced
- `app/web/manifest.json` — `background_color` and `theme_color` updated to match brand
- No Rust, no API, no Flutter Dart code changes required
- Affects any user who installs the PWA or has it cached; icon update takes effect on next service-worker refresh
