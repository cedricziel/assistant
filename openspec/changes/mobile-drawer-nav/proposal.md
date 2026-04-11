## Why

The Flutter app's mobile layout used a `NavigationBar` (Material 3 bottom nav) to show all 10 destinations. Material design recommends at most 5 bottom nav items; at 10 items the bar becomes unusable (requires scrolling, labels get clipped). A secondary "slim bar" above the bottom nav was proposed for user/server switching, further cluttering the bottom. Replacing the bottom nav with a drawer resolves all three issues: unlimited destinations, natural section separators, and a clear place for secondary actions.

## What Changes

- **`NavShell` narrow layout**: `NavigationBar` (`bottomNavigationBar`) replaced by a `Drawer` (`Scaffold.drawer`) with a `DrawerButton` in the `AppBar`.
- **Drawer structure**: `DrawerHeader` (placeholder, future user/server switcher) → primary destinations → `Divider` → Settings → PWA install option (when installable).
- **Removed**: `_InstallBanner` widget (the slim install bar above the old bottom nav); PWA install option moved into the drawer.
- **Unchanged**: Wide layout (`NavigationRail`, ≥ 768 px) is unaffected.

## Capabilities

### Modified Capabilities

- **Mobile navigation** (`app/lib/shared/nav_shell.dart`): narrow layout now uses a drawer. All 10 destinations accessible; Settings visually separated by a `Divider`. Drawer closes automatically after destination tap.
- **PWA install flow (mobile)**: install option now lives in the drawer instead of a banner above the bottom nav.

## Impact

- **Modified**: `app/lib/shared/nav_shell.dart`
- **Added**: `app/test/widget/nav_shell_test.dart` — 13 widget tests covering narrow/wide structure, drawer contents, Settings section, PWA install visibility, and drawer-close-on-navigation.
- **No breaking changes** to routing, providers, or any server-side code.
