## Why

Users can copy message text to the clipboard, but there is no way to share or save images, voice attachments, or formatted message text from the app. On Apple platforms, long-press context menus and the share sheet are the standard gestures for exporting content — their absence makes the assistant feel like a walled garden where content goes in but doesn't come out.

## What Changes

- **Extend the text selection toolbar** (`contextMenuBuilder`) on iOS/macOS with "Share" and "Save As" actions alongside the standard Copy/Paste items. When text is selected, the user can share or save the selection directly from the native popover.
- **Add `CupertinoContextMenu` to image thumbnails** so long-press on an attached image shows Save Image / Share Image / Copy actions with a lifted preview.
- **Add `CupertinoContextMenu` to audio player widgets** (both user voice messages and TTS-generated audio) with Save Audio / Share Audio actions.
- **Implement platform-specific share/save services**:
  - iOS: `UIActivityViewController` (share), `UIDocumentPickerViewController` in export mode (save)
  - macOS: `NSSharingServicePicker` (share), `NSSavePanel` (save)
  - Web: Web Share API with fallback to blob download (share), blob download via anchor element (save)
- **Keep `_MetaActionRow`** for whole-message actions (Copy all, Read Aloud, Retry) that don't conflict with content-level gestures.

## Non-goals

- **Inward sharing** (share sheet into the app) — covered by `share-files-native`.
- **Conversation export** (full thread as PDF/HTML) — separate future feature.
- **Android-specific share intents** — Apple platforms and web only for v1.
- **Sharing tool call results or thinking blocks** — only user/assistant message content, images, and audio.

## Capabilities

### New Capabilities

- `share-outward-text`: Extended selection toolbar on iOS/macOS with Share/Save actions for selected text or whole message content.
- `share-outward-media`: `CupertinoContextMenu` on image thumbnails and audio player widgets with platform-native Save/Share actions.
- `share-save-service`: Platform abstraction layer dispatching to `UIActivityViewController`/`NSSharingServicePicker` (share) and `UIDocumentPickerViewController`/`NSSavePanel`/blob-download (save).

### Modified Capabilities

- `chat-messages`: Message bubble rendering gains `CupertinoContextMenu` wrappers around image and audio content. `SelectableText` and `StreamMarkdown` gain custom `contextMenuBuilder` on Apple platforms.

## Impact

- **Flutter app only** — no backend/Rust changes required. All content (text, image bytes, audio bytes) is already available client-side or fetchable via existing API endpoints.
- **New dependencies**: Likely `share_plus` for cross-platform share sheet invocation, or a thin platform channel for finer control over `NSSavePanel`/`UIDocumentPickerViewController`.
- **Platform channels**: May need a method channel for "Save As" (file picker in export mode) if no existing package covers iOS `UIDocumentPickerViewController` export + macOS `NSSavePanel` cleanly.
- **Files touched**: `chat_screen.dart` (bubble wrappers, context menus), new `share_service.dart` (platform dispatch), `pubspec.yaml` (dependencies).
