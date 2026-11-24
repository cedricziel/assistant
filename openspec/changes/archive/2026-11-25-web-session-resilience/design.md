## Context

`flutter_secure_storage` on web is AES-GCM encrypted `localStorage` with the wrap key in `IndexedDB`. The key can disappear — through "clear site data," extensions, private-mode close, browser updates, profile sync issues, even routine cache clears. When that happens, decryption throws on read. The repository handles this defensively today (`context_repository.dart:108-128`):

```dart
for (final ctx in contexts) {
  try {
    var restored = ctx;
    final token = await _secureStorage.read(key: '$_kTokenPrefix${ctx.id}');
    if (token != null) restored = restored.copyWith(authToken: token);
    final oauthJson = await _secureStorage.read(key: '$_kOAuthPrefix${ctx.id}');
    if (oauthJson != null) restored = restored.copyWith(
      oauthCredentials: OAuthCredentials.fromJsonString(oauthJson));
    result.add(restored);
  } catch (_) {
    result.add(ctx);  // <-- silent token loss
  }
}
```

The catch is too quiet: it swallows the corruption signal, lets `loadContexts()` return a context with no credentials, and `activeContextProvider` happily reports a non-null active context. The router's redirect rule (`app_router.dart:127-130`) checks `hasContext`, not `effectiveToken`, so the user lands on `/chat` with broken auth. Post-#685 the 401 interceptor catches this on the first API call and bounces them to `/login`, but with zero explanation.

Two adjacent web-only quirks compound this:

1. **`spaceSelectionProvider`** is a regular Riverpod `Notifier`; its state is in-memory only. On native, the app process tends to live across navigation. On web, every hard reload (F5, browser restart, navigation that reloads the bundle) clears it. So even when auth recovers, the selection-empty flicker reproduces every time.
2. **`app/web/sw.js`** has a `// v__APP_VERSION__` comment that's _supposed_ to be replaced at build time so Chrome's byte-diff check forces a re-install. The build emits `WARN: sw.js does not contain __APP_VERSION__ placeholder — version injection skipped` (visible in every `cargo build` log on `assistant-web-ui`). The replace step matches the wrong placeholder syntax — so the SW comment never changes, and Chrome serves cached bytecode for up to 24 hours after a deploy. Bug fix #685 doesn't actually reach users until they happen to hard-reload past the cache.

Stakeholders: anyone using the web UI; primary impact is the schorschvm operator who hits this when their browser garbage-collects the IndexedDB key.

## Goals / Non-Goals

**Goals:**

- The web app SHALL detect `flutter_secure_storage` decrypt failures during `loadContexts()` and propagate that signal to the router and the login screen — no silent token loss.
- A user landing on `/login` because of a decrypt failure SHALL see an explanatory banner. They MUST NOT be left wondering "why am I logged out again?"
- `spaceSelectionProvider` SHALL survive a hard reload on web. Native behavior unchanged.
- The Flutter PWA's `sw.js` SHALL embed the package version on every release build. A stale `__APP_VERSION__` placeholder MUST fail the build, not warn-and-skip.

**Non-Goals:**

- Switching from bearer-token auth to the existing HttpOnly `assistant_session` cookie. That's the right long-term fix on web (eliminates the secure-storage class of bug entirely), but it's a CORS-sensitive refactor across all 100+ endpoints and the entire dio config. Tracked as `web-cookie-auth` follow-up.
- Cross-tab session sync via `BroadcastChannel` so logging out in one tab signs out the others.
- Migrating non-OAuth credentials (legacy auth tokens, Siri credentials) off `flutter_secure_storage`.
- Backend changes — the OAuth refresh endpoint and cookie issuance are already in place.
- Any change to the keychain story on iOS/macOS.

## Decisions

### Decision 1: Surface decrypt failure as a typed signal, not a silent drop

`ContextRepository.loadContexts()` keeps the per-context try/catch (loading must not fail entirely if one context is corrupted), but the catch sets a new flag on the returned context: `AssistantContext.credentialsCorrupted: bool`. This is an in-memory-only flag — never serialized to JSON, never persisted — populated at load time when secure-storage reads throw.

`activeContextProvider` already returns the active context. Add a sibling provider:

```dart
final hasUsableActiveContextProvider = Provider<bool>((ref) {
  final ctx = ref.watch(activeContextProvider).value;
  return ctx != null && !ctx.credentialsCorrupted;
});
```

The router redirect uses `hasUsableActiveContextProvider` instead of the raw `hasActiveContextProvider`. Same observable behavior for healthy contexts; corrupted ones get treated as `!hasContext` so the redirect fires immediately on app boot — before any API call hits 401.

We do NOT call `deactivate()` on detection. The context metadata stays intact so re-login uses `upsertContextByUrl` to refresh credentials in-place (preserving the context ID, name, createdAt). Calling `deactivate()` would orphan the context.

**Why not just delete and re-create?** Because the user's existing `/chat` history, persisted server-side, is keyed off their user_id which the JWT will reissue from the same email — but on the _client_ side we'd lose the link between this browser's stored context and that user. Better to fix the credentials in-place.

### Decision 2: A `?reason=session-ended` query param, not a global Riverpod flag

The login screen needs to know _why_ the user is here so it can show the banner. Options:

- A) Riverpod provider that the login screen reads.
- B) A `?reason=session-ended` query parameter in the redirect URL.

Choose **(B)**. It's stateless, survives a manual refresh of `/login`, doesn't require coordinating provider lifecycle across the redirect, and is trivially testable. The router's redirect callback rewrites `'/chat'` → `'/login?reason=session-ended'` when corruption is detected; the login screen reads the param via `state.uri.queryParameters` and renders the banner.

### Decision 3: Persist `spaceSelectionProvider` on web only

Add an optional `_persistOnWeb()` method to `SpaceSelectionNotifier` that writes to `localStorage` (via `package:web` or `dart:html` — the project already uses `web: ^1.1.1`). On `build()`, hydrate from localStorage on web; return `const SpaceSelection()` everywhere else. Use a single key `assistant.spaceSelection` storing `{"orgId":"...","spaceId":"...","orgName":"...","spaceName":"..."}`.

Cleared by `performWebLogout` (already wired) — add one line: `localStorage.removeItem('assistant.spaceSelection')` on web.

**Why not SharedPreferences (cross-platform)?** Because the persistence is web-specific by design — native already has stable in-memory state across foreground/background. Adding cross-platform persistence would change behavior on native, which we explicitly don't want (and could create bugs in iOS/macOS that we'd then need to test). Localized to the platform that needs it.

**Alternatives considered:**

- IndexedDB via `flutter_secure_storage`: same crypto-key fragility we're trying to escape. Rejected.
- A new `SharedPreferences` key: works, but conflates web's transient state with native's stable state. The selection is intentionally NOT persisted on native.
- Riverpod `riverpod_annotation` with `@riverpod`-generated persistence: out of scope; project doesn't use codegen for providers.

### Decision 4: Fail the build on missing `__APP_VERSION__`

In `crates/web-ui/build.rs`, the version-injection step today does something like:

```rust
if content.contains("__APP_VERSION__") {
    let injected = content.replace("__APP_VERSION__", env!("CARGO_PKG_VERSION"));
    fs::write(&sw_path, injected)?;
} else {
    println!("cargo:warning=sw.js does not contain __APP_VERSION__ placeholder — version injection skipped");
}
```

Replace the warn-and-continue branch with a hard error: if `sw.js` exists but doesn't contain the placeholder, the build fails. This makes the contract enforceable — no more silently shipping stale service workers.

We also need to verify the placeholder _is_ in `sw.js` today; the warning history suggests it might not be (or the regex doesn't match). Verify and fix in this change.

**Why fail the build?** Service-worker caching is the difference between "we shipped a fix" and "users are still on broken code 24 hours later." A warning that nobody reads is worse than no check at all.

### Decision 5: Don't introduce a new banner widget framework

Just inline a `MaterialBanner` (or a simple `Container` with a `Theme.colorScheme.errorContainer` background) at the top of the existing `LoginScreen` body. Keep it dismissible. No need for a SnackBar (would queue against existing ones), no need for a separate banner system.

## Risks / Trade-offs

- **`credentialsCorrupted` is per-load, not persistent.** If the user dismisses the banner, refreshes, decrypt fails _again_ → banner reappears. That's actually correct behavior; if the underlying issue persists, the user keeps getting reminded.
- **localStorage write failures on web** (quota exceeded, private mode in some browsers) → catch and ignore; falls back to in-memory behavior. Quietly degrading is fine for selection state.
- **Flutter web `package:web` migration**: project uses `web: ^1.1.1` (the new package replacing `dart:html`). Need to verify localStorage access works through it without IE-edge `dart:html` imports.
- **Build-time error on missing `__APP_VERSION__`** could break dev builds if someone edits `sw.js` and removes the placeholder. Mitigation: clear comment in `sw.js` documenting the placeholder is load-bearing; the build error message points to the comment.
- **Banner UX**: easy to mis-design. Stay minimal: error-color background, icon, single sentence, dismissible.

## Migration Plan

1. Land the four code changes in one PR. Order matters in commits for reviewability:
   - Commit 1: server `build.rs` + `sw.js` placeholder fix (smallest, most independent).
   - Commit 2: `context_repository` corruption detection + `AssistantContext.credentialsCorrupted` field.
   - Commit 3: router redirect + login banner.
   - Commit 4: `spaceSelectionProvider` web persistence.
2. Standard CI checks must all pass. No backend deploy required.
3. **Rollback**: revert the PR. The `credentialsCorrupted` flag is in-memory-only; removing it doesn't leave residue. The localStorage key is harmless if abandoned.

## Open Questions

- Should the banner offer a "Why did this happen?" link to a docs page explaining browser secure-storage fragility? Recommend deferring — write the docs first, link later. No need to add a dead link.
- Should `performWebLogout` also clear `flutter_secure_storage` aggressively (`deleteAll()`) so a corrupted-but-recoverable state can't survive a logout? Probably yes, but quiet impact — keep this PR scoped and add as a follow-up commit if review brings it up.
