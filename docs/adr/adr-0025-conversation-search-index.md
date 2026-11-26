# ADR 0025: Conversation Search Runs on a Contentful FTS5 Index of Messages

**Status**: Accepted
**Date**: 2026-11-26
**Builds on**: [ADR-0009](adr-0009-testability-architecture.md), [ADR-0018](adr-0018-conversation-message-tree.md)

## Context

The sidebar can only filter conversations by title in the client. Users remember a topic or a detail from the body of a conversation, not its title. Finding it needs a full-text index over message content, kept consistent with the message tree ([ADR-0018](adr-0018-conversation-message-tree.md)) and with `assistant backup`, which snapshots `space.db` with `VACUUM INTO`.

The runtime already has a search index of conversations: `conversation_indexer` writes one chunk per turn into `memory_chunks`, keyed by `session://{conversation}/turn_N`. That index is the wrong base for the sidebar:

- It runs asynchronously after a turn (`spawn_index`), so fresh messages are not searchable yet.
- Its rows carry no `user_id` and no message id, so scoping would need path parsing and a hit could not name its message.
- The same table feeds the model's memory search, so tuning it for the sidebar would change model recall.

## Decision

**Add `messages_fts`, a contentful FTS5 table maintained by triggers on `messages`, and match titles in Rust.** Migration `066_messages_fts` creates it. The index is part of the space database, so a space's search follows the space through backup, restore and delete.

### Contentful, not external-content

`messages_fts` is `fts5(content, message_id UNINDEXED, conversation_id UNINDEXED, tokenize = 'unicode61 remove_diacritics 2')`. An external-content table would use `messages.rowid` as its link, but `messages.id` is `TEXT PRIMARY KEY`, so the implicit rowid has no `INTEGER PRIMARY KEY` alias and `VACUUM` may renumber it. `VACUUM INTO` would then leave an index that points at the wrong messages in every restored backup. The contentful table keeps its own rowids in shadow tables, which `VACUUM` preserves, and ties rows to messages by the `message_id` column. The price is that indexed text is stored twice.

### A side table maps messages to FTS rowids

`message_id` is an `UNINDEXED` column, so `DELETE FROM messages_fts WHERE message_id = ?` scans the whole index. On a generated 10k-message space, cascade-deleting one 200-message conversation took about 3.5 s that way. `messages_fts_map(message_id TEXT PRIMARY KEY, fts_rowid INTEGER NOT NULL) WITHOUT ROWID` records each indexed message's FTS rowid, and the triggers delete by rowid through it. The map is keyed by `message_id`, never by `messages.rowid`, and FTS rowids survive `VACUUM INTO`, so the backup round trip stays correct.

### What is indexed

A message is indexed when `role IN ('user', 'assistant')`, `trim(content) <> ''`, and `content NOT LIKE '<think>%'`.

- **Thinking text is never searched.** The client hides reasoning behind a collapsed entry, so a hit inside it would open a conversation that shows nothing matching, and snippets would leak `<think>` markup.
- Tool results can be large web pages and are truncated on save anyway. System rows are not user-visible. Empty assistant rows are tool-call stubs.
- Every branch of the message tree is indexed. Inactive branches stay searchable, and a hit on one is a hit in the same conversation.

The predicate appears three times: the insert trigger's `WHEN`, the re-insert in the update trigger, and the backfill. Migration `066` is the only place it is spelled out.

### Triggers

- `AFTER INSERT ON messages WHEN <predicate>` inserts `(content, message_id, conversation_id)` and records the new FTS rowid in `messages_fts_map`.
- `AFTER UPDATE OF content ON messages` deletes the old FTS row through the map and re-inserts (and re-maps) when the new row matches the predicate. Updates to other columns (`parent_id`, `run_id`, tool columns) do not fire it, so they do not churn the index.
- `AFTER DELETE ON messages` deletes the FTS row and map entry with no guard: deleting a row that was never indexed is a no-op. The `conversation_id` foreign-key cascade fires it for each message, so deleting a conversation leaves no index rows.

Non-destructive compaction (`conversation_summaries`) adds no message rows and edits no content, so it needs no trigger. If a later change adds a code path that rewrites `content`, the update trigger already covers it.

### Highlight sentinels

Snippets mark hits with the private-use characters U+E000 and U+E001. The insert and update triggers replace both with a space in the indexed copy, so every sentinel in a snippet was put there by `snippet()`. The stored message keeps its original text.

### Tokens may be satisfied by different messages

A conversation matches when every required token appears somewhere in it, in the title or in any indexed message, not necessarily the same one. FTS5 cannot AND across rows, so the store runs one `DISTINCT conversation_id` query per token and intersects the results in Rust, together with the title tokens. Ranking and snippets then run over the surviving conversations only. This is the query side of the feature and lands in a later PR; the index is shaped for it (`conversation_id` is stored per row).

### Titles are matched in Rust

SQLite `LIKE` folds only ASCII case and keeps diacritics, while the message index removes diacritics. `LIKE` would miss "Ärztetermin" for `ärzte`. The store therefore loads the caller's scoped conversations and matches titles with one `fold` function (lowercase, NFD, drop combining marks; `unicode-normalization`), so title and message matching agree. One user's list is a few hundred rows.

## Consequences

- Each message insert adds one FTS insert in the same transaction. The backfill runs once, when a space's migrations run at first use.
- The index costs disk: the FTS structures plus a second copy of the text. The PR that introduced it records the measured delta.
- The map adds one small row per indexed message.
- A contract test runs `VACUUM INTO` on a file-backed database, then edits and deletes in the copy, so a change that makes the index rowid-dependent fails CI.

## Alternatives considered

- **Reuse the `memory_chunks` session index.** Rejected, see Context.
- **External-content FTS5 over `messages`.** Rejected: rowid instability under `VACUUM INTO`.
- **`LIKE '%token%'` over `messages.content`.** Rejected: a full scan per query, no ranking, no snippets, no diacritic folding.
- **Index thinking and tool rows.** Rejected: hits the user cannot see, leaked markup, and large noisy documents.
