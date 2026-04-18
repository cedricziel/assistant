## Context

Inline image attachments in chat messages return 401 Unauthorized. The browser console shows:

```
Failed to load resource: the server responded with a status of 401 ()
https://assistant.58lab.org/api/attachments/4dffd8b4-83a4-4fc4-9721-1261531e3848?w=300
```

The rendering code in `chat_screen.dart` `_attachmentThumbnails()` (line 694) uses `CachedNetworkImage` with:

```dart
httpHeaders: imageAuthToken != null
    ? {'Authorization': 'Bearer $imageAuthToken'}
    : const {},
```

Where `imageAuthToken` comes from `ref.read(activeProfileProvider)?.token` (line 438).

The server's auth middleware in `crates/web-ui/src/auth.rs` (line 108) correctly validates Bearer tokens. The attachment endpoint at `/api/attachments/{id}` is protected by `require_auth` (line 691 of `main.rs`).

Potential failure modes:

1. `imageAuthToken` is `null` at render time (auth state not yet loaded — race condition similar to deep-link issue)
2. `CachedNetworkImage` caches the 401 response and serves it on subsequent attempts
3. `imageBaseUrl` is null, causing `CachedNetworkImage` to resolve `/api/attachments/{id}` as a relative URL that the HTTP client can't handle

## Goals / Non-Goals

**Goals:**

- Inline image attachments load reliably with proper authentication
- Diagnose and fix the specific cause of the 401 (token availability vs. URL construction vs. cache poisoning)
- Show a retry-capable placeholder when image loading fails
- Ensure auth token is reactively available (watched, not just read once)

**Non-Goals:**

- Changing the server auth mechanism
- Adding token refresh / re-authentication logic
- Thumbnail resizing or format changes

## Decisions

### D1: Watch `activeProfileProvider` reactively for auth token

**Choice:** The `imageAuthToken` and `imageBaseUrl` are currently read via `ref.read()` inside `itemBuilder` (line 434-438). This is a one-shot read that captures the value at build time. If the provider hasn't resolved yet (e.g. during deep-link reload), the token is `null` and images fail with 401.

Change to pass these values from a `ref.watch()` higher up in the build tree, so the message list rebuilds when auth becomes available.

**Why:** `ref.read()` inside a builder callback doesn't trigger rebuilds. Images rendered before auth loads will permanently fail because the widget never rebuilds with the correct token.

### D2: Add error widget with retry

**Choice:** Replace the `errorWidget` in `CachedNetworkImage` (line 730) with a tappable retry button that clears the cache entry and reloads.

**Why:** If a 401 response is cached, the image will never load even after auth becomes available. A retry button lets the user force a fresh request. `CachedNetworkImage` supports `evictFromCache()` to clear a specific URL.

### D3: Guard against null `imageBaseUrl`

**Choice:** If `imageBaseUrl` is null, skip image rendering entirely (show a placeholder) rather than attempting to load from a relative URL.

**Why:** `CachedNetworkImage` requires an absolute URL. A relative path like `/api/attachments/{id}` will fail in the HTTP client. The `imageBaseUrl` should always be set when auth is available, but the guard prevents silent failures.

## Risks / Trade-offs

- **Cache invalidation:** Calling `evictFromCache()` on retry adds a small disk I/O cost. Acceptable for a user-initiated action.
- **Reactive rebuild:** Switching `imageAuthToken` from `ref.read()` to a watched value means the message list rebuilds when auth state changes. This is a one-time rebuild on app start — negligible performance impact.
- **Relationship to deep-link-reload-fix:** Both issues stem from the same root cause (auth state not ready at render time). Fixing the deep-link issue may partially fix this one, but the image auth fix should be independent and defensive.

## Migration Plan

No migration. Behavioural fix only.
