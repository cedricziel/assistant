## Context

Existing state of icon assets in `app/` (verified visually):

| File                                    | Size   | Visual                                        | Correct?                                      |
| --------------------------------------- | ------ | --------------------------------------------- | --------------------------------------------- |
| `app/icon_source.png`                   | 238 KB | spiderweb, dark bg, rounded corners + padding | orphan — nothing references it                |
| `app/web/favicon.png`                   | 546 B  | Flutter default blue "F"                      | ❌ wrong                                      |
| `app/web/icons/Icon-192.png`            | 20 KB  | spiderweb, rounded corners                    | brand-correct but **not** what maskable needs |
| `app/web/icons/Icon-512.png`            | 84 KB  | spiderweb, rounded corners                    | brand-correct                                 |
| `app/web/icons/Icon-maskable-192.png`   | 20 KB  | identical to Icon-192                         | ❌ wrong purpose — has transparent corners    |
| `app/web/icons/Icon-maskable-512.png`   | 84 KB  | identical to Icon-512                         | ❌ wrong purpose                              |
| `app/macos/.../app_icon_1024.png`       | 238 KB | full-bleed spiderweb, no corner rounding      | byte-identical to `icon_source.png`           |
| `app/ios/.../Icon-App-1024x1024@1x.png` | 213 KB | rounded corners + padding                     | out of scope                                  |

`pubspec.yaml` (lines 55–60):

```yaml
flutter_launcher_icons:
  web:
    generate: true
    image_path: macos/Runner/Assets.xcassets/AppIcon.appiconset/app_icon_1024.png
    background_color: "#080C1A"
    theme_color: "#080C1A"
```

There is no `ios:` / `macos:` / `android:` block. `flutter_launcher_icons` 0.14.x ignores the `favicon` field unless explicitly enabled, and produces maskable variants by copying the source as-is (no automatic safe-zone padding). Web icons in the repo predate the current package version; the favicon is the Flutter template default that nothing ever overwrote.

The pre-existing `openspec/specs/pwa-app-icon/spec.md` already says favicon should be the brand icon and maskable should have a safe zone. Reality drifted from spec. This change re-aligns code with intent and tightens the spec with testable acceptance.

## Goals / Non-Goals

**Goals:**

- Web favicon, PWA 192, PWA 512, PWA maskable 192, PWA maskable 512 all visually match the brand icon already in use on iOS / macOS.
- The maskable variants survive Android's circle / squircle / teardrop mask with no transparent corners and the logo fully visible within the inner safe zone.
- A named source-of-truth file (`app/icon_source.png` + `app/icon_source_maskable.png`) is the canonical input. Regenerating from these files is one command.
- Drift detection: a test (or hook) catches accidental reversion to the Flutter default favicon and any future maskable-without-safe-zone regression.

**Non-Goals:**

- Touching iOS, macOS, or Android launcher icons (user explicitly says those are correct).
- Designing a new brand icon. We ship what already exists.
- Migrating to an SVG-first source. PNG stays.
- Auto-generating the maskable safe-zone variant from the square source. That'd need a custom script with PIL/ImageMagick; cleaner to commit a hand-prepped second PNG.

## Decisions

### Decision 1: Two source files, one obvious location

Adopt:

- `app/icon_source.png` — 1024×1024, full-bleed, current artwork. Used for favicon + standard PWA 192/512. Already exists; just promote it to canonical.
- `app/icon_source_maskable.png` — 1024×1024, same artwork scaled to ~70% and centered, background filled to all edges with `#080C1A` (matches `theme_color`). NEW file. Used for maskable PWA 192/512.

Both live at `app/` root (not buried in a platform asset catalog) so they're discoverable. The `flutter_launcher_icons` config points at `app/icon_source.png`.

**Rejected alternative:** Single source + a build-time script that generates the maskable variant by padding. Adds a Python or `convert` dependency to CI for one image. Two committed PNGs is simpler and gets reviewed in PRs.

### Decision 2: `flutter_launcher_icons` generates web icons; maskable variants are committed manually

`flutter_launcher_icons` 0.14.4 doesn't accept a separate maskable source. It either skips maskable or generates them from the same source as the standard ones. Either way, it can't give us a proper safe-zone maskable from `icon_source.png` alone.

Therefore:

- Run the package for `web:` to regenerate `favicon.png` + `Icon-192.png` + `Icon-512.png` from `icon_source.png`.
- Generate `Icon-maskable-{192,512}.png` separately from `icon_source_maskable.png` via `sips` / `convert` / a tiny dart script invoked by the `make icons` target. Result is committed.

**Rejected alternative:** Wait for `flutter_launcher_icons` to support a `maskable_image_path` field. No such PR is merged upstream as of 0.14.4.

### Decision 3: Strengthen the spec with testable invariants

Acceptance criteria gain teeth:

- **Favicon**: `app/web/favicon.png` size > 1 KB AND SHA-256 ≠ the known Flutter-default favicon hash. Asserted by a unit test in `app/test/assets/icons_test.dart` (new file).
- **Maskable**: For each `Icon-maskable-*.png`, no pixel in the outer 10% border has alpha < 0xFF. Asserted by the same test using the `image` Dart package (already transitive in Flutter).

The test runs under `flutter test` and gates PRs via the existing flutter CI workflow. Drift gets caught immediately if someone re-overwrites the favicon by accident.

### Decision 4: `make icons` target lives in repo root, mirroring `make generate-flutter-client`

Symmetry with the existing API-client-generation workflow documented in `CLAUDE.md`. The target:

```
icons:
	cd app && dart run flutter_launcher_icons
	cd app && sips -z 192 192 icon_source_maskable.png --out web/icons/Icon-maskable-192.png
	cd app && sips -z 512 512 icon_source_maskable.png --out web/icons/Icon-maskable-512.png
```

`sips` is preinstalled on macOS; document a `convert` (ImageMagick) fallback for Linux contributors. CI doesn't run `make icons` — assets are committed.

## Risks / Trade-offs

- **`flutter_launcher_icons` may regenerate the favicon to something we don't expect.** Mitigation: the new test asserts the result is brand-correct (size + hash heuristic), so a bad generation fails CI.
- **Maskable safe-zone file is hand-prepped, so a contributor could re-export with wrong padding.** Mitigation: the outer-border-opaque test catches missing background fill.
- **Two source files invites drift between them** if the brand artwork updates. Mitigation: document in `app/icon_source.png` proximity (a `README.md` in `app/`) that both must be re-exported together.
- **macOS `sips` in the Makefile breaks Linux dev loops.** Acceptable: only people regenerating icons need it; CI never runs `make icons`. Document the `convert` alternative.

## Migration Plan

One PR, ordered:

1. Add `app/icon_source_maskable.png` (new file).
2. Update `pubspec.yaml` to point `image_path` at `app/icon_source.png`.
3. Add `make icons` target.
4. Run `make icons` locally; commit the regenerated favicon + standard PWA icons + maskable PWA icons.
5. Add `app/test/assets/icons_test.dart` with the favicon-hash and maskable-border-opaque assertions.
6. Verify locally: `flutter build web` → open `build/web/index.html` → check browser tab favicon → install as PWA on Android → confirm home-screen icon is the brand, not double-rounded.

Rollback: revert the PR; old (wrong) icons return. No DB or runtime migration needed.

## Open Questions

- Should the source files live in `app/icon_source/` as a directory (room to grow: dark variant, monochrome favicon SVG, etc.) instead of two siblings at `app/` root? Open to either; current proposal keeps them as root-level siblings for visibility.
- Do we want a Safari `mask-icon` (`<link rel="mask-icon">`) too? Out of scope unless someone reports Safari pinned-tab issues.
