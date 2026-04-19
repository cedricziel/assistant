## Dark Mode & Semantic Colors

### Summary

Add dark mode support across all platforms and replace all hard-coded color values with semantic color tokens from the theme's `ColorScheme`.

### Requirements

- **REQ-1**: `MaterialApp.router` receives a `darkTheme` parameter using `ColorScheme.fromSeed` with `brightness: Brightness.dark`.
- **REQ-2**: `CupertinoApp.router` uses `CupertinoThemeData` that respects `MediaQuery.platformBrightnessOf(context)`.
- **REQ-3**: The app follows system brightness preference — no manual toggle needed for v1 (a manual toggle can be added later).
- **REQ-4**: Replace all hard-coded color references with semantic tokens:
  - `Colors.black38` → `colorScheme.onSurfaceVariant`
  - `Colors.black54` → `colorScheme.onSurfaceVariant`
  - `Colors.black45` → `colorScheme.onSurfaceVariant`
  - `Colors.black26` → `colorScheme.outlineVariant`
  - `Colors.black12` → `colorScheme.outlineVariant`
  - `Colors.red` → `colorScheme.error`
  - `Colors.red.shade50` → `colorScheme.errorContainer`
  - `Colors.red.shade700` → `colorScheme.onErrorContainer`
  - `Colors.white` (on red backgrounds) → `colorScheme.onError`
- **REQ-5**: On Cupertino platforms, use `CupertinoColors` dynamic system colors where appropriate (they auto-adapt to dark mode).
- **REQ-6**: Chat message bubbles must be legible in both light and dark mode. User bubbles use `colorScheme.primary` / `colorScheme.onPrimary` (already the case). Assistant bubbles use `colorScheme.surfaceContainerHighest` / `colorScheme.onSurface` (already the case — but verify in dark mode).
- **REQ-7**: The streaming dots indicator (`_Dot` in chat_screen.dart) must use a semantic color, not `Colors.black38`.

### Files Requiring Color Cleanup

- `app/lib/features/chat/chat_screen.dart` — empty state, error banner, streaming dots, input row border, status message text
- `app/lib/features/chat/conversation_list.dart` — error state, empty state text, delete swipe background
- `app/lib/features/contexts/screens/context_switcher_screen.dart` — empty state icon/text (already uses `colorScheme.outline` — good)
- Any other file using `Colors.black*` or `Colors.red*`

### Acceptance Criteria

- App respects system dark mode on iOS Simulator (toggle in Settings > Developer > Dark Appearance).
- App respects system dark mode in Chrome (prefers-color-scheme media query).
- No hard-coded `Colors.black*` or `Colors.red*` remain in `app/lib/`.
- Chat bubbles are legible in both light and dark mode.
- Error banners are visible and readable in dark mode.
- Empty states are visible in dark mode.
