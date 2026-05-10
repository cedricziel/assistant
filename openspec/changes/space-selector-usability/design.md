## Context

`SpaceSelectorScreen` (`app/lib/features/spaces/space_selector_screen.dart`) was originally designed to handle multi-org / multi-space selection: the user picks an org, then picks a space, then lands on `/chat`. The single-org / single-space case was treated as a degenerate fast path: auto-pick both and bounce.

#687 added a fix so that _revisiting_ the screen (after the auto-pick already ran) renders the space list instead of an indefinite spinner — verified live on schorschvm (screenshot showed the highlighted "Default Space" tile + check icon).

What that fix didn't anticipate: in a 1+1 deploy, the rendered list is functionally a dead-end. The only tile already maps to your current selection. The "Change organization" link calls `SpaceSelectionNotifier.clear()`, which trips the auto-select chain to immediately re-pick. There is no explicit "go back to /chat without doing anything" path.

User report: "click the selector again, we end up in a dead end." Verified by both me (browser) and the user (native + web).

The user also explicitly does NOT want the SpaceSwitcher in the nav to be hidden or made non-interactive. The fix lives entirely in the _selector screen_.

## Goals / Non-Goals

**Goals:**

- A user landing on `/spaces` MUST always have a clear, non-loopy way back to `/chat` without committing to a selection change.
- 1-org users MUST NOT see a "Change organization" button that triggers a no-op loop.
- Multi-org / multi-space users MUST keep the existing picker UI and full functionality.
- No change to provider behavior (`OrgsNotifier`, `SpacesNotifier`, `SpaceSelectionNotifier`).
- No regression of the #687 single-space-revisit fix.

**Non-Goals:**

- Hiding or disabling the SpaceSwitcher in the nav. User explicitly wants it intact.
- Auto-bouncing `/spaces` to `/chat`. User explicitly wants the screen to render.
- Adding space-management actions (members, settings) — that's a separate scope.
- Changing route structure or the selector's place in the router.

## Decisions

### Decision 1: Add a "Done" close button in the screen header

A `IconButton(Icons.close, onPressed: () => GoRouter.of(context).go(AppRoutes.chat))` placed at top-right of the screen body, aligned with the existing "Select space" / "Select organization" heading.

Tooltip: "Close (no change)". Accessible label: "Close space selector".

The button is unconditional — present on every entry to `/spaces`, regardless of whether the user has 1, 2, or N spaces. Costs a few pixels in the header, gives every user a guaranteed exit.

**Why a close button vs. a "Cancel"/"Back" button?** "Back" is ambiguous (back to where?). "Cancel" implies a transactional flow that the selector isn't (no in-progress edit). "Close" reads as "dismiss this screen, keep current state" — matches what the user wants.

**Alternative considered:** Use the system back affordance (browser back, mobile swipe-back). Rejected: not discoverable on web, depends on history stack which can be empty if the user deep-linked.

### Decision 2: Conditional "Change organization" — hide when `orgs.length == 1`

The `_SpaceList` widget renders a "Change organization" button at the top, currently unconditionally (`space_selector_screen.dart:53-62`). When `orgsProvider`'s data shows exactly one org, hide this button.

Rendering becomes:

```dart
Consumer(builder: (ctx, ref, _) {
  final orgsAsync = ref.watch(orgsProvider);
  final hasMultipleOrgs = orgsAsync.value != null && orgsAsync.value!.length > 1;
  if (!hasMultipleOrgs) return const SizedBox.shrink();
  return TextButton.icon(/* existing widget */);
})
```

When a 2nd org is created, the button materializes on next rebuild — no further code change needed.

**Why not also `clear()`-protect the button** (e.g., make it a no-op when there's only 1 org)? Hiding is clearer: zero visual noise from a button that does nothing useful. Hide-on-condition is the cleanest UX for this case.

### Decision 3: Tapping the already-selected tile keeps its existing behavior

When the user taps the highlighted (already-selected) tile, the existing `onTap` calls `selectSpace + go(/chat)`. This is effectively "re-confirm and go to chat" — same exit as the close button, but via the tile.

Don't change this. It's harmless and matches user expectations ("I tapped my space, I'm in my space's chat").

### Decision 4: Keep the SpaceSwitcher fully interactive — no nav-shell changes

Per user direction: don't hide, don't disable, don't change behavior. The switcher remains an always-clickable entry point to `/spaces`. The fixes go _only_ on the destination screen.

This keeps the implementation surface tiny and avoids the "different switcher behavior on different screen widths" trap.

## Risks / Trade-offs

- **The new close button adds visual weight on the small selector screen.** Mitigation: keep the icon understated, use the existing color scheme, place it in the corner where similar dialogs put dismiss controls.
- **Conditional "Change organization" rendering depends on `orgsProvider` data** — if it's still loading, what do we render? Use the loading branch's existing spinner; the button shows up only after `orgsAsync.value` has data.
- **Some users might want to explicitly "leave" the only org** via the switcher (not a real workflow, but conceivably). They can still do so via the admin UI / settings — out of scope for this screen.

## Migration Plan

1. Land the change in one PR. No data migration. No feature flag.
2. Native + web both get the fix simultaneously (same Dart code).
3. **Rollback**: revert. UI returns to its post-#687 state — list still renders, just no close button or conditional org link.

## Open Questions

- Should the close button also clear an in-progress org change (e.g., user clicked "Change organization" and is now seeing `_OrgList`)? Suggest: yes — close always returns to `/chat` and resets selection only if it was mid-clear. Cleanest mental model: "close abandons whatever I was about to change and goes home with whatever I had."
