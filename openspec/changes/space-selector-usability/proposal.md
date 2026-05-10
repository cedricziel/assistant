## Why

#687 fixed the infinite-spinner bug where revisiting `/spaces` with an existing selection looped forever. The screen now renders the space list with the active tile highlighted — but for users with a single org and a single space (the schorschvm-style single-tenant deploy), the resulting UI is a functional dead-end. The "Change organization" button calls `clear()`, which immediately re-triggers the auto-select chain (1 org → auto-pick → 1 space → auto-pick → bounce back), creating a no-op loop. There's no "Cancel" / "Done" / close affordance, so the only way out is the browser back button. Users report this as "I clicked the switcher and now I can't get back to chat without picking again." Cross-platform — affects native and web equally.

## What Changes

- Add a clear "Done" / close affordance at the top of `SpaceSelectorScreen` that returns to `/chat` without changing the selection. Discoverable on every entry path (cold start, switcher click, deep link).
- Hide the "Change organization" button when `orgs.length == 1`. With no other orgs to switch to, the button only triggers a no-op loop. Re-appear it the moment a second org becomes available.
- Keep the SpaceSwitcher in the nav fully clickable on all platforms — explicit user preference. The selector screen needs to be useful, not the entry point hidden.
- (Bonus, low-cost) Make the active space tile's tap action explicit: a tap on the already-selected tile still calls `selectSpace + go(/chat)` so the user gets the same exit path as picking a different space.

## Capabilities

### New Capabilities

- `space-selector-usability`: How `SpaceSelectorScreen` behaves for users with a single org or single space — escape hatches, no useless buttons, no infinite loops.

### Modified Capabilities

- `space-selector-resilience` (from #687): the requirement that a single-space revisit "renders the list, not a stuck spinner" stays. This change adds the dismiss path the original spec didn't anticipate.

## Impact

- **Code touched**: `app/lib/features/spaces/space_selector_screen.dart` only.
- **Tests**: widget tests for the close button, the conditional "Change organization" rendering, and the navigation-back behavior.
- **Behavior change**: 1-org/1-space users no longer feel trapped on `/spaces`. Multi-org/multi-space users get the new close button as a bonus escape hatch.
- **Non-goals**:
  - Hiding the SpaceSwitcher chevron — explicit user pushback.
  - Auto-bouncing `/spaces` back to `/chat` on revisit — also unwanted.
  - Reworking the underlying providers (`OrgsNotifier`, `SpacesNotifier`).
  - Adding "Manage members / settings" actions to the selector screen — separate concern.
- **User-facing documentation needed**: No.
