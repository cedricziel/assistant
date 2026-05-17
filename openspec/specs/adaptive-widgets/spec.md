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
## Requirements
### Requirement: Expanded façade widget catalogue

The façade at `app/lib/shared/platform/` SHALL provide adaptive wrappers for the following widgets, each branching on `AppPlatformStyle` from `platform.dart` and rendering platform-appropriate underlyings: `AdaptiveListSection`, `AdaptiveListTile`, `AdaptiveSwitch`, `AdaptiveSwitchTile`, `AdaptiveActionSheet`, `AdaptiveButton`, `AdaptiveIcons`, `AdaptiveTextField`, `AdaptiveSlider`, `AdaptiveSnackBar`. Each wrapper SHALL ship with a widget test exercising the iOS, Material, and macOS paths.

#### Scenario: Settings screen on iOS uses adaptive list section

- **WHEN** the Settings screen is built on iPhone or iPad running iOS 26
- **THEN** sections SHALL render as `CupertinoListSection.insetGrouped` with `CupertinoListTile` rows
- **THEN** switches inside those rows SHALL render as `CupertinoSwitch`
- **THEN** the Settings screen file SHALL NOT import `package:flutter/cupertino.dart`

#### Scenario: Settings screen on web uses Material list

- **WHEN** the Settings screen is built in a desktop browser
- **THEN** sections SHALL render with Material list styling (section header + tiles)
- **THEN** switches SHALL render as Material `Switch`
- **THEN** the file SHALL NOT import `package:flutter/cupertino.dart`

#### Scenario: Chat composer uses adaptive text field on iOS

- **WHEN** the Chat screen composer is built on iOS 26
- **THEN** the text input SHALL render as a `CupertinoTextField` (or its `adaptive_platform_ui` iOS 26 native equivalent if adopted in Phase 3)
- **THEN** the Chat screen file SHALL NOT import `package:flutter/cupertino.dart`

#### Scenario: Action sheets use adaptive wrapper

- **WHEN** a destructive action (delete conversation, delete persona, etc.) requests confirmation on iOS
- **THEN** the rendered sheet SHALL be a `CupertinoActionSheet` presented via `showCupertinoModalPopup`
- **WHEN** the same action is requested in a desktop browser
- **THEN** the rendered sheet SHALL be a Material modal bottom sheet

### Requirement: Phase 3 adoption of adaptive_platform_ui input widgets

For each of `AdaptiveTextField`, `AdaptiveSwitch`, `AdaptiveSlider`, and `AdaptiveSnackBar`, the iOS path SHALL use the `adaptive_platform_ui` package widget IF an A/B comparison against Flutter Cupertino on iOS 26 hardware shows a visible improvement. Otherwise the iOS path SHALL stay on Flutter Cupertino. The decision per widget SHALL be recorded in the change's `tasks.md` and the wrapper file's docstring. Web, macOS, and Android paths SHALL NOT invoke the package under any circumstance.

#### Scenario: Adoption decision is recorded per widget

- **WHEN** Phase 3 lands
- **THEN** each of the four wrappers above SHALL contain a comment naming the iOS implementation (`adaptive_platform_ui` widget name OR `CupertinoXxx`) and the rationale for the choice
- **THEN** `tasks.md` SHALL list the A/B outcome for each widget

#### Scenario: Web path is unchanged

- **WHEN** any of the four wrappers above is built in a desktop browser
- **THEN** the package code path SHALL NOT execute
- **THEN** the rendered widget SHALL be the Material equivalent

### Requirement: No direct platform branching in feature code

Files under `app/lib/features/**` and `app/lib/shared/**` (excluding `app/lib/shared/platform/**`) SHALL NOT branch on `defaultTargetPlatform`, `Theme.of(context).platform`, or `Platform.is*` for rendering decisions. Platform-conditional rendering SHALL flow through the façade wrappers. Non-rendering platform branches (e.g. choosing a file-system API in services) remain permitted.

#### Scenario: A new feature screen needs platform-specific UI

- **WHEN** a developer writes a feature screen that needs a different widget on iOS vs Material
- **THEN** the developer SHALL extend or compose existing façade wrappers, or add a new wrapper under `app/lib/shared/platform/`
- **THEN** the feature screen file SHALL NOT contain `if (Platform.isIOS)` or `if (defaultTargetPlatform == TargetPlatform.iOS)` for rendering decisions

#### Scenario: Existing non-rendering branch is allowed

- **WHEN** a service file (e.g. `installer_launcher.dart`, `share_service.dart`) checks `Platform.isMacOS` to decide which subprocess to spawn
- **THEN** the lint SHALL allow it (no façade exists for non-rendering decisions)

