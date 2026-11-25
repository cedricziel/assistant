## Why

The conversation list can only open and delete conversations (`app/lib/features/chat/conversation_list.dart:111-160`). The server already supports renaming (`PATCH /api/conversations/{id}`, `crates/web-ui/src/api/conversations.rs:591`), but the app never calls it. There is no search, no pinning and no way to put a conversation away without deleting it.

Two bugs make titles worse than they look:

- **The web client turns auto-titling off.** "New Chat" and Cmd/Ctrl+N create conversations with the explicit title `'New Chat'` (`conversation_list.dart:45-47`, `conversation_list_provider.dart:204`, `nav_shell.dart:45-49`). An explicit title sets `title_locked = 1` (`crates/storage/src/conversations.rs:214`), so the title worker skips every conversation created from the sidebar.
- **The worker can overwrite a manual rename.** It reads `title_locked` (`crates/runtime/src/title_generator.rs:325`), waits seconds for the LLM, then writes unconditionally through `update_title` (`:348`). A rename that lands in that window is lost.

## What Changes

- **Titling fixes.** The client creates conversations untitled. The worker writes through a new conditional `set_generated_title` that only updates rows that are still unlocked.
- **Rename.** `PATCH` trims the title and rejects empty or over-long titles with `400`. Rename, pin and archive no longer bump `updated_at`, so they do not reorder the list.
- **Pin.** `conversations.pinned_at`, set with `PATCH {"pinned": true}`. Pinned conversations list first, in their own group. A conversation is never pinned and archived at once.
- **Archive.** `conversations.archived_at`, set with `PATCH {"archived": true}`. Archived conversations leave the main list and the snapshot. They are listed with `GET /api/conversations?archived=true`. A new user message unarchives the conversation.
- **Search.** `GET /api/conversations/search?q=` matches titles (substring, matched in Rust) and user/assistant message text (a contentful FTS5 index in `space.db`, keyed by message id so `VACUUM INTO` backups stay consistent; thinking steps are not indexed). Both use the same case and diacritic folding. It returns one hit per conversation with highlighted snippet segments.
- **Stream.** `snapshot` and `upserted` carry `pinned_at` and `archived_at`. There are no new event types. `deleted` is forwarded for archived conversations too.
- **Error envelope.** This route's 4xx responses, and the `404` of every route that loads a conversation through `owned_conversation`, become `{"error": …}`.
- **Client.** Row actions (rename, pin, archive, delete): a context menu on right-click or the overflow button, an action sheet on long-press, and swipe to pin or delete. Also a "Pinned" group, a collapsed "Archived" section, and a sidebar search field with highlighted results. The pure widgets land in `assistant_ui` with widgetbook stories and goldens.

## Capabilities

### New Capabilities

- `conversation-list-management`: rename validation, pin, archive, list ordering and grouping, stream fields, and client row actions.
- `conversation-search`: the message index, the search endpoint and the client search UI.

### Modified Capabilities

- `conversation-titling`: the worker's write is conditional on the lock, and clients create conversations untitled.
- `chat-element-library`: "Conversation list view is pure" gains selection, the pinned group, the archived slot and row-action requests. This change is therefore archived after chat-element-library.

## Impact

- **Server**: `assistant-storage` (3 migrations, `ConversationStore` methods, contract tests, `unicode-normalization` promoted from a transitive to a workspace dependency), `assistant-web-ui` (PATCH, list, search, stream, the shared `owned_conversation` 404 body), `assistant-runtime` (title worker). `openapi.json` and `app/packages/assistant_api` are regenerated. One ADR for the search index.
- **Sibling coupling**: archive's unarchive-on-user-message also hooks message-branching's `branch_message` (server PR 8 depends on message-branching PR 2). The FTS triggers fire only on `UPDATE OF content`, so siblings' new `messages` columns and backfills do not touch the index.
- **Client**: `app/lib/features/chat/conversation_list*.dart`, a new rename dialog in `shared/platform/`, and new `assistant_ui` widgets. The allowlist is unchanged.
- **Visual change**: the sidebar gains a search field, a row overflow button and group headers. The Playwright chat/sidebar baselines move in client PRs 13, 15 and 16; they are regenerated after each rebase, never merged by hand.
- **Non-goals**: scrolling to the matched message; search ranking beyond bm25; shared or cross-user search; folders/tags; bulk actions; regenerating a title on demand; searching tool output or thinking steps; CJK-aware tokenizing.
- **User-facing documentation**: needed. Add a "Conversation list" section to `docs/web-ui.md`.
