## Context

The Flutter app already has a keyboard event handler in `ChatScreen` (`chat_screen.dart` line ~1956) that uses `KeyboardListener` to detect Cmd+V for clipboard image paste. The conversation creation flow is well-established: `ConversationListNotifier.createConversation()` calls the API, and navigation uses `context.go('/chat/$id')` via go_router. The "New Chat" button in `ConversationList` follows this exact flow.

The shortcut must work globally (not just in the chat screen), which means it cannot live inside the existing `KeyboardListener` on the chat input. It needs to be registered higher in the widget tree.

## Goals / Non-Goals

**Goals:**

- Register Cmd+N / Ctrl+N as a global shortcut that creates a new conversation
- Work across all screens in the app
- Reuse the existing conversation creation and navigation infrastructure
- Debounce to prevent duplicate creations on rapid key presses

**Non-Goals:**

- Building a general-purpose keyboard shortcut framework
- Adding shortcut customization or preferences UI
- Handling offline/error states differently from the existing "New Chat" button

## Decisions

### 1. Use Flutter `Shortcuts` + `Actions` widgets at the `MaterialApp` / shell level

**Rationale:** Flutter's `Shortcuts`/`Actions` system is the idiomatic way to register global keyboard shortcuts. It propagates through the widget tree and is automatically platform-aware. Placing it at the `NavShell` or `MaterialApp.builder` level ensures it captures key events regardless of which screen is active.

**Alternative considered:** Adding another `KeyboardListener` to `ChatScreen` — rejected because it would only work on the chat screen, not globally.

**Alternative considered:** `RawKeyboardListener` at the root — rejected because `Shortcuts`/`Actions` is the higher-level, recommended API and handles platform modifier key mapping (Meta on macOS, Control elsewhere) cleanly via `SingleActivator`.

### 2. Use `SingleActivator(LogicalKeyboardKey.keyN, meta: true)` for macOS and `SingleActivator(LogicalKeyboardKey.keyN, control: true)` for other platforms

**Rationale:** `SingleActivator` is the standard Flutter shortcut activator. Registering both meta (Cmd) and control variants with platform detection ensures correct behavior across macOS, web-on-Windows, and web-on-Linux.

### 3. Guard against duplicate creation with a simple boolean flag

**Rationale:** A `_creatingConversation` flag in the action callback prevents concurrent API calls when the user taps Cmd+N rapidly. This is simpler than debouncing with timers and matches the existing pattern where the "New Chat" button disables during creation.

### 4. Place the `Shortcuts`/`Actions` wrapper in the router shell widget

**Rationale:** `app/lib/router/app_router.dart` defines a `ShellRoute` with a `NavShell` builder that wraps all routed screens. Wrapping `NavShell`'s child with the shortcut widget ensures coverage across all routes while having access to both `WidgetRef` (for providers) and `BuildContext` (for navigation).

## Risks / Trade-offs

- **[Risk] Shortcut conflicts with browser Cmd+N (new window)** → Mitigation: On web, Flutter captures keyboard events within the canvas. The shortcut will only fire when the Flutter app has focus. Document this behavior for users.
- **[Risk] Text fields swallowing the key event** → Mitigation: `Shortcuts` widget at the shell level takes priority when the shortcut matches, before the event reaches text input widgets. Test explicitly with focused text fields.
- **[Risk] No visual feedback during creation** → Mitigation: Navigation to the new chat is itself the feedback. The creation API call is fast (<100ms locally). If needed later, a loading indicator can be added.
