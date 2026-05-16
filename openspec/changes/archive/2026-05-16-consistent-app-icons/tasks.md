## 1. Failing tests first (TDD red)

- [x] 1.1 Add `app/test/assets/icons_test.dart` with test cases (initial draft used a forbidden-hash check; revised in §1.3 below after investigation showed the favicon was already brand-correct, just rendered at 16×16):
  - `maskable 192 has opaque outer border`: load `web/icons/Icon-maskable-192.png` with `package:image`, scan the outer 10% (≈19px on each side), assert every pixel has `a == 255`.
  - `maskable 512 has opaque outer border`: same assertion at 512px (≈51px border).
  - `maskable variants differ from standards`: SHA-256 of `Icon-maskable-{192,512}.png` ≠ SHA-256 of `Icon-{192,512}.png`.
- [x] 1.2 Run `cd app && flutter test test/assets/icons_test.dart` — confirm maskable tests FAIL with the current assets (transparent corners; byte-identical to standards).
- [x] 1.3 Revise the test to drop the false-premise `kFlutterDefaultFaviconSha256` check (the existing favicon turned out to be a correctly-sourced 16×16 brand export, not the Flutter default). Replace with: `favicon-32.png exists` AND `favicon-32.png is 32×32` AND `web/index.html declares sizes="32x32"`. Also harden the maskable opaque-border check to compare against `decoded.maxChannelValue` instead of hard-coded 255, so it survives 8-bit-vs-16-bit PNG depth variation.

## 2. Establish the source-of-truth files

- [x] 2.1 Verify `app/icon_source.png` is the correct full-bleed 1024×1024 brand artwork (it already exists; this is a no-op if its contents are right).
- [x] 2.2 Hand-prepare `app/icon_source_maskable.png`: 1024×1024, background filled to all four edges with `#080C1A`, brand logo scaled to ~70% (≈716px wide) and centered. Commit as a new file. Acceptable tools: any image editor that can export PNG with no alpha in the outer 10%.
- [x] 2.3 Add a one-paragraph note in `app/README.md` (create the file if absent — keep to <30 lines) explaining the two source files and that `make icons` regenerates derived assets.

## 3. Wire up the generator

- [x] 3.1 Edit `app/pubspec.yaml`'s `flutter_launcher_icons:` block: change `image_path` to `icon_source.png`. (No favicon flag needed — `flutter_launcher_icons` 0.14.4 emits the favicon by default when `web.generate: true`.) `background_color` and `theme_color` stay at `#080C1A`.
- [x] 3.2 Add a top-level `icons` target to the root `Makefile` that runs `dart run flutter_launcher_icons` then overwrites the maskable PNGs from `icon_source_maskable.png` via `magick`. Document the `convert` fallback for Linux contributors in a comment.
- [x] 3.3 Extend the `make icons` target with an extra step that generates `app/web/favicon-32.png` (32×32) from `app/icon_source.png`. Required magick flags for reproducibility and test compatibility: `-strip -define png:exclude-chunk=time,tIME -define png:color-type=6 -depth 8` (strip metadata + drop time chunk for idempotency, force RGBA so alpha-channel checks work, force 8-bit so alpha reads as 0–255).
- [x] 3.4 Update `app/web/index.html`: add `<link rel="icon" type="image/png" sizes="16x16" href="favicon.png" />`, `<link rel="icon" type="image/png" sizes="32x32" href="favicon-32.png" />`, and `<link rel="icon" type="image/png" sizes="192x192" href="icons/Icon-192.png" />` so modern browsers can pick the sharper variant for high-DPI tab rendering.

## 4. Regenerate and commit derived assets

- [x] 4.1 Run `make icons` locally.
- [x] 4.2 Inspect the regenerated `app/web/favicon-32.png` visually — confirm it shows the brand cluster legibly at 32×32 (a recognizable improvement over the 16×16 favicon.png that previously looked like a tiny dark blob in browser tabs).
- [x] 4.3 Inspect each maskable PNG visually — confirm the background fills to all four corners with no transparency, with the logo centered at ~70% scale.
- [x] 4.4 Re-run `flutter test test/assets/icons_test.dart` — all tests GREEN. Also verify `make icons; sha; make icons; sha; diff` produces no diff (the `make icons` idempotency requirement from the spec).

## 5. Spec + integration smoke

- [x] 5.1 `cd app && flutter analyze --fatal-infos` → 0 issues.
- [x] 5.2 `cd app && flutter test` → all green (full suite, not just the new test). 824 tests pass.
- [x] 5.3 `flutter build web --release` → web bundle builds; `build/web/` contains favicon-32.png + regenerated maskable PNGs; `build/web/index.html` carries the new `<link rel="icon" sizes="...">` tags. (Full `make build` skipped — that also runs the Rust workspace build; out of scope for an icons change. User can verify in browser locally.)
- [ ] 5.4 Manual PWA install on an Android device (or Chrome's "Add to Home screen" emulation): icon on home screen is the full brand logo, NOT a tiny double-rounded version inside a transparent square. **Pending user — requires real device or Chrome's PWA install flow.**
- [x] 5.5 `openspec validate consistent-app-icons` → passes.

## 6. Ship

- [ ] 6.1 PR: `fix(app): web favicon + PWA maskable icons match brand`. Body shows before/after screenshots of the favicon tab and Android home-screen install.
- [ ] 6.2 Merge.
- [ ] 6.3 Archive: `openspec archive consistent-app-icons`.
