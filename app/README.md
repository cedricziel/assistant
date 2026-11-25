# assistant_app

A new Flutter project.

## Getting Started

This project is a starting point for a Flutter application.

A few resources to get you started if this is your first Flutter project:

- [Learn Flutter](https://docs.flutter.dev/get-started/learn-flutter)
- [Write your first Flutter app](https://docs.flutter.dev/get-started/codelab)
- [Flutter learning resources](https://docs.flutter.dev/reference/learning-resources)

For help getting started with Flutter development, view the
[online documentation](https://docs.flutter.dev/), which offers tutorials,
samples, guidance on mobile development, and a full API reference.

## App icons

Two vector masters in this directory are the source of every app icon:

- `icon_source.svg`: full-bleed artwork, 1024×1024.
- `icon_glyph.svg`: the bare monogram in black on transparent, used for the
  macOS menu-bar template icon.

`make icons` at the repo root renders them into:

- `icon_source_maskable.png` (full bleed) and `icon_source.png` (rounded
  corners)
- the iOS AppIcon set (opaque, no alpha channel) and LaunchImage
- the macOS AppIcon set (824px rounded tile on a transparent 1024px canvas)
- `assets/tray_icon.png`, the 64px menu-bar template image, and
  `web/icons/badge-96.png`, the white notification badge
- `web/favicon.png`, `web/favicon-32.png`, and the standard and maskable PWA
  icons in `web/icons/`

Re-running it produces byte-identical files. It needs librsvg and ImageMagick
(`brew install librsvg imagemagick`, or `apt-get install librsvg2-bin
imagemagick` on Linux).

To change the logo, edit the SVGs, run `make icons`, and commit the SVGs
together with every regenerated PNG. `test/assets/icons_test.dart` checks the
outputs. See `icon_philosophy.md` for the design constraints.
