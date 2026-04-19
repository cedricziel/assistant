## Adaptive Navigation Shell

### Summary

The navigation shell (`NavShell`) becomes platform-aware, rendering Cupertino navigation on Apple touch platforms and Material navigation on web/macOS.

### Requirements

- **REQ-1**: On Apple touch platforms, compact width (< 768dp): render `CupertinoTabBar` at the bottom with the 4 primary destinations + a "More" item.
- **REQ-2**: On Apple touch platforms, regular width (>= 768dp): render a sidebar with all destinations (primary + overflow) using a styled `ListView`. The sidebar should visually match Apple's iPad sidebar pattern (translucent background, selected state highlight, SF Symbols icons via `cupertino_icons`).
- **REQ-3**: On Material platforms: preserve existing `NavigationBar` (compact) and `NavigationRail` (wide) behavior unchanged.
- **REQ-4**: The "More" overflow sheet on compact Apple platforms should use `showCupertinoModalPopup` with a `CupertinoActionSheet`-style presentation instead of `showModalBottomSheet`.
- **REQ-5**: The 768dp breakpoint is shared across all platforms (no platform-specific breakpoints).
- **REQ-6**: Active destination highlighting must work correctly for both primary and overflow routes on all platform variants.

### Acceptance Criteria

- iPhone Simulator shows CupertinoTabBar with 5 items (4 primary + More).
- iPad Simulator (landscape) shows sidebar list, no bottom tab bar.
- Web browser shows existing NavigationBar / NavigationRail unchanged.
- Tapping "More" on iPhone shows Cupertino-styled overflow menu.
- Route-based active state highlighting works on all variants.
