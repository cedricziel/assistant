# Design: conversation-list-management

## Context

What is in place today:

| Area            | Current state                                                                                                                                                  | Where                                                                                                                                           |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Record          | `ConversationRecord { id, agent_id, title, title_locked, user_id, created_at, updated_at }`                                                                    | `crates/storage/src/conversations.rs:26-40`                                                                                                     |
| Store trait     | `ConversationStore` with `update_title`, `list_conversations` (`ORDER BY updated_at DESC`), Sqlite and InMemory impls                                          | `conversations.rs:49-86`, `:289-327`, `:586`                                                                                                    |
| Rename          | `update_title` sets `title_locked = 1` **and `updated_at = now`**                                                                                              | `conversations.rs:335-358`                                                                                                                      |
| HTTP            | `PATCH /api/conversations/{id}` takes `{title: String}` with no validation. Errors are plain strings.                                                          | `crates/web-ui/src/api/conversations.rs:139-142`, `:591-646`                                                                                    |
| Summary         | `ConversationSummary { id, title, created_at, updated_at }` built by hand in 6 places                                                                          | `conversations.rs:67-73`, `:166`, `:259`, `:290`, `:368`, `:519`, `:629`                                                                        |
| Stream          | `GET /api/conversations/stream` sends a `snapshot`, then `upserted` / `deleted`. It is filtered by caller and agent, and the snapshot is re-sent on reconnect. | `conversations.rs:211-334`                                                                                                                      |
| Title worker    | check `title_locked` → LLM call → `update_title`                                                                                                               | `crates/runtime/src/title_generator.rs:325-352`                                                                                                 |
| FTS precedent   | `memory_chunks_fts`, external-content FTS5 keyed on `memory_chunks.id INTEGER PRIMARY KEY`, with sync triggers                                                 | `migrations/007_memory_chunks.sql`                                                                                                              |
| Session index   | `conversation_indexer` copies each finished turn's user/assistant text into `memory_chunks` under `session://…/turn_N`, skipping text that starts with `{`, `<` or `[` | `crates/runtime/src/conversation_indexer.rs:91-122`, spawned from `orchestrator/mod.rs:1091`, `:1344`                                     |
| Message rows    | `messages.id TEXT PRIMARY KEY`, so the implicit rowid is not stable across `VACUUM`. Thinking steps are assistant rows `<think>…</think>`; tool-call rows are assistant rows with empty content | `migrations/001_conversations.sql:10`, `orchestrator/mod.rs:1112-1116`, `orchestrator/dispatch.rs:65`                                    |
| Backup          | `assistant backup` snapshots each SQLite file with `VACUUM INTO`                                                                                               | `crates/backup/src/paths.rs:143-161`                                                                                                            |
| SQLite          | `libsqlite3-sys 0.30.1` bundles SQLite **3.46.0**, built with `-DSQLITE_ENABLE_FTS5`                                                                           | `Cargo.lock:3192`, `libsqlite3-sys-0.30.1/build.rs:129`                                                                                         |
| Migrations      | hand-listed `include_str!` table in `run_migrations`, tracked by name in `_migrations`                                                                         | `crates/storage/src/lib.rs:273-474`                                                                                                             |
| Client list     | `ConversationList` (Riverpod) with `Dismissible` swipe-to-delete and `ListTile`                                                                                | `app/lib/features/chat/conversation_list.dart:111-160`                                                                                          |
| Client model    | `ConversationListEntry { id, title, createdAt, updatedAt }`, which ignores unknown JSON keys                                                                   | `app/lib/api/models/stream_event.dart:65-90`                                                                                                    |
| Client sort     | upserts are re-sorted by `updatedAt`                                                                                                                           | `conversation_list_provider.dart:162`, `:171`                                                                                                   |
| Adaptive façade | `showAdaptiveConfirmDialog`, `showAdaptiveActionSheet`, `AdaptiveMediaContextMenu`, `AdaptiveTextField`                                                        | `app/lib/shared/platform/adaptive_dialog.dart:10`, `adaptive_action_sheet.dart:30`, `adaptive_context_menu.dart`, `adaptive_text_field.dart:13` |
| Element package | chat-element-library PR 12 lifts a pure `ConversationListView(items, header, trailingBuilder, onSelect, onDelete: Future<bool>)`, with no selection highlight   | `openspec/changes/chat-element-library/specs/chat-element-library/spec.md:452-454`, `tasks.md:227-233`                                          |

## Goals / Non-goals

**Goals:** rename that sticks, pin, archive, title and message search with highlights, and platform-appropriate row actions. Every list change reaches other tabs and devices through the existing stream.

**Non-goals:** scrolling to the matched message (it needs a `ThreadViewport` scroll-to API from chat-element-library PR 10, and is left as a follow-up); folders or tags; bulk selection; searching tool output; on-demand title regeneration; CJK tokenizing.

## Decisions

### D1. Fix the client, not the lock rule, so web conversations get auto-titled

The sidebar sends `title: 'New Chat'` (`conversation_list.dart:45-47`), and so does the provider default (`conversation_list_provider.dart:204`, reached from Cmd/Ctrl+N at `nav_shell.dart:45-49`). By spec, an explicit title locks the row (`openspec/specs/conversation-titling/spec.md`, "Explicit title at creation locks the title"). The client should send no title. The server already coerces `NULL` to `"Untitled"`.

**Alternative:** have the server treat `"New Chat"` as no title. Rejected, because it adds a magic string to a rule that is correct.

### D2. The worker writes conditionally: `set_generated_title`

Add `ConversationStore::set_generated_title(id, title) -> Result<bool>`, implemented as `UPDATE … SET title = ?, title_locked = 1 WHERE id = ? AND agent_id = ? AND title_locked = 0`. It returns `false` when no row changed, emits `Upserted` only when one did, and does not touch `updated_at`. The worker (`title_generator.rs:348`) calls it and acks on `false`. `update_title` stays the manual path: it is unconditional and locks the row.

**Alternative:** re-read `title_locked` just before writing. Rejected, because that only shrinks the race window. The `WHERE` clause closes it.

### D3. Metadata edits do not change `updated_at`

`updated_at` drives list order and the "2h ago" label (`conversation_list.dart:148`, `:166-173`). Renaming or pinning should not move a conversation to the top or make it look recent. `update_title`, `set_generated_title` and `update_metadata` (D5) therefore leave `updated_at` alone. Only `save_message` (`conversations.rs:440`) bumps it. message-branching makes the same choice for branch switches (`openspec/changes/message-branching/design.md:89`).

**Alternative:** add a separate `last_message_at` column. Rejected for now: nothing else writes `updated_at`, so splitting the column adds a migration with no benefit.

### D4. Pin and archive are nullable timestamps on `conversations`

Conversations are owned by one user (`user_id`, migration 038, and every web read is scoped through `caller_store`, `conversations.rs:40-44`). A per-conversation column is therefore per-user, and no join table is needed. `pinned_at` orders the pinned group (newest pin first). `archived_at` records when the conversation was put away. These are two migrations, `conversation_pinned_at` and `conversation_archived_at`, so the archive PRs can be dropped without touching pin.

Ordering is `pinned_at IS NULL, pinned_at DESC, updated_at DESC`. No new index is added. That `ORDER BY` cannot be served by any single index, so SQLite sorts the caller's rows after filtering them through the existing `idx_conversations_user_id` (`migrations/038_conversation_user_id.sql:5`). One user's list is a few hundred rows, so the sort is cheap.

**Archive semantics:**

- `GET /api/conversations` and the stream snapshot exclude archived rows. `GET /api/conversations?archived=true` returns only archived rows. The client loads them lazily when the user expands "Archived".
- Archiving emits `upserted` with `archived_at`. The client moves the row out of the main list. It does not emit `deleted`, because the conversation still exists, and other tabs with "Archived" expanded must be able to add it.
- **Pinned and archived are exclusive.** The invariant `pinned_at IS NULL OR archived_at IS NULL` always holds:
  - Archiving clears `pinned_at` (archive wins).
  - `{"pinned": true, "archived": true}` returns `400`.
  - `{"pinned": true}` on a conversation that is archived returns `409` with an `error` envelope ("conversation is archived"). `{"pinned": true, "archived": false}` unarchives and pins in one request.
  - Two tabs racing: whichever write commits last sees the other's state, because `update_metadata` evaluates the rule inside its `BEGIN IMMEDIATE` transaction against the stored row. Archive-then-pin yields `409`; pin-then-archive yields an unpinned archive.
  - The rule is enforced in SQL: one `UPDATE … SET archived_at = …, pinned_at = CASE WHEN <archived after patch> IS NOT NULL THEN NULL ELSE <pinned after patch> END WHERE id = ? AND NOT (<pin requested> AND <archived after patch> IS NOT NULL)`. Zero changed rows are told apart (not found vs conflict) by a read in the same transaction. `update_metadata` returns `MetadataUpdate::{Updated(record), NotFound, Invalid, PinArchived}`; the handler maps `Invalid` to `400` and `PinArchived` to `409`.
- `save_message` for a `user` role row clears `archived_at` (Gmail semantics: new activity brings it back). So does message-branching's `branch_message` for a `user` row (edit and resend), so both user write paths agree. Assistant rows do not: a turn that is still streaming when the user archives must not undo the archive when its reply is persisted (`orchestrator/mod.rs:1330`, `orchestrator/turn_control.rs:47`, `:70`). Subagent results and cancelled replies are assistant rows too.

**Why archive is cheap enough to include:** it reuses everything pin builds: the column pattern, the PATCH field, the stream field and the row-action menu. The extra server work is one migration, one filter and one query param. The extra client work is one collapsed section. It lives in its own PRs (server PRs 8 and 9, client PR 16), so it can be dropped if the product says no (see Open Questions).

**Alternative:** soft-delete via a `deleted_at` column, with a trash view. Rejected, because delete is already permanent and confirmed (`conversation_list.dart:122-138`). Trash is a different feature.

### D5. One PATCH, all fields optional

`UpdateConversationRequest { title?: String, pinned?: bool, archived?: bool }`. At least one field is required, otherwise `400`. A title is trimmed. An empty title, or one over 200 Unicode scalar values (Rust `chars().count()`), is a `400` (titles from the worker are ≤ 60). The response is the full `ConversationSummary` with `200`, per `openspec/config.yaml` PATCH rules. Fields are applied in one transaction through a new `ConversationStore::update_metadata(id, ConversationPatch) -> Result<MetadataUpdate>`, which emits one `Upserted`.

**Error envelope.** Every 4xx from this route uses `json_error` (`crates/web-ui/src/errors.rs:25`):

- The body is taken as `Result<Json<UpdateConversationRequest>, JsonRejection>` and a rejection (`{"pinned": "yes"}`, malformed JSON, wrong content type) maps to `json_error(400, …)`. Precedent: `crates/web-ui/src/api/integrations/policy.rs:59`.
- The invalid-UUID path maps to `json_error(400, "invalid conversation id")`.
- `owned_conversation` (`crates/web-ui/src/api/conversations.rs:50-62`) switches to `json_error` for its `404` and `500` **globally**, rather than being mapped locally. It is shared by `get_conversation`, `delete_conversation`, `update_conversation`, and handlers in `commands.rs:169`, `attachments.rs:92`, `audio.rs`, `turns.rs:102` and `messages.rs:135`. Their `utoipa` `404` responses gain `body = ErrorBody`, and `openapi.json` is regenerated. This matches the project convention and the `openapi-error-response-coverage` change; the Flutter client reads only status codes on these routes, and there is no backward compatibility to keep.
- `403` and `401` already use the envelope through `SpaceError` (`crates/web-ui/src/space.rs:71-84`).

**Alternative:** `POST /api/conversations/{id}/pin` and `/unpin`. Rejected: pinning is a field update, and the config reserves verb routes for non-CRUD actions.

### D6. Search: a contentful FTS5 index over messages, titles matched in Rust

**Index.** `messages_fts` is a **contentful** FTS5 table: `fts5(content, message_id UNINDEXED, conversation_id UNINDEXED, tokenize = 'unicode61 remove_diacritics 2')`. It does not depend on `messages.rowid`. `messages.id` is `TEXT PRIMARY KEY` (`migrations/001_conversations.sql:10`), so the implicit rowid has no `INTEGER PRIMARY KEY` alias, and SQLite documents that `VACUUM` may renumber such rowids. `assistant backup` snapshots `space.db` with `VACUUM INTO` (`crates/backup/src/paths.rs:143-161`), so an external-content index keyed on that rowid could point at the wrong messages after a restore. The contentful table's own rowid lives in shadow tables with `INTEGER PRIMARY KEY`, which `VACUUM` keeps. The price is that indexed text is stored twice.

**What is indexed.** A message is indexed when `role IN ('user','assistant')`, its trimmed content is non-empty, and it does not start with `<think>`. Thinking steps are stored as assistant rows `<think>…</think>` (`orchestrator/mod.rs:1112-1116`), and tool-call rows are assistant rows with empty content (`dispatch.rs:65`). Reasoning is hidden behind a collapsed entry in the client, so search does not surface it, and snippets never show `<think>` markup. Tool results can be large web pages (and are truncated anyway, `conversations.rs:99`), and system rows are not user-visible. The same predicate appears in the insert trigger's `WHEN`, the update trigger's re-insert `WHEN`, and the backfill `WHERE`. Unlike `conversation_indexer` (`conversation_indexer.rs:97-104`), content starting with `{` or `[` stays searchable: a user pasting JSON should find it again.

**Triggers.**

- `AFTER INSERT ON messages WHEN <predicate>`: insert `(content, message_id, conversation_id)`. The content is stored with U+E000 and U+E001 replaced by a space (`replace(replace(new.content, char(57344), ' '), char(57345), ' ')`), so `snippet()` can only ever see sentinels it inserted itself (see Highlighting).
- `AFTER UPDATE OF content ON messages`: `DELETE FROM messages_fts WHERE message_id = old.id`, then re-insert `WHEN <predicate on new>`. Updates to other columns (message-branching's `parent_id` backfill, tool-result-renderers' columns) do not fire it, so they do not churn the index.
- `AFTER DELETE ON messages`: `DELETE FROM messages_fts WHERE message_id = old.id`. It needs no role guard: on a contentful table, deleting a row that was never indexed is a no-op, not corruption. FK cascades from `conversations` fire it.

`DELETE … WHERE message_id = ?` scans `messages_fts`'s content table, because `UNINDEXED` columns have no index. Task 5.5 measures a cascade delete of a 200-message conversation on a 10k-message fixture. If it exceeds 200 ms, add a `messages_fts_map(message_id TEXT PRIMARY KEY, fts_rowid INTEGER NOT NULL)` side table maintained by the same triggers and delete by rowid.

**Backfill.** `INSERT INTO messages_fts(content, message_id, conversation_id) SELECT … FROM messages WHERE <predicate>`. On a contentful table `'rebuild'` would also be safe, but the explicit `SELECT` keeps the predicate in one visible place.

**Query.** User input is split on whitespace. At most 8 tokens are accepted; more is a `400`. Each token has `"` doubled and is emitted as `"token"*` (a prefix match), and the tokens are joined with implicit AND, so FTS5 syntax never reaches `MATCH` raw. Tokens of one character are dropped from the message query (`"t"*` matches nearly every message), but still apply to titles. When no token has two or more characters, only titles are searched.

**Titles are matched in Rust, not with `LIKE`.** SQLite `LIKE` folds only ASCII case and keeps diacritics, while the message index uses `unicode61 remove_diacritics 2`, so `LIKE` would miss "Ärztetermin" for `ärzte` and "Lisbôa" for `lisboa`. Instead the store loads the caller's scoped conversations (the same query as `list_conversations`, plus archived rows; one user's list is a few hundred rows) and matches titles in Rust with one `fold` function: per source `char`, lowercase it, NFD-decompose it, and drop combining marks (`unicode-normalization`, already in `Cargo.lock` at 0.1.25 as a transitive dependency; promoted to `[workspace.dependencies]`). `fold` returns the folded string plus, for every folded char, the index of the source char it came from. Matching runs on the folded string and highlights map back through that table, so a lowercase expansion such as `İ` → `i̇` cannot shift offsets. The same function decides whether a title matches and which part is highlighted, so the two cannot disagree. It approximates unicode61's folding; a contract test pins the cases that must agree (`ärzte`, `lisboa`).

**Results.** One `ConversationSearchSummary` per conversation, scoped by `agent_id` and `user_id` exactly like the list. Each carries the conversation summary fields, `title_segments`, and up to 3 `matches { message_id, role, created_at, segments }`. A title hit ranks first. Message hits rank by best `bm25` per conversation. Archived conversations are included and carry `archived_at`. The default `limit` is 20 and the max is 50.

**Bounded query cost.** The SQLite query runs in two steps:

1. A ranking CTE: `MATCH` plus `bm25(messages_fts)` only, `ROW_NUMBER() OVER (PARTITION BY conversation_id ORDER BY rank)`, joined to the caller's scoped conversations, keeping at most 3 rows for each of the top `limit` conversations. No `snippet()` here.
2. `snippet()` for the surviving rows only: `SELECT rowid, snippet(messages_fts, 0, char(57344), char(57345), '…', 12) FROM messages_fts WHERE messages_fts MATCH ?1 AND rowid IN (…)`. `snippet()` needs the `MATCH` in the same statement, so the query is repeated, restricted by rowid.

That way `snippet()` runs at most `3 × limit` times per request, however broad the query. Task 6b.4 records p95 latency for 1-, 2- and 4-character queries on the 10k fixture.

**Highlighting as segments, not offsets.** The server returns `segments: [{text, highlighted}]`. `snippet()` marks hits with the private-use sentinels U+E000/U+E001, and the server splits on them into segments. Because the insert trigger replaces both characters in the indexed copy, every sentinel in a snippet was put there by `snippet()`. The stored message itself is unchanged. Title segments come from the Rust `fold` matcher. Segments avoid the UTF-8 byte vs UTF-16 code unit mismatch between Rust and Dart offsets. The client renders them as `TextSpan`s without parsing anything.

**Cost.** For prose, an FTS5 index is roughly 30–60 % of the indexed text; the contentful copy adds about 100 % more. Each message insert adds one FTS insert in the same transaction, a sub-millisecond write. The backfill runs once, when a space's migrations run at first use. Task 5.5 measures it on a generated 10k-message space.db and records the number in the PR. If the backfill exceeds 5 s, the fallback is to backfill from a background task instead of the migration.

**Alternatives:**

- Reuse the `conversation_indexer` session index in `memory_chunks` (`conversation_indexer.rs:91-122`). Rejected: it runs asynchronously after each turn (`spawn_index`, `orchestrator/mod.rs:1091`, `:1344`), so fresh messages are not searchable yet; its rows are keyed by `session://{conversation}/turn_N` file paths with no `user_id`, so user scoping would need path parsing and a join; it stores one chunk per turn without message ids, so a hit cannot name its message; and the same table feeds the model's memory search, so tuning it for the sidebar would change model recall.
- External-content FTS5 keyed on `messages.rowid` (the `memory_chunks_fts` pattern). Rejected: `messages` has no `INTEGER PRIMARY KEY`, so `VACUUM INTO` backups can renumber rowids and desynchronise the index (see Index).
- `LIKE '%q%'` on `messages.content`. Rejected: it scans every message on every keystroke, it cannot rank, and it gives no snippets.
- The `trigram` tokenizer (SQLite ≥ 3.34). It gives substring and CJK matches, but the index is about 3× larger, and queries of fewer than 3 characters match nothing. Rejected for v1. Switching later is a single migration that drops and recreates `messages_fts`.
- `contentless_delete=1` (SQLite ≥ 3.43). It stores no content, so `snippet()` cannot run. Rejected.

ADR: this index sits beside the `memory_chunks` session index and is an architectural addition, so PR 5 adds an ADR (`docs/adr/adr-00NN-conversation-search-index.md`, the next free number when the PR is cut; 0017–0020 are claimed by sibling changes) covering `messages_fts` versus `memory_chunks`, the tokenizer, and rowid stability.

### D7. InMemory parity for search

`InMemoryConversationStore::search_conversations` folds message text with the same `fold` function, splits it on non-alphanumerics, and matches token prefixes. It applies the same indexing predicate (role, non-empty, no `<think>` prefix). Titles go through the same Rust matcher as SQLite, so title behaviour is identical. Contract tests (`crates/storage/tests/contract/conversation_store.rs`) cover the cases where both implementations agree: prefix match, AND semantics, the indexing predicate, scoping, the archived flag, `limit`, and the non-ASCII title cases. Ranking order is tested against SQLite only.

### D8. Client: the package renders, the app decides the presentation

`assistant_ui` may not branch on platform (`openspec/changes/chat-element-library/specs/chat-element-library/spec.md`, purity requirement). The package `ConversationListView` therefore gains:

- `ConversationListItem.pinnedAt` / `archivedAt`, a "Pinned" group header, and an `archivedSection` slot;
- `selectedId`, which highlights the selected row (chat-element-library leaves selection to this change);
- `onRequestActions(ConversationActionRequest request)`, fired on long-press, on secondary tap (right-click), and by a trailing overflow button that shows on hover, on focus and on the selected row (`MouseRegion` and `Focus`, which are platform-neutral). `ConversationActionRequest { item, PointerDeviceKind? kind, Offset? position, Rect? anchorRect }`:
  - `kind` is recorded from the `PointerDownEvent` that started the gesture (a `Listener`, as `app/lib/shared/edge_swipe_sidebar_toggle.dart:32` already does), so a long-press with a finger on mobile web reports `touch`, and a right-click reports `mouse`. Keyboard activation of the overflow button reports `null`.
  - `position` is the global pointer position for long-press and secondary tap, and `null` for the overflow button.
  - `anchorRect` is the overflow button's global rect from its `RenderBox`, and `null` for the row gestures.
- `onTogglePin(item)` for a start-to-end swipe. `confirmDismiss` toggles and returns `false`, so the row stays. End-to-start swipe stays delete.

The app (`conversation_row_actions.dart`) picks the presentation from `(kind, platform)`:

| `kind`                     | Platform                    | Presentation                                                                                    |
| -------------------------- | --------------------------- | ----------------------------------------------------------------------------------------------- |
| `mouse`, `trackpad`        | any                         | popup menu at `position`, or anchored below `anchorRect` for the overflow button                 |
| `touch`, `stylus`          | any (incl. mobile web)      | `showAdaptiveActionSheet`, which picks Cupertino on Apple touch and Material elsewhere (`adaptive_action_sheet.dart:30`) |
| `null` (keyboard)          | web, macOS, Windows, Linux  | popup menu anchored below `anchorRect`                                                          |
| `null` (keyboard)          | iOS, Android                | `showAdaptiveActionSheet`                                                                       |

Platform here is the façade's `isAppleTouch` plus `defaultTargetPlatform`; the input kind, not the platform, decides between menu and sheet whenever a pointer is involved. Rename opens a new façade helper `showAdaptiveTextInputDialog` (a `CupertinoAlertDialog` with a `CupertinoTextField` on Apple touch, an `AlertDialog` with a `TextField` elsewhere), prefilled and select-all. Its length limit counts Unicode scalar values (`text.runes.length`), the same unit the server uses. Flutter's `maxLength` counts grapheme clusters, so the dialog does not use it; it disables Confirm and shows the counter from the rune count.

Search: the app injects a search field into `ConversationListView.header`. It is an `AdaptiveTextField` with a clear button, and Esc clears it. While the query is non-empty, the app swaps the list for the package `ConversationSearchResultsView`, which renders `HighlightedSegments` (a pure `Text.rich` from segments). `conversationSearchProvider` debounces for 250 ms, cancels the in-flight request with a dio `CancelToken`, and exposes loading, empty, error and data states. Search results do not come from the stream, so the provider listens to it: a `deleted` for a visible result removes that row at once, and an `upserted` for a visible result re-runs the current query (debounced), so renames, pins and archive badges stay current.

**Alternative:** inline rename in the row. Rejected for v1, because it needs an injected text field inside every row and focus juggling in a lazily built list. A dialog matches the existing delete confirmation.

### D9. Stream contract

`snapshot` and `upserted` carry `pinned_at` / `archived_at`, omitted when absent per config. A pre-change client ignores them (`ConversationListEntry.fromJson`, `stream_event.dart:79-89`), so no new event type is needed. Reconnect re-sends the snapshot, so a missed `upserted` heals on its own. That is the list stream's equivalent of replay. The snapshot excludes archived rows, so two things keep archived conversations consistent:

- The stream forwards `deleted` only for ids in its `owned` set (`conversations.rs:256`, `:300`), which today is seeded from the snapshot. It is seeded instead from a new id-only query, `ConversationStore::list_owned_ids()`, that includes archived rows, so deleting an archived conversation from another client still reaches a stream opened after the archive.
- While the "Archived" section is expanded, the client refetches `?archived=true` on every new `snapshot`, so the section heals on reconnect too. An `upserted` without `archived_at` removes the row from the loaded archived list (an explicit unarchive or a new user message), and `deleted` removes it from either list. The stream handler builds summaries through the single `From<&ConversationRecord>` projection from PR 1, so the stream cannot drift from the REST shapes.

## Risks / Trade-offs

- **FK-cascade deletes and the FTS triggers.** Deleting a conversation cascades to `messages` (`001_conversations.sql:12`). Tests 5.1 and 5.3 assert that the index drops those rows. If cascade does not fire the trigger, add `AFTER DELETE ON conversations` that runs `DELETE FROM messages_fts WHERE conversation_id = old.id`.
- **Compaction loses searchable history.** `replace_history` (`conversations.rs:369`) deletes and reinserts rows, and the triggers keep the index consistent. Pre-compaction text stops being searchable. message-branching deletes `replace_history` (`openspec/changes/message-branching/tasks.md:86`, its PR 5), which fixes this for free; PR 5 here drops its `replace_history` assertion once that lands.
- **Contended `messages` table.** message-branching, tool-result-renderers, message-feedback and chat-a11y-and-code-blocks all change `Message`, `save_message` or the final-answer save sites. This change touches `messages` only through triggers (`AFTER UPDATE OF content`, so new columns and backfills do not churn the index) and the `archived_at` clear in `save_message` and `branch_message`.
- **Off-path branch hits.** Once message-branching lands, search can match a message that is not on the active path. Opening the conversation shows the active path. This is accepted for v1. A follow-up can filter hits to the active path with its recursive CTE.
- **Migration numbering.** Siblings add about 16 migrations concurrently. Each migration here uses the number pre-allocated to this change in the shared sibling migration plan; if the plan has no slot, take the next free number after `045_skill_executions.sql` when the PR is cut and record it in the plan. Register it in `run_migrations` (`crates/storage/src/lib.rs:273-474`).
- **Sentinel collision.** Stored content containing U+E000 or U+E001 would mis-split a snippet. The insert trigger replaces both with a space in the indexed copy, so `snippet()` output contains only its own markers. A contract test saves a message containing U+E000 and searches for it.
- **Sort stability.** The client sorts pinned rows by `pinnedAt`, then by `updatedAt`. A test asserts that the client order equals the server order for the same records.

- **Main titling spec is malformed.** `openspec/specs/conversation-titling/spec.md:1` starts with `## ADDED Requirements`, so archive refuses the MODIFIED delta until its headers are repaired. That is task 17.3.

## Migration Plan

Three forward-only migrations in `migrations/`: `conversation_pinned_at`, `conversation_archived_at`, and `messages_fts` (table, triggers, backfill). No data shape changes besides those. Rollback means restoring a backup. `assistant backup` snapshots `space.db` with `VACUUM INTO` (`crates/backup/src/paths.rs:161`), which may renumber `messages` rowids; the index does not depend on them, and test 5.6 runs a `VACUUM INTO` round trip followed by `integrity-check` and a search. The PATCH body change is additive. The plain-string 400/404 bodies on this route, and the 404/500 bodies of every route that uses `owned_conversation`, become `{"error": …}`.

## Open Questions

1. **Archive in or out?** It is included as separable PRs (server PRs 8 and 9, client PR 16). Drop them if archiving is not wanted.
2. **Should a new user message unarchive?** The proposal says yes (Gmail-style). The alternative keeps the conversation archived until it is unarchived explicitly.
3. **Pinning an archived conversation.** This design returns `409` (archive wins; the user unarchives first). The alternative is that pinning unarchives silently.
