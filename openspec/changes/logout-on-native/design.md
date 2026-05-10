## Context

`_LogoutButton` was added to `nav_shell.dart` as part of #685 to give web users a way to drop credentials. It's gated:

```dart
if (kIsWeb)
  _LogoutButton(
    onLogout: () async {
      await performWebLogout(ProviderScope.containerOf(context, listen: false));
      if (context.mounted) context.go(AppRoutes.login);
    },
  ),
```

The gate exists because at the time, native had a `/contexts` screen as the primary identity-management surface (switch / add / delete contexts), and the design preference was to keep the nav shell minimal on native. Web didn't have `/contexts` in its primary nav, so logout went there directly.

In practice this leaves native users with no fast path to "log out" — they must navigate to `/contexts`, find the active context, delete it. The `_handleAuthExpired` 401-interceptor path from #685 _does_ deactivate via `performWebLogout`, but a user can't manually trigger it.

The `performWebLogout` implementation is platform-agnostic:

```dart
Future<void> performWebLogout(ProviderContainer container) async {
  container.read(spaceSelectionProvider.notifier).clear();
  await container.read(activeContextProvider.notifier).deactivate();
}
```

`SpaceSelectionStorage` no-ops on native, so the `clear()` call is a cheap state reset. `deactivate()` clears the active context ID; the router redirect lands at `/login`. The login screen on native already handles both legacy-token and OAuth paths.

There's no platform-specific reason for the gate. It's just a leftover from the original review.

## Goals / Non-Goals

**Goals:**

- Native users (mac, iOS) get a discoverable logout affordance in the nav shell — same icon, same position, same behavior as on web.
- The function name reflects its platform-agnostic nature (`performLogout`, not `performWebLogout`).
- No new failure modes: existing #685 / #687 tests pass unchanged after the rename.

**Non-Goals:**

- Confirmation dialog (mac/iOS users might appreciate it; defer).
- Distinct post-logout destination on native (e.g., go to `/contexts` instead of `/login`). Current behavior is consistent and works.
- Logging out across multiple devices.
- Removing the `/contexts` screen on native (the user might still want it for context management).

## Decisions

### Decision 1: Drop the `kIsWeb` gate, period

The simplest possible change: delete `if (kIsWeb)` and let the logout button render on every platform. No conditional placement, no per-platform copy, no different post-logout flow.

### Decision 2: Rename `performWebLogout` → `performLogout`

The function was named with `Web` because of its original caller. Now that both web and native call it, the name is misleading. Rename to `performLogout`. Update:

- `app/lib/shared/auth_actions.dart` — the function definition and dartdoc.
- `app/lib/shared/nav_shell.dart` — the call site.
- `app/lib/features/connection/connection_provider.dart` — `_handleAuthExpired` calls it.
- `app/test/unit/spaces/logout_resets_space_selection_test.dart` — test file uses the function.

The two openspec specs that mention the old name (`web-401-recovery`, `space-selector-resilience`) are updated as **MODIFIED Requirements** with the new name. No behavior change to the requirements themselves.

### Decision 3: No confirmation dialog

A confirmation dialog ("Sign out?") is reasonable on native where logout is more disruptive (no SW caching). But it adds widget surface and complicates testing. Defer until someone actually requests it.

## Risks / Trade-offs

- **A user might tap the logout button accidentally on native** and lose session state. Mitigation: same risk exists on web today; no incidents reported. If it becomes an issue, add the confirm dialog as a follow-up.
- **The rename creates churn in two existing specs** (`web-401-recovery`, `space-selector-resilience`). Mitigation: the spec deltas are mechanical — `MODIFIED Requirements` blocks that copy the original text with the function name swapped. Reviewers can compare line-by-line.

## Migration Plan

1. Land in one PR. Three small code edits + one test rename.
2. Native app builds (`flutter build macos`, `flutter build ios`) include the button on next release.
3. **Rollback**: revert. Web behavior unchanged either way.

## Open Questions

- Does iOS have any platform-specific UX guideline against an in-nav logout button? (Apple HIG generally puts it under Settings.) Not blocking — we already have nav-level logout on web; consistency wins. If HIG-conformance is a concern later, move it to a Settings sub-screen on iOS only.
