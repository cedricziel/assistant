## Context

The Flutter app currently connects to a single hardcoded server URL configured at build time or via a single `ServerProfile`. Users who run both a local personal instance and a remote work server must manually reconfigure and restart the app to switch targets. This friction discourages multi-instance setups and leads to accidental message leakage between contexts.

The app already has a `features/connection/` module and a `ServerProfile` model. The new feature builds on top of these abstractions rather than replacing the transport layer.

```
┌─────────────────────────────────────────────────────────────┐
│                    CURRENT STATE                            │
│                                                             │
│  App Launch ──► Connection Screen ──► Single ServerProfile  │
│                       │                      │              │
│                       └──────────────────────┘              │
│                              hardcoded URL                  │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    TARGET STATE                             │
│                                                             │
│  App Launch ──► Context Switcher ──► Active Context         │
│                      │    │               │                 │
│               [Work] │    │ [Personal]    ▼                 │
│                      │    │        ServerProfile            │
│                      ▼    ▼        (URL + auth)             │
│              Context Store (shared_prefs + keychain)        │
└─────────────────────────────────────────────────────────────┘
```

## Goals / Non-Goals

**Goals:**

- Users can define N named contexts, each with a display name and server URL.
- Optional per-context auth token stored in secure storage.
- Context Switcher screen shown on launch when no active context exists; accessible anytime via a trailing icon button in the navigation rail (not a primary `NavigationRailDestination`).
- Active context persists across restarts.
- macOS tray menu shows the active context name and allows switching.
- Feature is fully covered by widget + unit tests.

**Non-Goals:**

- Sync or share contexts between devices.
- Per-context conversation history isolation (conversations are server-side).
- SSO or OAuth flows (out of scope for v1; auth token is manual paste).
- Android/iOS targets (macOS + web only per existing build targets).

## Decisions

### D1: Context stored as local app data, not server-side

**Decision**: Contexts live in `shared_preferences` (names/URLs) and `flutter_secure_storage` (tokens). No server endpoint.

**Rationale**: Contexts are client-side routing preferences. Requiring a server round-trip creates a chicken-and-egg problem (you need a context to reach the server). The `flutter_secure_storage` library is already approved for the macOS keychain work in #414.

**Alternatives considered**:

- Export/import via JSON file — adds complexity, deferrable to v2.

---

### D2: `Context` model replaces `ServerProfile` as the connection abstraction

**Decision**: Introduce a `Context` dataclass (`id`, `name`, `serverUrl`, `authToken?`, `createdAt`). The existing `ServerProfile` in `assistant_api` is a generated DTO and stays untouched; the new `Context` wraps it.

**Rationale**: `ServerProfile` is generated code (per CLAUDE.md — never edit manually). Wrapping it avoids breaking the generated client while giving us a richer domain model.

**Alternatives considered**:

- Extend `ServerProfile` directly — would require touching generated code on every regeneration.

---

### D3: `ActiveContextNotifier` as the single source of truth via Riverpod

**Decision**: A `StateNotifier<Context?>` provider (`activeContextProvider`) drives the entire app. The existing `serverProfileProvider` becomes a computed provider derived from `activeContextProvider`.

**Rationale**: Consistent with the existing Riverpod 2.x pattern (`AsyncNotifier`). Any widget that previously depended on `serverProfileProvider` keeps working with no changes.

**Alternatives considered**:

- `InheritedWidget` — less ergonomic, not consistent with existing patterns.

---

### D4: Context Switcher shown as a full-screen route, not a dialog

**Decision**: `/contexts` is a named GoRouter route rendered as a full screen. On launch, if `activeContextProvider` is `null`, the router redirects to `/contexts`.

**Rationale**: Full-screen avoids z-index / barrier issues on macOS desktop. Consistent with the existing `/connect` route pattern.

---

### D5: Tests — widget tests for the switcher screen, unit tests for the notifier

**Decision**:

- Unit tests for `ContextRepository` (CRUD + persistence with a fake SharedPreferences).
- Unit tests for `ActiveContextNotifier` state transitions.
- Widget tests for `ContextSwitcherScreen` (list renders, tap activates, FAB opens create dialog).
- No integration tests for keychain in CI (secure storage is mocked).

## Risks / Trade-offs

| Risk                                                                                                | Mitigation                                                                          |
| --------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `flutter_secure_storage` macOS entitlements may conflict with existing keychain setup fixed in #414 | Reuse the same entitlement group; add regression test                               |
| Router redirect loop if `activeContextProvider` emits `null` after the switcher screen loads        | Guard redirect with a `isOnContextsRoute` check in the redirect callback            |
| Existing connection screen (`/connect`) becomes redundant                                           | Keep it — it handles first-time URL entry; Context creation form reuses its widgets |

## Migration Plan

1. Add `Context` model and `ContextRepository` (no UI yet — existing app unaffected).
2. Wire `activeContextProvider`; make `serverProfileProvider` derive from it with a fallback to existing behavior.
3. Add Context Switcher screen behind the `/contexts` route (not yet in redirect logic).
4. Update router redirect to send users to `/contexts` when no active context.
5. Update macOS tray menu to show active context name.
6. Remove `ServerProfile`-as-first-class-concept from the connection screen; replace with Context form.

**Rollback**: Steps 1–3 are additive. Reverting the router redirect (step 4) restores the previous flow instantly.

## Open Questions

- Should context names be unique enforced client-side, or allow duplicates distinguished by ID? → Enforce uniqueness by display name for UX clarity (to be confirmed).
- macOS tray quick-switch: switch immediately on click, or show a submenu confirmation? → Immediate switch (consistent with standard macOS menu behavior).
