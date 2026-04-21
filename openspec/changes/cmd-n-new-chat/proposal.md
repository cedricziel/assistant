## Why

The chat screen lacks a keyboard shortcut for starting a new conversation. Power users expect Cmd+N (macOS) / Ctrl+N (other platforms) to create a fresh chat — this is a standard UX convention across messaging and productivity apps. Currently, users must click the "New Chat" button in the sidebar, which breaks keyboard-driven workflows.

## What Changes

- Add a global keyboard shortcut (Cmd+N / Ctrl+N) that creates a new conversation and navigates to it
- The shortcut should work from any screen within the app, not just when the chat input is focused
- Reuse the existing `ConversationListNotifier.createConversation()` flow and `context.go('/chat/$id')` navigation

## Non-goals

- Adding a customizable keybinding system or shortcut preferences
- Adding other keyboard shortcuts (Cmd+W to close, Cmd+K for search, etc.) — those can come later
- Changing the existing "New Chat" button behavior in the sidebar

## Capabilities

### New Capabilities

- `keyboard-new-chat`: Global Cmd+N / Ctrl+N shortcut that creates a new conversation and navigates to it

### Modified Capabilities

_(none)_

## Impact

- **Flutter app only** — no backend or API changes required
- Affected files: `ChatScreen` widget (keyboard listener), potentially `NavShell` or `app_router.dart` for global scope
- The existing `KeyboardListener` in `chat_screen.dart` already handles Cmd+V for clipboard paste, so the pattern is established
- No new dependencies needed — uses Flutter's built-in `Shortcuts`/`Actions` or `KeyboardListener` widgets
