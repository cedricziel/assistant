## 1. Icon Generation

- [x] 1.1 Inspect `app/macos/Runner/Assets.xcassets/AppIcon.appiconset/app_icon_1024.png` to confirm it is the correct source asset
- [x] 1.2 Resize source icon to 192×192 and export as `app/web/icons/Icon-192.png`
- [x] 1.3 Resize source icon to 512×512 and export as `app/web/icons/Icon-512.png`
- [x] 1.4 Generate maskable 192×192 variant (icon centered with padding on solid background) and export as `app/web/icons/Icon-maskable-192.png`
- [x] 1.5 Generate maskable 512×512 variant (icon centered with padding on solid background) and export as `app/web/icons/Icon-maskable-512.png`
- [x] 1.6 Resize source icon to 16×16 or 32×32 and export as `app/web/favicon.png`

## 2. Manifest Update

- [x] 2.1 Determine the app's brand color from the macOS icon or existing brand guidelines
- [x] 2.2 Update `background_color` in `app/web/manifest.json` to the app's brand color
- [x] 2.3 Update `theme_color` in `app/web/manifest.json` to the app's brand color

## 3. Verification

- [x] 3.1 Run `flutter build web` in `app/` and confirm icons appear correctly in `app/build/web/icons/`
- [x] 3.2 Visually inspect each icon at its target size to confirm the logo is legible and not clipped
- [x] 3.3 Verify maskable icons keep content within the inner 80% safe zone (use a maskable icon preview tool)
- [x] 3.4 Load the built web app in Chrome and verify the favicon displays the correct icon in the browser tab
