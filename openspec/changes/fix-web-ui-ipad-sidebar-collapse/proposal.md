## Why

On iPad in landscape orientation, users cannot meaningfully collapse the navigation sidebar:

- iPad Safari and iPad PWA report `kIsWeb == true`, so the app falls into the **Material wide** branch of `NavShell`, not the Cupertino branch. The Material wide branch _does_ render a toggle button inside the sidebar (`Icons.menu_open`, `nav_shell.dart:627`), but it sits on top of the sidebar list — visually small, easy to miss, and pinned to the right edge of an already-narrow rail.
- `SidebarCollapsedNotifier` (`nav_shell.dart:29`) is a plain `Notifier<bool>` that starts at `false` and lives in memory only. Every page reload re-expands the sidebar, undoing the user's preference.
- Native iPad (Catalyst / iOS app) takes the Cupertino branch, which has a Cupertino toggle button overlaid at the top-leading corner — but that toggle only renders inside the `Stack` above the sidebar and is easy to confuse with the conversation header.

The result is exactly what the bug report describes: on iPad landscape the sidebar feels permanently fixed, eats screen real estate, and doesn't remember what the user wanted.

## What Changes

- Persist `sidebarCollapsedProvider` across reloads via `SharedPreferences` (key `assistant.sidebarCollapsed`). State rehydrates on first `build()` so refresh keeps the user's choice.
- On the Material wide branch, surface a primary collapse / expand affordance in the **app bar / top-leading area of the main content** (not buried inside the sidebar), matching the Cupertino branch's overlay position. Keep the existing in-sidebar toggle for discoverability.
- Add a swipe-from-left-edge gesture on touch platforms (`iOS`, iPad PWA via touch input) that toggles the sidebar — wired into the same `sidebarCollapsedProvider`.
- Verify the Material wide branch actually triggers at the iPad-landscape width (1024px, well above 768px breakpoint) and that the sidebar can shrink to `_kSidebarCollapsedWidth` (72px) without overflow.
- Add Playwright + widget coverage for the iPad-landscape viewport (1180×820) verifying the toggle is visible, tappable, and that the persisted state survives a reload.

## Non-goals

- Redesigning the sidebar's contents.
- Adding multi-pane layouts (independent navigation + chat panes).
- Persisting other UI state (theme, density, etc.) — this change limits itself to sidebar collapse.
- Replacing `cupertino_sidebar` / `CupertinoSidebarCollapsible` — we reuse the existing collapsible widget.

## Capabilities

### Modified Capabilities

- `adaptive-shell` — REQ-2 already mandates the iPad sidebar exists; new requirements pin down its collapse behaviour and persistence.

### Added Capabilities

- `sidebar-collapse-state` (new spec) — the rules around persistence and the toggle affordance.

## Impact

- `app/lib/shared/nav_shell.dart` — extract the collapse toggle into a reusable widget, render it in the main content top bar in addition to the in-sidebar location, and add swipe-gesture handling.
- `app/lib/shared/nav_shell.dart` — replace `SidebarCollapsedNotifier` with an `AsyncNotifier<bool>` (or hydrated `Notifier`) backed by `SharedPreferences`.
- `app/test/widget/nav_shell_test.dart` — widget tests for the new toggle and persistence behaviour.
- `app/test/e2e/sidebar_collapse_ipad_test.dart` (Playwright) — visual + state regression at iPad-landscape viewport.
- No backend changes.

## Visual / UI change

Yes — the main content area gains a top-leading toggle button on the Material wide branch. Playwright iPad-landscape baselines for routes that render `NavShell` will move. Sidebar visuals otherwise unchanged.

## User-facing documentation

Brief note in `docs/operations/web-ui-shortcuts.md` (or the closest existing reference) describing the toggle and the new swipe gesture. No standalone docs page needed.
