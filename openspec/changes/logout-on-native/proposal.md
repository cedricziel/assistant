```
       Before this change                       After this change

       ┌──────────────────┐                     ┌──────────────────┐
       │  Chat            │                     │  Chat            │
       │  Skills          │                     │  Skills          │
       │  Workflows       │                     │  Workflows       │
       │  Personas        │                     │  Personas        │
       │  …               │                     │  …               │
       │                  │                     │                  │
       │  Default Org     │                     │  Default Org     │
       │  Default Space   │                     │  Default Space   │
       │  [⏏  contexts ]  │   ← native only     │  [⏏  contexts ]  │
       │                  │                     │  [⏻  log out  ]  │   ← new on native
       │  Settings        │                     │  Settings        │
       └──────────────────┘                     └──────────────────┘

           web:                                    web:
       ┌──────────────────┐                     ┌──────────────────┐
       │  …               │                     │  …               │
       │  [⏻  log out  ]  │   ← only here       │  [⏻  log out  ]  │   unchanged
       │                  │                                              ↑
       └──────────────────┘                     └──────────────────┘  same code path
```

## Why

The logout button in the navigation shell (`app/lib/shared/nav_shell.dart:653`) is gated behind `if (kIsWeb)`. Native users (mac, iOS) have no logout affordance at all — the only way to drop credentials and re-authenticate is to delete the active context entirely from the `/contexts` screen. Users report being unable to test re-login flows or clear stale credential state on native. Long-standing gap; surfaced now because the post-#685 401-recovery code is not exercisable on native without a way to log out.

## What Changes

- Remove the `kIsWeb` gate around `_LogoutButton` in `nav_shell.dart`. The button shows on every platform.
- Rename `performWebLogout` → `performLogout` in `app/lib/shared/auth_actions.dart`. The implementation is already platform-agnostic (clears `spaceSelectionProvider`, then deactivates the active context). Update the only caller in `nav_shell.dart` and the existing call from `connection_provider.dart`'s `_handleAuthExpired`.
- Update the unit test (`app/test/unit/spaces/logout_resets_space_selection_test.dart`) to use the new name. No new tests required — behavior on web is unchanged; native gets the same well-tested path.

## Capabilities

### New Capabilities

(none — this is just a gate removal and a rename.)

### Modified Capabilities

- `web-401-recovery` (from #685): the spec mentions `performWebLogout` by name in commentary; update the reference to `performLogout`. No behavior change to the requirements themselves.
- `space-selector-resilience` (from #685): same — references to `performWebLogout` updated to `performLogout`.

## Impact

- **Code touched**: `app/lib/shared/nav_shell.dart` (drop `kIsWeb`), `app/lib/shared/auth_actions.dart` (rename), `app/lib/features/connection/connection_provider.dart` (call-site rename), `app/test/unit/spaces/logout_resets_space_selection_test.dart` (test name + import).
- **Tests**: none added; existing test renamed.
- **Behavior change**: native users can now log out from the nav shell. Web behavior identical.
- **Non-goals**:
  - Confirmation dialog before logout. Could add later; out of scope.
  - Different post-logout behavior on native (e.g., redirect to `/contexts` instead of `/login`). Current router redirect on `!hasContext` already lands at `/login`; the login screen on native handles legacy-token + OAuth, so the path works.
  - Cross-tab / cross-device session termination.
- **User-facing documentation needed**: No.
