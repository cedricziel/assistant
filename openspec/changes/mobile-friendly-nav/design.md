## Context

The app's `NavShell` already switches between a `NavigationRail` (≥768px) and a `NavigationBar` (<768px). Both layouts currently share the same flat list of 9 destinations. On mobile, all 9 items are rendered by `NavigationBar`, which Material 3 officially supports up to 5 destinations — beyond that, items are hidden or the bar becomes unusable.

The chat screen further complicates mobile layout by embedding a drawer trigger in the AppBar leading slot, creating an inconsistent gesture layer: swipe from left for conversations, tap bottom bar for sections.

## Goals / Non-Goals

**Goals:**

- Reduce mobile bottom bar to 4 primary destinations + 1 overflow ("More")
- Place developer/observability items (Traces, Logs, Webhooks, Agents, Analytics) in the overflow sheet
- Add a visual divider in the desktop nav rail between primary and developer items
- Keep Workflows accessible on mobile via the overflow sheet (power user but not developer-only)
- Maintain deep-link URLs and GoRouter route constants unchanged

**Non-Goals:**

- Changing the desktop nav rail to collapse or hide items
- Adding search or pinning to the overflow sheet (future work)
- Reordering chat conversation management (drawer stays in chat screen)
- Backend, API, or Rust changes

## Decisions

### D1: 4 primary destinations on mobile

**Choice**: Chat, Contexts (Personas), Skills, Workflows → bottom bar. Everything else in overflow.

**Rationale**: Chat is the core loop. Contexts and Skills configure the agent — users set these up early and return often. Workflows is the automation surface and growing in importance. Traces, Logs, Webhooks, Agents, and Analytics are inspection/administration tools used far less frequently and by more technical users.

**Alternative considered**: 3 primary + More. Rejected because it hides Workflows which is a primary productivity surface.

### D2: Overflow as Modal Bottom Sheet, not a dedicated route

**Choice**: Tapping "More" shows a `showModalBottomSheet` with a grid/list of the remaining destinations.

**Rationale**: A dedicated `/more` route would break the shell route index mapping and require router changes. A bottom sheet is zero-route, immediately dismissible, and familiar on mobile (used by Gmail, Notion, etc.).

**Alternative considered**: A separate tab that renders a list screen. Rejected because it adds a nav-level route with no content of its own.

### D3: Shared destination list with a `primaryDestinations` / `overflowDestinations` split

**Choice**: In `nav_shell.dart`, replace the single `_destinations` list with two lists: `_primaryDestinations` (always shown) and `_overflowDestinations` (shown in More sheet on mobile, shown in rail with divider on desktop).

**Rationale**: Single source of truth for icon, label, route. The split is purely presentational.

### D4: Nav rail divider via `NavigationRailDestination` with a `Divider` widget inserted between groups

**Choice**: Insert a non-interactive `Divider` row between primary and overflow destinations in the rail's `destinations` list using a custom wrapper or a `SizedBox`+`Divider` combo.

**Rationale**: `NavigationRail` supports arbitrary leading/trailing widgets but not mid-list separators natively. The cleanest approach is to use `NavigationRail`'s `destinations` parameter with a sentinel `NavigationRailDestination` that has `disabled: true` and renders only a `Divider`.

**Alternative considered**: Two separate `NavigationRail` widgets stacked in a `Column`. Rejected because selection index management becomes complex with two separate widgets.

## Risks / Trade-offs

- **Index mapping fragility** → The `_selectedIndex` used by NavShell maps to GoRouter paths by position. Splitting into two lists means the index math must stay correct for both rail and bar. Mitigation: encapsulate index-to-route logic in a single helper method tested in isolation.
- **"More" sheet discoverability** → Users may not discover items moved to overflow. Mitigation: apply a notification badge or highlight to "More" when a route within overflow is active.
- **Active state in overflow** → If the user is on `/traces`, the bottom bar should indicate "More" is active. Mitigation: detect active route in overflow list and set the "More" destination as selected.

## Migration Plan

1. Update `nav_shell.dart` — no route changes needed.
2. Verify all 9 routes still reachable via keyboard/a11y (bottom sheet must be focus-traversable).
3. No server restart or data migration required.
