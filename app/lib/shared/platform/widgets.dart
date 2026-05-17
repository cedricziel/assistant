/// Façade barrel — the single public surface for UI primitives in feature code.
///
/// Feature code under `app/lib/features/**` and `app/lib/shared/**`
/// (excluding `app/lib/shared/platform/**` itself) SHOULD import this file
/// rather than `package:flutter/cupertino.dart` or
/// `package:flutter/material.dart` directly.
///
/// The Phase 4 lint rule will forbid direct cupertino.dart and material.dart
/// imports outside the allowlist (`lib/shared/platform/**` and `lib/main.dart`).
/// This barrel is the only sanctioned way for feature code to reach Material
/// primitives.
///
/// What's exported:
///
/// 1. Everything from `flutter/widgets.dart` (the neutral Flutter layer —
///    Container, Padding, Row, Column, Text, GestureDetector, etc.).
///
/// 2. A curated subset of `flutter/material.dart` for widgets that have no
///    meaningful iOS counterpart and for theming/type access. The `show`
///    list below is the discipline boundary — adding to it is a deliberate
///    act, and every new `Adaptive*` wrapper added in later phases removes
///    one entry from it (ratchet pattern).
///
/// 3. `CupertinoIcons` (constants only) from `flutter/cupertino.dart`, so
///    feature code can pass `cupertino:` IconData to `AdaptiveIcon` without
///    importing cupertino.dart directly. No Cupertino widget classes are
///    re-exported — those are reachable only via the `Adaptive*` wrappers.
///
/// 4. All `Adaptive*` wrappers from `lib/shared/platform/`, plus the
///    `platformStyle` / `AppPlatformStyle` helpers. Feature code needs only
///    one import to get the entire UI vocabulary.
///
/// Intentionally NOT in the material `show` list (force migration to
/// wrappers): `Scaffold`, `AppBar`, `ListTile`, `SwitchListTile`, `Switch`,
/// `TextField`, `Slider`, `SnackBar`, `AlertDialog`, `FilledButton`,
/// `TextButton`. Also NOT re-exported: `Colors` (gated by
/// `scripts/check_no_raw_colors.sh`).
library;

// 1. Neutral Flutter widgets — always safe.
export 'package:flutter/widgets.dart';

// 2. Curated Material subset.
export 'package:flutter/material.dart'
    show
        // Surface primitives without iOS equivalents.
        Card,
        InkWell,
        Material,
        MaterialType,
        // Icon glyph constants (Material side).
        Icons,
        // Theme access — Theme.of(context) and the data types it returns.
        Theme,
        ThemeData,
        ColorScheme,
        TextTheme,
        Brightness,
        // Indicators — CircularProgressIndicator.adaptive picks the right
        // glyph automatically, so the static class itself is re-exported.
        CircularProgressIndicator,
        // Buttons without a wrapper yet. AdaptiveIconButton, AdaptiveButton.outlined,
        // and AdaptivePopupMenu are future work; until they exist, these
        // remain in the show list. Each new wrapper removes one entry.
        IconButton,
        OutlinedButton,
        PopupMenuButton,
        PopupMenuItem,
        CheckedPopupMenuItem,
        PopupMenuDivider,
        PopupMenuEntry,
        // Material-only concepts (no Cupertino equivalent at all).
        FloatingActionButton,
        Tooltip,
        Divider,
        VerticalDivider,
        ExpansionTile,
        // Snackbar control surface — AdaptiveSnackBar wraps the rendering,
        // but feature code occasionally needs ScaffoldMessenger for in-flight
        // control (clearing snackbars, etc.).
        ScaffoldMessenger,
        ScaffoldMessengerState,
        SnackBarAction,
        // Navigation primitives used in router config.
        MaterialPageRoute,
        MaterialApp,
        // Drawer / TabBar are used in nav_shell — wrappers TBD.
        Drawer,
        DrawerHeader,
        TabBar,
        Tab,
        TabBarView,
        TabController,
        DefaultTabController,
        // Chip / Badge / Avatar style widgets — no iOS equivalent.
        Chip,
        Badge,
        CircleAvatar,
        // Form-aware list tile, occasionally used in settings.
        // (Settings will migrate to AdaptiveSwitchTile, but `CheckboxListTile`
        // is occasionally useful and has no Cupertino equivalent.)
        CheckboxListTile,
        // Ink ripple / hover / focus support used with InkWell.
        Ink,
        // Stepper for multi-step flows (Material-only pattern).
        Stepper,
        Step,
        // Bottom sheets — `showModalBottomSheet` is used via
        // AdaptiveActionSheet on iOS; the function itself is sometimes
        // needed directly for non-action sheets.
        showModalBottomSheet,
        // Dialogs — `showDialog` is sometimes needed for custom dialogs.
        // AdaptiveDialog covers the confirm pattern; this is the lower-level
        // entry point.
        showDialog,
        // Material localisations (used in date/time pickers).
        DefaultMaterialLocalizations;

// 3. Cupertino constants only — never Cupertino widget classes.
export 'package:flutter/cupertino.dart' show CupertinoIcons;

// 4. Façade wrappers and platform helpers.
export 'adaptive_action_sheet.dart';
export 'adaptive_app.dart';
export 'adaptive_button.dart';
export 'adaptive_context_menu.dart';
export 'adaptive_dialog.dart';
export 'adaptive_icon.dart';
export 'adaptive_list_section.dart';
export 'adaptive_list_tile.dart';
export 'adaptive_nav_bar.dart';
export 'adaptive_scaffold.dart';
export 'adaptive_slider.dart';
export 'adaptive_sliver_nav_bar.dart';
export 'adaptive_snack_bar.dart';
export 'adaptive_switch.dart';
export 'adaptive_switch_tile.dart';
export 'adaptive_text_field.dart';
export 'platform.dart';
