# Tasks: conversation-list-management

This change is a stack of 18 PRs (PR 6 is split into 6a and 6b; the other numbers are kept so sibling changes' references stay valid). All 18 ship: archive is a v1 feature, not an optional tail. Each PR stays under about 500 changed lines and leaves `main` shippable. Wave placement across changes is set in `openspec/chat-ui-parity-stack.md` (PRs 1, 2, 3, 10 in wave 1; 4, 5 in wave 2; 6, 8, 12, 14 in wave 3; 7, 9, 11 in wave 4; 13, 15 in wave 5; 16 in wave 6; 17 in wave 7). Server PRs come first. Most need no client work, so they can start immediately. Every PR starts with a failing test (TDD, `.claude/skills/tdd/SKILL.md`), keeps production Rust free of `unwrap`/`expect`/`panic`, and runs `make lint && make format` before committing. PRs that touch `app/` also run `make lint-flutter && make test-flutter`.

**Migrations.** This change adds three migrations, using the numbers the stack plan pre-allocates: `052_conversation_pinned_at` (PR 3), `056_messages_fts` (PR 5) and `058_conversation_archived_at` (PR 8). Register each in both lists of the `run_migrations` table (`crates/storage/src/lib.rs:273-474`). `seed_database_through_016` (`lib.rs:562`) is a test fixture for the pre-017 upgrade path and stays unchanged.

**OpenAPI.** Every PR that changes a route runs `make dump-openapi` and `make generate-flutter-client`, and commits `openapi.json` and `app/packages/assistant_api/` in the same PR. Never hand-merge `openapi.json` or `app/packages/assistant_api/`: after a rebase, rerun both commands and commit the output.

**Playwright baselines.** PRs 13, 15 and 16 move the sidebar/chat baselines (`crates/web-ui/e2e/screenshots/`). Never merge PNG baselines by hand: after rebasing onto a sibling's baseline change, regenerate them with the e2e-testing skill and record the diff in the PR body.

**Element PRs** (11, 13, 14, 15, 16) follow the chat-element-library per-PR checklist (`openspec/changes/chat-element-library/tasks.md:7-17`): a registry entry, a widgetbook use-case, a `golden`-tagged golden, the `ELEMENTS.md` row, existing `Key`s kept, the package and app suites green, and the Playwright suite run against the baselines current on `main`.

## Parallelism

```
Server                                                     Client
PR 1 (projection) ──┬────────────────────────┐             PR 10 (untitled new chats)      ── independent
PR 2 (title race)   │                        │             PR 14 (search widgets)          ── needs CEL PR 3+4 only
PR 3 (storage pin) ─┴─ PR 4 (PATCH+pin) ─────┼──────────── PR 11 (pinned group) ──┐
                        │                    ├──────────── PR 12 (dialog+provider) ┴─ PR 13 (row actions) ─┐
PR 5 (fts index) ──┐    │                    │                                                            │
PR 6a (helpers) ───┴─ PR 6b (store search) ──┴─ PR 7 (search API) ── PR 15 (sidebar search, + PR 14)       │
PR 3 + PR 6b + message-branching PR 2 ── PR 8 (storage archive) ── PR 9 (archive API, + PR 4, PR 7) ── PR 16 (archive UI, + PR 13)
                                                                                         all ── PR 17 (docs, archive)
```

- **Start immediately, in parallel:** PRs 1, 2, 3, 5, 6a and 10. PR 14 starts as soon as chat-element-library PRs 3 and 4 merge. PR 1 belongs in the first wave: seven sibling changes edit `get_conversation` and `ConversationSummary` in `crates/web-ui/src/api/conversations.rs` after it.
- PR 4 needs PRs 1 and 3. PR 6b needs PRs 5 and 6a. PR 7 needs PRs 1 and 6b. PR 8 needs PRs 3 and 6b and message-branching PR 2 (`branch_message`). PR 9 needs PRs 4, 7 and 8.
- PRs 11 and 12 run in parallel once PR 4 merges (PR 11 also needs chat-element-library PR 12).
- PR 13 needs PRs 11 and 12. PR 15 needs PRs 7, 14 and chat-element-library PR 12. PR 16 needs PRs 9 and 13.
- PR 5 needs message-branching's non-destructive compaction PR (MB PR 3 in the stack plan), which removes `replace_history`; the stack plan merges it first.
- PRs 8, 9 and 16 are the archive feature. They are separate only to stay small; PR 17 depends on all of them.

---

## PR 1 — refactor(web-ui): single ConversationSummary projection

Layer: server
Depends on: none
Parallel with: PRs 2, 3, 5, 6a, 10

- [x] 1.1 Test first, in `crates/web-ui/src/api/conversations.rs` tests: a unit test that `ConversationSummary::from(&record)` maps a `NULL` title to `"Untitled"` and copies `id`, `created_at` and `updated_at`. Confirm RED (no `From` impl).
- [x] 1.2 Add `impl From<&ConversationRecord> for ConversationSummary`. Replace the six hand-built projections (`conversations.rs:166`, `:259`, `:290`, `:368`, `:629`, and the detail header at `:519` via a shared helper). Confirm GREEN. No wire change, so the existing handler and stream tests stay green unchanged.

## PR 2 — fix(runtime): auto-title only writes unlocked conversations

Layer: server
Depends on: none
Parallel with: PRs 1, 3, 5, 6a, 10

- [x] 2.1 Test first, in `crates/storage/tests/contract/conversation_store.rs` (both implementations): `set_generated_title` on an unlocked row returns `true`, locks it, and leaves `updated_at` unchanged. On a row locked by `update_title` it returns `false` and the title is unchanged. A test with a broadcaster asserts that no `Upserted` is emitted on `false`. Confirm RED.
- [x] 2.2 Add `ConversationStore::set_generated_title(&self, id, title) -> Result<bool>`: `UPDATE … WHERE id = ? AND agent_id = ? AND title_locked = 0` in Sqlite, and the same guard in `InMemoryConversationStore`. Confirm GREEN.
- [x] 2.3 Test first, in `crates/runtime/src/title_generator.rs` tests: an LLM stub whose `chat` call renames the conversation through `update_title` before returning. After `run_one`, the stored title is the manual one and the bus message is acked. Confirm RED.
- [x] 2.4 Switch the worker (`title_generator.rs:348`) to `set_generated_title`. Log at `debug!` and ack on `false`. Confirm GREEN.

## PR 3 — feat(storage): conversation pinning and metadata patch

Layer: server
Depends on: none
Parallel with: PRs 1, 2, 5, 6a, 10

- [x] 3.1 Test first, contract tests: `update_metadata(id, ConversationPatch { title: Some(..), pinned: Some(true), .. })` returns `MetadataUpdate::Updated(record)` with `pinned_at` set, `title_locked = true`, and `updated_at` unchanged. Re-pinning keeps `pinned_at`. `pinned: Some(false)` clears it. An unknown id returns `MetadataUpdate::NotFound`. A title patch on an already locked row overwrites it. `list_conversations` orders pinned first (newest pin first), then by `updated_at`. `update_title` no longer changes `updated_at`. Confirm RED.
- [x] 3.2 Add migration `052_conversation_pinned_at.sql` (`ALTER TABLE conversations ADD COLUMN pinned_at DATETIME`). Add `pinned_at: Option<DateTime<Utc>>` to `ConversationRecord`, plus every `SELECT` and row mapper in `conversations.rs`.
- [x] 3.3 Add `ConversationPatch { title: Option<String>, pinned: Option<bool> }`, the `MetadataUpdate` enum, and `ConversationStore::update_metadata` (`BEGIN IMMEDIATE`, one `UPDATE`, one `Upserted`) for Sqlite and InMemory. Change `list_conversations` to `ORDER BY pinned_at IS NULL, pinned_at DESC, updated_at DESC`. Drop `updated_at = ?` from `update_title` (`conversations.rs:339`). Confirm GREEN. Fix the existing tests that asserted bumped timestamps.

## PR 4 — feat(web-ui): partial conversation PATCH with pin and validation

Layer: server
Depends on: PR 1, PR 3
Parallel with: PRs 5, 6a, 6b, 10

- [x] 4.1 Test first, handler tests in `conversations.rs`: the title is trimmed (`"  Trip plan  "` → `"Trip plan"`). A blank title, a title of 201 `é` (while 200 `é` is accepted), `{}`, a wrong-typed body `{"pinned": "yes"}`, malformed JSON and an invalid UUID each return `400` with an `error` key. `{"pinned": true}` returns `pinned_at`. `{"title","pinned"}` together produce one `upserted` on the stream. Another user's conversation returns `404` with an `error` key. A member with read-only access gets `403` with an `error` key. The rename does not reorder `GET /api/conversations`. `pinned_at` is absent from JSON when unset. Confirm RED.
- [x] 4.2 Test first: `owned_conversation`'s `404` is a JSON envelope on `GET /api/conversations/{id}` and on one route from each of `commands.rs`, `attachments.rs`, `audio.rs`, `turns.rs` and `messages.rs`. Confirm RED. Switch `owned_conversation` (`conversations.rs:50-62`) to `json_error` for its `404` and `500`, and add `body = ErrorBody` to the `404` response of each of those routes' `utoipa` annotations. Confirm GREEN.
- [x] 4.3 Make `UpdateConversationRequest` fields optional (`title`, `pinned`). Take the body as `Result<Json<UpdateConversationRequest>, JsonRejection>` and map a rejection to `json_error(400, …)` (precedent: `crates/web-ui/src/api/integrations/policy.rs:59`). Map the invalid UUID to `json_error`. Count the title length with `chars().count()`. Call `update_metadata`. Add `pinned_at` (`skip_serializing_if = "Option::is_none"`, `format: date-time`) to `ConversationSummary`. Update the `utoipa` annotations (400, 403 and 404 use `ErrorBody`). Confirm GREEN.
- [x] 4.4 Test first: the stream `snapshot` and `upserted` carry `pinned_at` (extend `stream_conversations_forwards_upserted_delta`). Confirm GREEN through the PR 1 projection.
- [x] 4.5 `make dump-openapi && make generate-flutter-client`. Commit `openapi.json` and `app/packages/assistant_api/`. Confirm `flutter analyze` is clean (the generated builder's `title` becomes nullable, so fix any compile errors at existing call sites).

## PR 5 — feat(storage): full-text index over user and assistant messages

Layer: server
Depends on: message-branching non-destructive compaction PR (MB PR 3 in the stack plan)
Parallel with: PRs 1–4, 6a, 10

- [x] 5.1 Test first, in `crates/storage/src/conversations.rs` tests using `StorageLayer::new_in_memory()`: after saving user, assistant, tool, thinking (`<think>Lisbon…</think>`) and empty assistant tool-call messages, `SELECT message_id FROM messages_fts WHERE messages_fts MATCH 'lisbon'` returns only the user and assistant prose rows. Editing a message's `content` re-indexes it. An `UPDATE` of another column on every row leaves the index unchanged. Deleting the conversation leaves no FTS rows. `INSERT INTO messages_fts(messages_fts) VALUES('integrity-check')` succeeds after each step. A saved message containing U+E000 is stored unchanged, while its indexed copy has a space in its place. Confirm RED (no table).
- [x] 5.2 Add migration `056_messages_fts.sql`: `CREATE VIRTUAL TABLE messages_fts USING fts5(content, message_id UNINDEXED, conversation_id UNINDEXED, tokenize='unicode61 remove_diacritics 2')`, the `AFTER INSERT` trigger and the `AFTER UPDATE OF content` trigger (delete by `message_id`, then re-insert) guarded by the predicate (`role IN ('user','assistant') AND trim(content) <> '' AND content NOT LIKE '<think>%'`), the unguarded `AFTER DELETE` trigger (`DELETE FROM messages_fts WHERE message_id = old.id`), U+E000/U+E001 replaced by a space in the indexed copy, and a backfill `INSERT … SELECT` with the same predicate (design.md D6). Confirm GREEN.
- [x] 5.3 If 5.1's cascade-delete assertion fails, add `AFTER DELETE ON conversations` cleanup by `conversation_id` (design.md Risks). Keep the test either way.
- [x] 5.4 Backfill test: insert rows directly with SQL into a pool migrated up to the previous migration (including a thinking row and a tool row), run the new migration, and assert the predicate. Confirm GREEN.
- [x] 5.5 Measure on a generated 10k-message space.db (an ignored `#[tokio::test]` or a small bench): the migration time, the database size delta, and a cascade delete of a 200-message conversation. Record all three in the PR body. If the backfill takes more than 5 s, stop and raise it (design.md D6 fallback). If the cascade delete takes more than 200 ms, add the `messages_fts_map` side table (design.md D6).
- [x] 5.6 Test first, backup round trip, on a file-backed pool in a temp dir: save messages, delete some to leave rowid gaps, run `VACUUM INTO` (the statement `assistant backup` uses, `crates/backup/src/paths.rs:161`), open the copy, delete one message and edit another, then assert `integrity-check` succeeds and an FTS query returns the expected `message_id`s. Confirm GREEN against 5.2 (it would fail against a rowid-keyed external-content table).
- [x] 5.7 Add `docs/adr/adr-00NN-conversation-search-index.md` (the next free number when the PR is cut; 0017–0020 are claimed by sibling changes; the stack plan assigns none to this change): `messages_fts` versus the `memory_chunks` session index (`crates/runtime/src/conversation_indexer.rs`), the contentful table and rowid stability under `VACUUM INTO`, the tokenizer, the indexing predicate (thinking text excluded), the cross-message token rule, and the Rust title matcher.

## PR 6a — feat(storage): search query, folding and highlight segment helpers

Layer: server
Depends on: none
Parallel with: PRs 1–5, 10

- [x] 6a.1 Test first, unit tests for `fts_query(tokens)`: whitespace tokens become `"tok"*`, `"` is doubled, `OR`/`NEAR`/`*`/`(` stay literal, one-character tokens are dropped, blank or all-short input gives `None`. `tokenize_query(input)` rejects more than 8 tokens. Confirm RED.
- [x] 6a.2 Test first, unit tests for `fold(text) -> (String, Vec<usize>)`: lowercases, strips diacritics after NFD, and maps every folded char back to its source char index, including `İ`, `Ä` and `ô`. `title_segments(title, tokens)` matches folded substrings and highlights the original text (`Ärztetermin` for `ärzte`, `Lisbôa` for `lisboa`), highlighting only the tokens the title contains, returning the set of satisfied tokens, and `None` when it satisfies none. `segments_from_snippet` splits on the U+E000/U+E001 markers. Confirm RED.
- [x] 6a.3 Test first, unit tests for `satisfied_conversations(required_tokens, title_tokens_by_conversation, message_tokens_by_conversation)`: a conversation whose token A comes from message 1 and token B from message 2 is returned; one with only A is not; a token satisfied by the title plus another by a message is returned; one-character tokens are not required when a longer token exists, and are required (title only) when none does; an empty required set returns nothing. Confirm RED.
- [x] 6a.4 Implement the helpers in a new `crates/storage/src/conversation_search.rs`, with `SearchSegment { text, highlighted }`, including `satisfied_conversations`. Add `unicode-normalization` (0.1.25, already in `Cargo.lock`) to `[workspace.dependencies]` and to `assistant-storage` (rust-dependencies skill). Confirm GREEN.

## PR 6b — feat(storage): conversation search on ConversationStore

Layer: server
Depends on: PR 5, PR 6a
Parallel with: PRs 3, 4, 10

- [x] 6b.1 Test first, contract tests: a prefix match, tokens satisfied by different messages of one conversation (a conversation with only one of the tokens is excluded), a title token plus a message token, thinking rows never matching, a title hit ranked before a message-only hit, non-ASCII title matches (`ärzte`, `lisboa`), a one-character query returning title hits only, a one-character token ignored beside a longer one, at most one hit per conversation, at most 3 matches, `limit` respected, tool rows never matched, scoping by `agent_id` and `user_id`, and a saved message containing U+E000 yielding exactly one highlighted segment and no sentinel in any segment. Confirm RED.
- [x] 6b.2 Add `ConversationSearchHit { record, title_segments, matches: Vec<MessageHit> }`, `MessageHit { message_id, role, created_at, segments }` and `ConversationStore::search_conversations(query, limit) -> Result<Vec<ConversationSearchHit>>` to the trait. Implement it in Sqlite (design.md D6): title token sets through the PR 6a matcher over the caller's scoped conversations; phase 1, one `SELECT DISTINCT conversation_id … MATCH '"tok"*'` per required token, combined with `satisfied_conversations`; phase 2, a ranking CTE over the candidates with the tokens OR-ed (`MATCH`, `bm25`, `ROW_NUMBER() OVER (PARTITION BY conversation_id ORDER BY rank)`, top 3 for the top `limit` conversations, no `snippet()`); then `snippet(messages_fts, 0, char(57344), char(57345), '…', 12)` for the surviving rowids only. Confirm the Sqlite half of 6b.1 GREEN.
- [x] 6b.3 Implement `InMemoryConversationStore::search_conversations` per design.md D7. Confirm the full contract suite GREEN.
- [x] 6b.4 Test first: an ignored test on the 10k fixture records p95 latency over 50 runs for 1-, 2- and 4-character queries and for an 8-token query of two-character prefixes. Record the numbers in the PR body. If the 2-character p95 exceeds 100 ms, or the 8-token p95 exceeds 150 ms, add `prefix='2 3'` to `messages_fts` (a change to PR 5's migration, so land it there if PR 5 has not merged yet, otherwise as a new migration in this PR).

## PR 7 — feat(web-ui): GET /api/conversations/search

Layer: server
Depends on: PR 1, PR 6b
Parallel with: PRs 8, 11, 12

- [x] 7.1 Test first, handler tests: `q=lisb` returns a bare array with `matches[0].segments` as specified. A blank `q`, a `q` over 200 Unicode scalar values and a `q` with 9 tokens return `400` `{"error"}`. `limit=500` is capped at 50. User B cannot see user A's hits. A request without credentials returns `401` `{"error"}`. `q="foo" OR bar*` returns `200`. The route does not shadow `GET /api/conversations/{id}`. Confirm RED.
- [x] 7.2 Add `search_conversations` (operationId `search_conversations`, tag `conversations`, `Access::Read`) with `SearchConversationsQuery { q, limit? }`, the `ConversationSearchSummary` schema (summary fields + `title_segments` + `matches`), `MessageSearchMatch` and `SearchSegment`. Register `/conversations/search` next to `/conversations/stream` (`crates/web-ui/src/api/mod.rs:244`). Confirm GREEN.
- [x] 7.3 `make dump-openapi && make generate-flutter-client`. Commit both.

## PR 8 — feat(storage): archive conversations

Layer: server
Depends on: PR 3, PR 6b, message-branching PR 2
Parallel with: PRs 4, 7

- [x] 8.1 Test first, contract tests: `update_metadata(.. archived: Some(true))` sets `archived_at` and clears `pinned_at`, and `Some(false)` clears it. `pinned: Some(true)` with `archived: Some(true)` returns `MetadataUpdate::Invalid` and changes nothing. `pinned: Some(true)` on an archived row (with `archived` absent or `Some(false)`) returns `Updated` with `pinned_at` set and `archived_at` cleared. `pinned: Some(false)` alone leaves `archived_at` unchanged. Two interleaved patches (archive, then pin) end pinned and unarchived, and the reverse order ends archived and unpinned, never both. `list_conversations` excludes archived rows. A new `list_archived_conversations` returns only archived rows, newest first. A new `list_owned_ids()` returns the caller's ids including archived ones. `save_message` with role `user` clears `archived_at`, and role `assistant` does not. `branch_message` with a `user` row clears `archived_at`. `updated_at` is unchanged by archive and unarchive. `search_conversations` still returns archived conversations, with `archived_at` set. Confirm RED.
- [x] 8.2 Add migration `058_conversation_archived_at.sql` (the column only; no new index, design.md D4). Extend `ConversationRecord`, `ConversationPatch`, `MetadataUpdate` (`Updated`, `NotFound`, `Invalid`) and the mappers.
- [x] 8.3 Enforce the pin/archive invariant inside the single `UPDATE` of `update_metadata` (design.md D4): pin clears `archived_at`, archive clears `pinned_at`, and `Invalid` is returned before the statement. `NotFound` is zero changed rows. Clear `archived_at` in `save_message` and in `branch_message` for `user` rows. Implement for Sqlite and InMemory. Confirm GREEN.

## PR 9 — feat(web-ui): archive via PATCH and archived listing

Layer: server
Depends on: PR 4, PR 7, PR 8
Parallel with: PRs 11–15

- [x] 9.1 Test first, handler tests: `{"archived": true}` returns `archived_at` and no `pinned_at`. `{"pinned": true, "archived": true}` returns `400` with an `error` key. `{"pinned": true}` on an archived conversation returns `200` with `pinned_at` and no `archived_at`, lists it in the main list's pinned group, and emits exactly one `upserted`. `GET /api/conversations` excludes the conversation, and `?archived=true` includes it. The stream `snapshot` excludes it. Archiving emits `upserted` (not `deleted`) carrying `archived_at`. A later `POST …/messages` unarchives it. `ConversationSearchSummary` carries `archived_at`. Confirm RED.
- [x] 9.2 Test first, stream handler test: archive A, open a new stream (its snapshot lacks A), delete A from another client, and assert the new stream receives `deleted` for A. Confirm RED.
- [x] 9.3 Add `archived` to `UpdateConversationRequest` and map `MetadataUpdate::Invalid` to `json_error` `400`. Add `archived_at` to `ConversationSummary` and an `ListConversationsQuery { archived: Option<bool> }` param on `list_conversations`. Filter the snapshot. Seed the stream's `owned` set (`conversations.rs:256`) from `list_owned_ids()` instead of the snapshot. Confirm GREEN.
- [x] 9.4 `make dump-openapi && make generate-flutter-client`. Commit both.

---

## PR 10 — fix(app): new chats start untitled so auto-titling runs

Layer: client
Depends on: none
Parallel with: everything

- [x] 10.1 Test first, in `app/test/unit/chat/conversation_list_provider_test.dart`: `createConversation()` sends a `CreateConversationRequest` with `title == null`. In `app/test/widget/new_chat_shortcut_test.dart` and a `ConversationList` widget test, the New Chat button and Cmd/Ctrl+N call `createConversation` with no title. Confirm RED (currently `'New Chat'`, `conversation_list_provider.dart:204`, `conversation_list.dart:45-47`).
- [x] 10.2 Drop the default and the explicit `'New Chat'` title. Keep the button label. Confirm GREEN.

## PR 11 — feat(app): pinned group and selection in the conversation list

Layer: client
Depends on: PR 4, chat-element-library PR 12
Parallel with: PR 12, PR 14

- [x] 11.1 Test first, in `stream_event_test` / `conversation_list_provider_test.dart`: `ConversationListEntry.fromJson` parses `pinned_at` (and its absence). An `upserted` with `pinned_at` moves the entry to the top. The provider order equals the server order (pinned by `pinnedAt` desc, then `updatedAt` desc) for a shared fixture. Cover the loading, success and error states for the new sort. Confirm RED.
- [x] 11.2 Add `pinnedAt` to `ConversationListEntry` (`stream_event.dart:65-90`). Replace the two `updatedAt` sorts (`conversation_list_provider.dart:162`, `:171`) with one `compareConversations` function. Confirm GREEN.
- [x] 11.3 Test first, in package `test/conversation_list_view_test.dart`: with pinned items, a "Pinned" header precedes them, and with none, there is no header. The row whose id equals `selectedId` is highlighted. Confirm RED. Add `ConversationListItem.pinnedAt`, `selectedId` and the group header to `ConversationListView` (specs/chat-element-library delta), and map the fields in the app adapter. Confirm GREEN.
- [x] 11.4 Add widgetbook use-cases "pinned + recent" and "selected", and goldens.

## PR 12 — feat(app): rename dialog and list metadata actions

Layer: client
Depends on: PR 4
Parallel with: PR 11, PR 14

- [x] 12.1 Test first, in `app/test/widget/platform/adaptive_text_input_dialog_test.dart` (next to the other façade tests): `showAdaptiveTextInputDialog` renders a `CupertinoAlertDialog` on iOS and an `AlertDialog` on web/macOS, is prefilled with the text selected, keeps Confirm disabled while blank or over `maxLength` counted in runes (200 single-scalar emoji enabled, 201 disabled), confirms on Enter, and returns `null` on Esc or cancel. Confirm RED.
- [x] 12.2 Implement it in `app/lib/shared/platform/adaptive_dialog.dart` and export it from `widgets.dart`. Confirm GREEN.
- [x] 12.3 Test first, provider unit tests: `renameConversation(id, title)` and `setPinned(id, bool)` call `PATCH` with the right body. Each exposes success and error (the error is returned to the caller and the list state is unchanged). The list is updated by the stream, not optimistically. Confirm RED, then implement in `conversation_list_provider.dart`. Confirm GREEN.

## PR 13 — feat(app): conversation row actions

Layer: client
Depends on: PR 11, PR 12
Parallel with: PR 14, PR 15

- [x] 13.1 Test first, package `conversation_list_view_test.dart`: a touch long-press fires `onRequestActions` with `kind == touch` and the press position; a mouse secondary tap fires it with `kind == mouse`. The overflow button shows on hover, on focus and on the selected row; a click fires with a `null` position and the button's `anchorRect`, and keyboard activation fires with a `null` kind. A start-to-end swipe fires no callback and the row stays (the `Dismissible` direction is end-to-start only). End-to-start swipe still goes through `onDelete`. Confirm RED.
- [x] 13.2 Add `ConversationActionRequest`, `onRequestActions` and the overflow button (with a `Semantics` label "Conversation actions") to `ConversationListView`. Restrict the `Dismissible` to `DismissDirection.endToStart`. Record the pointer kind from `Listener.onPointerDown`, as `app/lib/shared/edge_swipe_sidebar_toggle.dart:32` does. Confirm GREEN. Add use-cases for hover and selected, and goldens.
- [x] 13.3 Test first, app widget test `app/test/widget/features/chat/conversation_list_test.dart`: on web with a mouse, right-click opens a popup menu at the pointer with Rename, Pin/Unpin and Delete (and no swipe offers Pin), and an overflow click opens it anchored to the button. On a non-Apple platform, a touch long-press opens an action sheet, not a menu (the mobile-web case). On iOS, long-press opens a Cupertino action sheet. Keyboard activation of the overflow button opens an anchored menu on macOS and a sheet on Android. Rename → dialog → `renameConversation`. A failed PATCH shows an adaptive snack bar. Confirm RED.
- [x] 13.4 Extract `conversation_row_actions.dart` from `conversation_list.dart`, implementing the `(kind, platform)` table in design.md D8 and the action handlers. Keep `conversation_list.dart` a thin composition. Confirm GREEN.
- [x] 13.5 Regenerate the Playwright sidebar/chat baselines (e2e-testing skill), because the selected row now shows the overflow button. Document the diff in the PR.

## PR 14 — feat(app): highlighted search result elements

Layer: client
Depends on: chat-element-library PR 3, PR 4
Parallel with: PRs 1–13

- [x] 14.1 Test first, package tests: `HighlightedSegments(segments)` builds one `TextSpan` per segment, and highlighted spans are bold with `colorScheme.primary`. `ConversationSearchResultsView(results, onSelect)` renders the title segments and up to 3 snippets per row, shows an archived badge when `archivedAt` is set, and fires `onSelect(id)` on tap. Confirm RED.
- [x] 14.2 Add the package models `SearchSegmentItem` and `ConversationSearchItem`, and the two widgets. They are pure, so the allowlist is unchanged. Confirm GREEN.
- [x] 14.3 Add widgetbook use-cases (title hit, message hits, empty, long snippet) and goldens (light and dark).

## PR 15 — feat(app): search conversations from the sidebar

Layer: client
Depends on: PR 7, PR 14, chat-element-library PR 12
Parallel with: PR 13, PR 16

- [x] 15.1 Test first, unit tests for `conversationSearchProvider` (with `fake_async`): 250 ms debounce (one request for "lis"+"b"), cancellation of the superseded request (its result is discarded), the empty query idle state, and the loading, empty, error and data states. A `deleted` stream event for a visible result removes it without a request. An `upserted` for a visible result re-runs the query once (debounced). Events for conversations not in the results are ignored. Confirm RED.
- [x] 15.2 Implement `conversation_search_provider.dart`, mapping `ConversationSearchSummary` to package items and listening to the conversation stream. Confirm GREEN.
- [x] 15.3 Test first, widget test: typing shows `ConversationSearchResultsView`. "No conversations match" shows for `[]`. The error state offers Retry. Tapping a result navigates to `/chat/{id}`, clears the query, and calls `onConversationSelected` (to close the drawer). Esc and the clear button restore the list. Confirm RED.
- [x] 15.4 Add `conversation_search_field.dart` (an `AdaptiveTextField` with a clear button and an Esc shortcut) to the `ConversationListView.header` slot, and swap the body while the query is non-empty. Confirm GREEN.
- [x] 15.5 Regenerate the Playwright baselines (the sidebar gains a search field). Add one e2e test that searches a seeded conversation and screenshots the results. Document the diff.

## PR 16 — feat(app): archive action and Archived section

Layer: client
Depends on: PR 9, PR 13
Parallel with: PR 15

- [x] 16.1 Test first, provider tests: `setArchived(id, bool)` sends PATCH. `setPinned(id, true)` on an archived entry sends `{"pinned": true}` and the resulting `upserted` (pinned, no `archived_at`) moves it from the archived list into the Pinned group. An `upserted` with `archived_at` removes the entry from the main list and adds it to the loaded archived list. An `upserted` without `archived_at` for an entry in the loaded archived list removes it there and inserts it into the main list in server order. A `deleted` removes the entry from the archived list. While the section is expanded, every new `snapshot` triggers a refetch of `?archived=true` that replaces the archived list; while collapsed, it does not. `loadArchived()` calls `GET /api/conversations?archived=true` only on demand, and covers loading, success and error. Confirm RED, then implement. Confirm GREEN.
- [x] 16.2 Test first, package and widget tests: the `archivedSection` slot renders a collapsed "Archived" header. Expanding it triggers `loadArchived`. The row actions show Archive/Unarchive, and an archived row still offers Pin (which unarchives and pins). Confirm RED, then implement. Confirm GREEN. Add a use-case and a golden.
- [x] 16.3 Regenerate the Playwright baselines (the "Archived" header). Document the diff.

## PR 17 — docs(web-ui): document conversation list management and search

Layer: both
Depends on: PRs 1–16

- [x] 17.1 Add a "Conversation list" section to `docs/web-ui.md`: rename, pin, archive (pinned and archived are exclusive: pinning an archived conversation unarchives it, and a new message unarchives it), search syntax (prefix tokens, every term must appear somewhere in the conversation but not necessarily in the same message, titles by substring, one-character tokens match titles only, at most 8 terms, tool output and thinking not searched), per-platform gestures (right-click, overflow button, long-press; pin is in the menu only and swipe deletes), and that metadata edits do not reorder the list.
- [x] 17.2 Add the new endpoints and the PATCH fields to the REST table in `docs/web-ui.md` (`## REST API`).
- [x] 17.3 Repair `openspec/specs/conversation-titling/spec.md` so this change can be archived: it opens with a delta header `## ADDED Requirements` instead of `## Purpose` + `## Requirements`, so OpenSpec cannot apply this change's MODIFIED delta (reported as INFO by `openspec validate --strict`). Change only the headers; the requirement text stays as it is.
- [x] 17.4 Run `make lint && make format && make test && make lint-flutter && make test-flutter`. Run `openspec validate conversation-list-management --strict`.
- [x] 17.5 Archive. This change carries a MODIFIED delta for `chat-element-library` ("Conversation list view is pure"), which OpenSpec can apply only once that capability exists in `openspec/specs/`. Wait until chat-element-library has been archived, then run `openspec archive conversation-list-management`.
