## Why

Inline image attachments in chat messages return 401 Unauthorized errors. The browser console shows:

```
Failed to load resource: the server responded with a status of 401 ()
https://assistant.58lab.org/api/attachments/{id}?w=300
```

The `CachedNetworkImage` widget is configured to send `Authorization: Bearer $token` headers, but the token is either not available at render time or not being sent correctly.

## What Changes

- Diagnose and fix the auth token flow for inline image rendering
- Ensure `imageAuthToken` is reliably available when `CachedNetworkImage` renders (may be a race condition similar to the deep-link issue where `activeProfileProvider` hasn't loaded yet)
- Add error handling: show a placeholder with retry capability instead of a broken image icon when auth fails
- Investigate whether `CachedNetworkImage` caches 401 responses and serves them on subsequent attempts (cache poisoning)

## Capabilities

### Modified Capabilities

- `image-attachments`: Inline image thumbnails load reliably with proper authentication
- `image-error-handling`: Failed image loads show a retry-capable placeholder instead of a silent broken icon

## Impact

- `app/lib/features/chat/chat_screen.dart` — Fix `imageAuthToken` availability timing; add retry on error widget; possibly switch from `CachedNetworkImage` to a provider-aware image widget that watches auth state
- `app/lib/features/connection/connection_provider.dart` — Ensure `activeProfileProvider` is reliably available before image rendering
- May relate to deep-link-reload-fix (same auth state race condition)
- No backend changes (server auth middleware is correctly configured)
