## Why

Users frequently run multiple assistant instances — a local personal instance and a remote work server — and must reconfigure the app manually each time they switch, which is tedious and error-prone. A first-class context switcher lets users define named server profiles upfront and switch between them in seconds.

## What Changes

- Add a **Context** model: a named profile that bundles a server URL, optional display name, and optional authentication credentials.
- Add a **Context Switcher** UI: shown on app launch (and accessible from the nav rail) to let users pick or create a context.
- Persist contexts in local app storage (shared preferences / keychain for secrets).
- Replace the single hardcoded server URL with the active context's URL throughout the app.
- **BREAKING**: `ServerProfile` is superseded by `Context`; callers must migrate.

## UX

### Launch flow — no active context

```
┌─────────────────────────────────────┐
│         Context Switcher            │
│                                     │
│  ┌─────────────────────────────┐    │
│  │  💼 Work                    │    │
│  │  https://work.example.com   │    │
│  └─────────────────────────────┘    │
│  ┌─────────────────────────────┐    │
│  │  🏠 Personal           ✓   │    │  ← active
│  │  http://localhost:8080      │    │
│  └─────────────────────────────┘    │
│                                     │
│                          [ + ]      │  ← FAB: add new context
└─────────────────────────────────────┘
```

### Create / Edit context form

```
┌─────────────────────────────────────┐
│         New Context                 │
│                                     │
│  Name    [ Work                   ] │
│  URL     [ https://work.example.. ] │
│  Token   [ ••••••••••••••••       ] │  ← optional, stored in keychain
│                                     │
│              [ Cancel ]  [ Save ]   │
└─────────────────────────────────────┘
```

### macOS tray menu

```
┌──────────────────────┐
│ ● Personal           │  ← active context name
│ ─────────────────    │
│ Switch Context    ▶  │──┐
│ Open                 │  │  ┌──────────────┐
│ Quit                 │  └─▶│ ✓ Personal   │
└──────────────────────┘     │   Work       │
                             └──────────────┘
```

### Nav rail entry

```
┌────┬──────────────────────────────────┐
│    │                                  │
│ 💬 │   Chat screen                    │
│    │                                  │
│ 🔀 │  ← Contexts entry in nav rail    │
│    │                                  │
│ …  │                                  │
└────┴──────────────────────────────────┘
```

## Capabilities

### New Capabilities

- `context-management`: Create, edit, delete, and persist named server contexts (name + URL + optional auth token).
- `context-switcher-ui`: Launch-time and in-app UI that lists available contexts and activates a selected one.

### Modified Capabilities

- `macos-tray`: Surface the active context name in the tray menu and allow quick-switching from there.

## Impact

- **Flutter app** (`app/lib/`): new `features/contexts/` feature directory; `ServerProfile` provider replaced by `ActiveContextProvider`.
- **`app/packages/assistant_api/`**: no server-side API changes needed; URL injection point changes.
- **Keychain / secure storage**: credentials stored via `flutter_secure_storage`.
- **macOS tray** (`macos-tray` spec): tray menu updated to show active context.
