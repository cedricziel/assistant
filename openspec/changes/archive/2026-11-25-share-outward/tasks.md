## Definition of Done

- [x] `flutter build ios --no-codesign` succeeds (updated entitlements, no signing errors in CI)
- [x] `flutter build macos` succeeds (via xcodebuild with CODE_SIGNING_ALLOWED=NO)
- [x] `flutter build web` succeeds
- [x] `flutter analyze --fatal-infos` passes
- [x] `flutter test` passes (including new adaptive widget tests)
- [ ] Long-press on image → context menu works on iOS simulator and macOS
- [ ] Long-press on audio → context menu works on iOS simulator and macOS
- [ ] Text selection → extended toolbar with Share/Save works on iOS simulator
- [ ] Save As writes file to user-chosen location (macOS NSSavePanel, iOS UIDocumentPicker)
- [ ] Share opens system share sheet on iOS and macOS
- [ ] Web: Save triggers browser download, Share uses Web Share API (or fallback)
- [x] Entitlement: `com.apple.security.files.user-selected.read-write` replaces `read-only` in macOS entitlements

## Entitlement Changes

macOS `DebugProfile.entitlements` and `Release.entitlements`:

```diff
- <key>com.apple.security.files.user-selected.read-only</key>
+ <key>com.apple.security.files.user-selected.read-write</key>
  <true/>
```

No new iOS entitlements needed — `UIDocumentPickerViewController` and `UIActivityViewController` don't require additional entitlements. The existing App Group and Keychain groups are sufficient.

---

## Tasks

### 1. Platform share/save service abstraction

- [x] Create `app/lib/services/share_service.dart` with `ShareService` interface (`shareText`, `shareFile`, `saveFile`, `saveText`)
- [x] Implement iOS/macOS method channel for `saveFile` (NSSavePanel / UIDocumentPickerViewController export mode)
- [x] Implement web save via blob download (anchor element trick)
- [x] Evaluate `share_plus` for share actions — adopt if it covers iOS `UIActivityViewController` + macOS `NSSharingServicePicker` cleanly, otherwise use method channel
- [x] Add `share_plus` (or equivalent) to `pubspec.yaml`

### 2. Adaptive context menu widget

- [x] Create `app/lib/shared/platform/adaptive_context_menu.dart` with `AdaptiveMediaContextMenu` and `AdaptiveContextMenuAction`
- [x] Apple platforms: `CupertinoContextMenu` with lifted preview + blur
- [x] Material/web: long-press → `showMenu()` popup, `onSecondaryTap` for right-click
- [x] Add test `app/test/widget/platform/adaptive_context_menu_test.dart` verifying both platform branches via `debugDefaultTargetPlatformOverride`

### 3. Image context menu

- [x] Wrap image thumbnails in `_attachmentThumbnails()` with `AdaptiveMediaContextMenu`
- [x] Add actions: Save Image, Share Image, Copy Image
- [x] Implement byte-fetching from `attachment.url` with loading indicator
- [x] Test with still-loading images (ensure preview doesn't show broken placeholder)

### 4. Audio context menu

- [x] Wrap `_VoiceMessagePlayer` with `AdaptiveMediaContextMenu`
- [x] Wrap `AudioPlayerWidget` (TTS audio) with `AdaptiveMediaContextMenu`
- [x] Add actions: Save Audio, Share Audio
- [x] Handle byte-fetching: use in-memory `audioBytes` for user voice, fetch via `api.fetchAudio(audioId)` for TTS

### 5. Text selection toolbar extension

- [x] Investigate `flutter_smooth_markdown` support for `contextMenuBuilder` / `SelectionArea` customization
- [x] Add `contextMenuBuilder` to `SelectableText` (user messages) with Share / Save As actions
- [x] Add equivalent to assistant markdown rendering (if supported by the package)
- [x] Share action: shares selected text (or full message if no selection)
- [x] Save As action: saves as `.md` file with auto-generated filename
- [x] Fallback if markdown package doesn't support customization: add Share/Save to `_MetaActionRow`

### 6. Web-specific implementations

- [x] Image hover overlay with Save/Share buttons
- [x] Audio inline Save/Share buttons
- [x] Web Share API integration (with feature detection and graceful fallback to download)
- [x] Blob download helper for text/image/audio

### 7. Polish and integration

- [x] Verify no gesture conflicts between text selection and context menus
- [x] Haptic feedback on `CupertinoContextMenu` activation (verify Flutter default behavior)
- [x] Error handling: toast on failed fetch, dismiss menu gracefully
- [ ] Test on iOS, macOS, and web
