## Why

The web app and PWA install both ship icons that don't match the brand the native (iOS / macOS) apps already use:

1. **`app/web/favicon.png` is only 16×16** — that's the maximum size `flutter_launcher_icons` 0.14.4 emits (`kFaviconSize = 16` is a package constant, not configurable). At 16px the spiderweb brand is just a dark smudge in browser tabs; users cannot recognize it as the same logo they see on the macOS dock or iOS home screen. (My initial inspection mistook it for the Flutter template default; verification by re-running the generator from `icon_source.png` produced a byte-identical 546-byte file — so the favicon is correctly _sourced_ from the brand, it's just rendered far below modern tab-icon resolution.)
2. **`app/web/icons/Icon-maskable-{192,512}.png` are byte-identical to the non-maskable variants** — same source, same baked-in rounded corners, same transparent padding outside the rounded square. Maskable icons MUST fill the full square so the OS (Android squircle / circle / teardrop) can apply its mask without exposing transparent corners. Today the corners get masked again, producing a tiny "double-rounded" logo on Android home screens.
3. **The pipeline is fragile**: `pubspec.yaml` runs `flutter_launcher_icons` only for `web:` and reads its source from `app/macos/Runner/Assets.xcassets/AppIcon.appiconset/app_icon_1024.png` (buried in a platform-specific asset catalog). An orphaned `app/icon_source.png` exists at the project root, byte-identical to the macOS 1024, but nothing references it. No `make icons` target exists, so the only way to regenerate is to know the secret incantation `dart run flutter_launcher_icons`.

Scope: make the web/PWA icons match the (correct) macOS/iOS app icons, fix the maskable variant to do what its `purpose` declares, and put one named source file in one obvious place so future regeneration is reproducible. Native (iOS, macOS, Android) icons are **out of scope** for this change — the user has explicitly stated those are correct as-is.

## What Changes

- Promote `app/icon_source.png` to the documented source-of-truth and update `flutter_launcher_icons` config to read from it (no longer from the macOS asset catalog).
- Add a second source `app/icon_source_maskable.png` (full-bleed background, logo at ~70% scale within the inner safe zone) for maskable PWA variants. Regenerate `Icon-maskable-192.png` and `Icon-maskable-512.png` from it.
- Keep `flutter_launcher_icons`'s 16×16 `web/favicon.png` for legacy browsers, AND emit a higher-resolution `web/favicon-32.png` (32×32) directly via ImageMagick. Add a `<link rel="icon" sizes="32x32">` tag in `app/web/index.html` so modern browsers pick the sharper variant for tab icons. The existing PWA `Icon-192.png` already covers the 192-size slot via Apple touch icon and `<link rel="icon" sizes="192x192">`.
- Regenerate `app/web/icons/Icon-192.png` and `app/web/icons/Icon-512.png` from `icon_source.png` so they are reproducible from the named source (today they happen to be derived from the macOS file but the link is invisible).
- Add a `make icons` target (Makefile target in repo root) that runs the regeneration end-to-end.
- Strengthen the `pwa-app-icon` spec: a 32×32 favicon variant MUST exist for tab rendering, maskable icons MUST have zero transparent pixels in the outer 10% border, and maskable PNGs MUST differ byte-wise from their standard counterparts.

## Capabilities

### Modified Capabilities

- `pwa-app-icon`: tighten the favicon and maskable requirements to match what was originally intended; add the source-of-truth + reproducible-generation requirements that the original spec assumed but never wrote down.

## Impact

- **Code touched**: `app/pubspec.yaml` (launcher_icons config + `image` dev dep), `app/web/favicon.png` (regenerated, still 16×16), new `app/web/favicon-32.png`, `app/web/index.html` (add 32×32 + 192×192 `<link rel="icon">` tags), `app/web/icons/Icon-{192,512,maskable-192,maskable-512}.png` (regenerated), `app/icon_source.png` (already exists, becomes canonical), new `app/icon_source_maskable.png`, root `Makefile` (new `icons` target), new `app/README.md` icons section.
- **Tests**: `app/test/assets/icons_test.dart` asserting (a) `favicon-32.png` exists and is 32×32, (b) maskable PNGs have a fully opaque outer 10% border, (c) maskable PNGs differ byte-wise from their standard counterparts. Runs as part of `flutter test`.
- **Behavior change**: end-users see the correct brand icon in browser tabs, on Android home screen after PWA install, and in the desktop PWA window. No native-app behavior changes.
- **Non-goals**:
  - Regenerating iOS / macOS / Android launcher icons (those are explicitly stated to be correct).
  - Switching to an SVG-first pipeline.
  - Designing a new icon — purely a fidelity fix to ship what the brand already is.
  - Adding `flutter_launcher_icons` config for `ios:` / `macos:` / `android:` (would touch out-of-scope assets).
- **User-facing documentation needed**: No (silent fidelity fix; users see the right icon, no behavior change to explain).
