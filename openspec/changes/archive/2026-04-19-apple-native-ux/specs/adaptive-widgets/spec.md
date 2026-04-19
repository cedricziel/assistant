## Adaptive Interactive Widgets

### Summary

Replace Material interactive widgets with their `.adaptive` or Cupertino equivalents on Apple touch platforms for a native interaction feel.

### Requirements

- **REQ-1**: `SwitchListTile` → `SwitchListTile.adaptive` in Settings screen (3 instances). Renders `CupertinoSwitch` on iOS.
- **REQ-2**: `CircularProgressIndicator` → `CircularProgressIndicator.adaptive` throughout the app. Renders iOS-style spinner on Apple platforms.
- **REQ-3**: `AlertDialog` for destructive confirmations → `CupertinoAlertDialog` on Apple touch platforms. Specifically:
  - Delete context confirmation (`context_switcher_screen.dart`)
  - Delete conversation confirmation (`conversation_list.dart`)
  - Any future destructive confirmation dialogs
  - Provide a `showAdaptiveConfirmDialog` helper in `adaptive_dialog.dart`.
- **REQ-4**: Chat input `TextField` → `CupertinoTextField` on Apple touch platforms. Use rounded-rect decoration (iOS style), placeholder text, and a send button.
- **REQ-5**: Haptic feedback (`HapticFeedback.lightImpact()`) on key actions on Apple touch platforms:
  - Sending a message
  - Selecting a conversation
  - Toggling a switch
  - Deleting an item
- **REQ-6**: Settings screen uses `CupertinoListSection.insetGrouped` with `CupertinoListTile` on Apple touch platforms for the iOS Settings look.
- **REQ-7**: Pull-to-refresh on list screens uses `CupertinoSliverRefreshControl` on Apple touch platforms instead of `RefreshIndicator`.

### Acceptance Criteria

- Switches in Settings render as CupertinoSwitch on iPhone Simulator.
- Loading spinners render as iOS-style on iPhone Simulator.
- Delete confirmation dialog renders as CupertinoAlertDialog (stacked buttons, iOS style) on iPhone.
- Chat input field has rounded-rect appearance on iPhone.
- Haptic feedback fires on message send (verify via Simulator console or physical device).
- Settings screen shows grouped inset list sections on iPhone.
- All widgets render as Material on web — no regression.
