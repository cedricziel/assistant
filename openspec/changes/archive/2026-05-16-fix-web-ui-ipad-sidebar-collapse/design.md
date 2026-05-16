## Context

`NavShell` (`app/lib/shared/nav_shell.dart`) has three render branches gated on `isAppleTouch` and `isWide` (width >= 768):

1. **Apple touch + wide** (native iPad / Mac Catalyst): renders `CupertinoSidebarCollapsible` plus a Cupertino toggle button overlaid in a `Stack` at top-leading. Already collapsible.
2. **Apple touch + compact** (iPhone): `CupertinoTabBar`. No sidebar.
3. **Material + wide** (web, Android tablet, macOS native): renders an `AnimatedContainer` sidebar with an in-sidebar toggle (`IconButton(Icons.menu_open)`, line 627). **This is the branch iPad Safari and iPad PWA hit** because `platformStyle` resolves to `material` whenever `kIsWeb` is true (`app/lib/shared/platform/platform.dart:33`).

The shared state is `sidebarCollapsedProvider` — a `Notifier<bool>` whose `build()` returns `false` and which is never persisted.

So the bug has two real causes for the iPad case:

- The Material wide branch's toggle is inside the rail itself; on a 1024×768 iPad the rail is on the left edge of the screen and the icon button is hard to spot above the destination list.
- Even when found, toggling does not survive a refresh.

## Goals / Non-Goals

**Goals**

- Sidebar collapse state survives hard reload on every platform.
- A clearly visible toggle exists on iPad landscape (both PWA / Safari Material branch and native Cupertino branch).
- Swipe-from-left-edge toggles the sidebar on touch input.
- Default state remains `expanded` for first-time users.

**Non-goals**

- Auto-collapse at narrow widths (kept manual).
- Different default per platform.
- Multi-pane / split-view navigation.

## Decisions

### D1: Persist via `SharedPreferences` under `assistant.sidebarCollapsed`

**Choice:** Replace `SidebarCollapsedNotifier` with an `AsyncNotifier<bool>` that reads `SharedPreferences` on `build()` and writes on every `toggle()`. Default to `false` if the key is missing or read fails.

**Why:** Other UI state in the app uses `SharedPreferences` (per `web-session-resilience` for `assistant.spaceSelection`). Consistent storage keeps the persistence story uniform. `SharedPreferences` also works identically on web (localStorage backed) and native.

**Alternative considered:** Use `hydrated_riverpod` or a dedicated package. Rejected — single value, not worth a new dep.

### D2: Promote toggle to the main content's top bar on Material wide

**Choice:** Render the toggle button in a thin top-leading slot **inside the main content `Expanded` area**, not only inside the sidebar. The in-sidebar toggle stays (some users will rediscover it after the first collapse). The new toggle is unconditional on the Material wide branch.

**Layout:**

```
┌─────────┬──────────────────────────────────┐
│ sidebar │ [☰] page header              … │
│  …      │ ─────────────────────────────── │
│         │ page body                       │
```

**Why:** Mirrors the Cupertino branch's overlay button and matches mainstream desktop apps (VS Code, Linear, Slack). The button stays visible whether the sidebar is collapsed or expanded.

**Alternative considered:** A floating action button. Rejected — clashes with chat composer FAB and uses too much vertical real estate.

### D3: Swipe-from-left-edge on touch input

**Choice:** Wrap the main content `Expanded` in a `GestureDetector` listening to `onHorizontalDragUpdate` at the left edge (first 20 logical pixels). A rightward drag of ≥ 40 px expands; a leftward drag collapses. Only active when the primary pointer is touch — gated on `defaultTargetPlatform` plus the `kIsWeb && touch` check.

**Why:** Matches the gesture requested in #444. Implementing it now means #444 mostly resolves alongside this fix.

**Alternative considered:** `Dismissible` widget — wrong semantics (it dismisses, not toggles). Rejected.

### D4: Promote the in-sidebar Material toggle from `IconButton` to a labelled `TextButton.icon`

**Choice:** When the sidebar is expanded, the in-sidebar toggle gets a small `"Collapse"` label next to the icon so it reads as a discoverable affordance. When collapsed (rail = 72px), the icon-only `IconButton` is retained.

**Why:** First-run discoverability. The label costs us ~24 px of width inside a 240 px sidebar.

## Risks / Trade-offs

- **Top-bar toggle conflicts with screen-specific app bars:** Several screens (`/traces`, `/logs`) already render an `AppBar` with a back button. We render the new toggle as the first item in the row alongside the existing app bar — `NavShell` already wraps each screen, so we add a slim `Row` above the child. Where children already render their own `AppBar`, we need the toggle to live _outside_ the screen's scaffold. Implementation note: render the toggle as an overlay in `Positioned` rather than nesting into screen scaffolds.
- **Swipe gesture vs. horizontal scroll:** Some screens (timeline scrubbers, table views) use horizontal scrolling. We constrain the swipe-detect zone to the left 20 px of the viewport so it doesn't intercept content scrolling.
- **`AsyncNotifier` initial value race:** While SharedPreferences is loading, the sidebar should show its previous state, not flash open then closed. We render based on `AsyncValue.value ?? false` — first-paint defaults to expanded, hydrated state replaces it within one frame.

## Migration Plan

- No data migration. First time the new code runs, the `assistant.sidebarCollapsed` key is absent → defaults to expanded — identical to today's behaviour.
- `sidebarCollapsedProvider` keeps the same public name to avoid touching every call site.
