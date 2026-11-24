## Context

The assistant app renders messages in `_MessageBubble` widgets within `chat_screen.dart`. Current state:

- **User text messages**: rendered with `SelectableText` — long-press triggers text selection.
- **Assistant text messages**: rendered with `StreamMarkdown(selectable: true)` — same long-press text selection.
- **Image attachments**: rendered via `CachedNetworkImage` in `_attachmentThumbnails()`. No gesture handler beyond tap-to-fullscreen.
- **Audio (user voice)**: `_VoiceMessagePlayer` widget with in-memory `audioBytes`.
- **Audio (assistant TTS)**: `AudioPlayerWidget` fetching from server via `audioId`.
- **Existing actions**: `_MetaActionRow` below each bubble with Copy, Read Aloud, Retry buttons — always visible.

Platform detection is handled by `platformStyle` in `shared/platform/platform.dart` (three buckets: `cupertino`, `macos`, `material`). The app uses an **adaptive widget convention**: `shared/platform/adaptive_*.dart` files provide widgets that branch on `isAppleTouch` / `isMacOS` to render Cupertino or Material variants. Examples: `AdaptiveScaffold`, `AdaptiveNavBar`, `AdaptiveApp`, `showAdaptiveConfirmDialog`. Tests in `test/widget/platform/adaptive_*_test.dart` verify both branches via `debugDefaultTargetPlatformOverride`.

## Goals / Non-Goals

**Goals:**

- Long-press on image → native context menu with Save / Share / Copy
- Long-press on audio player → native context menu with Save / Share
- Text selection toolbar extended with Share / Save on Apple platforms
- Web gets equivalent functionality via download and Web Share API
- No gesture conflicts between text selection and context menus
- Platform-native feel on iOS and macOS

**Non-Goals:**

- Replacing `_MetaActionRow` entirely (it stays for whole-message actions)
- Custom context menu on web (use inline actions / right-click)
- Sharing structured data (JSON tool results, thinking blocks)

## Decisions

### D1: Per-content-type gesture mapping (no unified long-press)

Text, images, and audio each get the gesture that makes sense for their content type:

| Content | Gesture                       | Result                              |
| ------- | ----------------------------- | ----------------------------------- |
| Text    | Long-press → select → toolbar | Extended toolbar with Share/Save    |
| Image   | Long-press                    | `CupertinoContextMenu` with preview |
| Audio   | Long-press                    | `CupertinoContextMenu`              |

**Why:** A single long-press gesture for "message actions" conflicts with text selection, which is the established iOS behavior for text. Splitting by content type means each gesture does the expected thing for that content type — no disambiguation needed.

**Trade-off:** Two visual patterns within one bubble (toolbar popover for text, lifted context menu for media). Acceptable because users already expect these different patterns for different content types across iOS.

### D2: `contextMenuBuilder` for text selection toolbar extension

Use Flutter's `contextMenuBuilder` parameter on `SelectableText` and the markdown widget's selection area to inject custom actions into the native selection toolbar.

```dart
SelectableText(
  message.content,
  contextMenuBuilder: (context, editableTextState) {
    return AdaptiveTextSelectionToolbar.buttonItems(
      anchors: editableTextState.contextMenuAnchors,
      buttonItems: [
        ...editableTextState.contextMenuButtonItems, // Copy, Select All, etc.
        ContextMenuButtonItem(
          label: 'Share',
          onPressed: () => _shareText(editableTextState.textEditingValue.selection),
        ),
        ContextMenuButtonItem(
          label: 'Save As...',
          onPressed: () => _saveText(editableTextState.textEditingValue.selection),
        ),
      ],
    );
  },
)
```

**Why over custom gesture recognizer:** This extends the platform's own toolbar rather than fighting it. The user sees Share/Save alongside Copy in the same familiar popover. Zero gesture conflicts.

**Why over a separate button:** Putting Share in the selection toolbar means the action applies to the _selected_ text. The user can share a snippet rather than the whole message.

### D3: `AdaptiveMediaContextMenu` for images and audio

Follow the existing adaptive widget convention (`shared/platform/adaptive_*.dart`) to create an `AdaptiveMediaContextMenu` widget that wraps media content with platform-appropriate context menu behavior.

```dart
/// Platform-adaptive context menu for media content (images, audio).
///
/// - Apple touch (iOS/iPadOS): [CupertinoContextMenu] with lifted preview + blur.
/// - macOS: [CupertinoContextMenu] (same visual, right-click also triggers).
/// - Material/web: long-press → [showMenu] popup, or hover overlay on web.
class AdaptiveMediaContextMenu extends StatelessWidget {
  const AdaptiveMediaContextMenu({
    super.key,
    required this.child,
    required this.actions,
  });

  final Widget child;
  final List<AdaptiveContextMenuAction> actions;

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch || isMacOS) {
      return CupertinoContextMenu(
        actions: actions
            .map((a) => CupertinoContextMenuAction(
                  onPressed: () {
                    Navigator.of(context, rootNavigator: true).pop();
                    a.onPressed();
                  },
                  trailingIcon: a.cupertinoIcon,
                  isDestructiveAction: a.isDestructive,
                  child: Text(a.label),
                ))
            .toList(),
        child: child,
      );
    }
    // Material / web fallback
    return GestureDetector(
      onLongPress: () => _showMaterialMenu(context),
      onSecondaryTap: () => _showMaterialMenu(context),
      child: child,
    );
  }
}

/// A single action for [AdaptiveMediaContextMenu].
class AdaptiveContextMenuAction {
  const AdaptiveContextMenuAction({
    required this.label,
    required this.onPressed,
    this.cupertinoIcon,
    this.materialIcon,
    this.isDestructive = false,
  });

  final String label;
  final VoidCallback onPressed;
  final IconData? cupertinoIcon;
  final IconData? materialIcon;
  final bool isDestructive;
}
```

File: `app/lib/shared/platform/adaptive_context_menu.dart`

**Why an adaptive widget over inline `if (isAppleTouch)`:** Follows the established project convention. Keeps platform branching in one place. Enables testing both paths via `debugDefaultTargetPlatformOverride` (like the existing `adaptive_*_test.dart` files).

**Why `CupertinoContextMenu` over `showCupertinoModalPopup`:** The lifted-preview-with-blur effect is the standard iOS pattern for actionable media (Photos app, Safari image long-press, iMessage). It signals "this item has actions" through the animation.

### D4: Platform share/save service abstraction

Create a `ShareService` class that dispatches to the correct platform mechanism:

```
ShareService
  ├── shareText(String text)
  ├── shareFile(Uint8List bytes, String filename, String mimeType)
  ├── saveFile(Uint8List bytes, String filename, String mimeType)
  └── saveText(String text, String suggestedFilename)
```

Implementation per platform:

| Method      | iOS                                       | macOS                                  | Web                                            |
| ----------- | ----------------------------------------- | -------------------------------------- | ---------------------------------------------- |
| `shareText` | `UIActivityViewController` with text      | `NSSharingServicePicker`               | Web Share API → fallback: copy + toast         |
| `shareFile` | `UIActivityViewController` with file URL  | `NSSharingServicePicker` with file URL | Web Share API with `File` → fallback: download |
| `saveFile`  | `UIDocumentPickerViewController` (export) | `NSSavePanel`                          | Blob download via anchor                       |
| `saveText`  | `UIDocumentPickerViewController` (export) | `NSSavePanel`                          | Blob download via anchor                       |

**Why a custom service over `share_plus`:** `share_plus` handles `shareText` and `shareFile` well, but doesn't cover "Save As" (file export with picker). We need both sharing (to people/apps) and saving (to filesystem with location choice). A single abstraction covers both and keeps the UI code clean.

**Alternative:** Use `share_plus` for sharing + `file_saver` or a method channel for saving. This avoids writing platform channels but adds two dependencies. Viable for v1 if the packages cover the needed APIs.

### D5: Fetching bytes before share/save

Images and audio must be downloaded from the server before they can be shared or saved:

- **Images**: Fetch from `attachment.url` (relative path resolved against `imageBaseUrl`)
- **User voice**: Already in memory as `message.audioBytes` during the session
- **TTS audio**: Fetch via `api.fetchAudio(audioId)`

Show a brief loading indicator (spinner overlay on the context menu action) while fetching. If the fetch fails, show a toast error and dismiss the menu.

**Why not pre-fetch:** Messages may contain multiple images. Pre-fetching all of them wastes bandwidth. Fetch on-demand when the user explicitly requests share/save.

### D6: Saved file naming convention

| Content              | Default filename                           |
| -------------------- | ------------------------------------------ |
| Text (whole message) | `assistant-message-{iso-date}.md`          |
| Text (selection)     | `assistant-snippet-{iso-date}.md`          |
| Image                | Original `attachment.filename` from server |
| Voice (user)         | `voice-message-{iso-date}.{ext}`           |
| Audio (TTS)          | `assistant-audio-{iso-date}.{ext}`         |

On macOS (`NSSavePanel`), the user can change the filename. On iOS (`UIDocumentPickerViewController` export mode), the default is used but the user picks the destination folder.

### D7: Web fallback — no `CupertinoContextMenu`, use inline actions

On web, there is no `CupertinoContextMenu` equivalent. Instead:

- Image thumbnails get a hover overlay with Save/Share icon buttons (similar to how image viewers work on web).
- Audio players get Save/Share buttons inline (next to the play controls).
- Text keeps the `_MetaActionRow` Copy action; browsers provide their own right-click → "Copy" for selected text.

This is acceptable because web users expect hover-based actions, not long-press.

## Risks / Trade-offs

**[Risk] `contextMenuBuilder` not supported by `StreamMarkdown`** → The `flutter_smooth_markdown` package may not expose `contextMenuBuilder` on its internal `SelectionArea`. If so, wrap the markdown widget in a `SelectionArea` with a custom `contextMenuBuilder` at the parent level. Fallback: add Share/Save to `_MetaActionRow` only.

**[Risk] `CupertinoContextMenu` animation janky with `CachedNetworkImage`** → The lift animation clones the child widget. If the image is still loading, the preview may show a placeholder. Mitigation: only wrap images that have finished loading (check `CachedNetworkImage` state or use a `frameBuilder`).

**[Risk] Large audio files slow to fetch before share** → TTS audio and voice recordings are typically small (<5 MB). If a fetch takes >2 seconds, show a progress indicator within the context menu action. Cancel on dismiss.

**[Trade-off] Two menu patterns per bubble** → Users see a selection toolbar for text and a lifted context menu for images/audio within the same message. This matches how iOS itself handles mixed content (e.g., Safari shows text selection toolbar for text and a context menu for images on the same page).

**[Trade-off] `share_plus` vs custom platform channel** → `share_plus` is well-maintained and handles 80% of the share use case. "Save As" with a file picker is the gap. Options: (a) use `share_plus` + `file_saver`, (b) write a single method channel for the save picker. Decision can be made during implementation based on package quality.

## Open Questions

- **`StreamMarkdown` selection customization**: Does `flutter_smooth_markdown` support `contextMenuBuilder` or `selectionControls` override? Needs investigation.
- **macOS `NSSavePanel` from Flutter**: Is there an existing package, or do we need a method channel? `file_saver` may only do Downloads folder without a picker.
- **Haptic feedback**: Should the `CupertinoContextMenu` trigger a haptic on iOS? (Default Flutter behavior may already include this.)
