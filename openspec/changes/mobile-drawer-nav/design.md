## Context

`NavShell` is the root shell widget wrapping all main screens. It branches on a `_kNavRailBreakpoint` (768 px): wide → `NavigationRail`; narrow → previously `NavigationBar`, now `Drawer`.

## Decision: Replace bottom navigation bar with Scaffold drawer on narrow screens

**Decision:** The narrow branch removes `bottomNavigationBar` entirely and adds `Scaffold.drawer` + `AppBar(leading: DrawerButton())`. The `DrawerButton` is always visible in the `AppBar`, making the drawer reachable from anywhere in the app without any extra state.

The drawer structure:

```
DrawerHeader          ← branding / future user–server switcher
─────────────────
ListTile: Chat        ← primary destinations (indices 0..N-2)
ListTile: Traces
ListTile: Logs
ListTile: Personas
ListTile: Skills
ListTile: Workflows
ListTile: Webhooks
ListTile: Agents
ListTile: Analytics
─────────────────
Divider
─────────────────
ListTile: Settings    ← secondary section
ListTile: Install App ← only when pwaInstallProvider == true
```

Each `ListTile.onTap` calls `context.go(path)` then `Navigator.of(context).pop()` to close the drawer after navigation.

`_InstallBanner` (the slim Material banner that previously sat above the bottom nav) is deleted — its purpose is now served by the `Install App` list tile in the drawer.

**Rationale:** Material 3 bottom nav supports 3–5 items. The app has 10 destinations. A drawer naturally handles any number and supports section grouping via `Divider`. A gesture-accessible drawer (swipe from left edge + always-visible `DrawerButton`) is effectively omnipresent and requires no extra tap target.

**Alternatives considered:**

- _Scrollable bottom nav_: Requires horizontal scrolling for destination discovery — poor UX for a primary nav pattern.
- _Separate settings screen launched from FAB_: Settings is a real destination, not a secondary action; should live in nav, not a FAB.
- _NavigationDrawer (Material 3 widget)_: Provides `NavigationDrawerDestination` tiles but limits flexibility for the header widget and the Settings divider. Plain `Drawer` + `ListView` + `ListTile`s is sufficient and more flexible.

## Future work

- `DrawerHeader` placeholder → replace with user/server switcher chip (connects to server profile provider).
- Wide layout: move Settings + user switcher to `NavigationRail.trailing` slot (separate change).
