## Why

A conversation is a flat, append-only list today. `messages` has no parent column (`migrations/001_conversations.sql:10`), `load_history` orders by `turn, created_at` (`crates/storage/src/conversations.rs:465`), and `base_turn` is the row count (`crates/runtime/src/orchestrator/mod.rs:1554`). Users cannot fix a sent message, get a second answer, or try another model on the same question. `chat-element-library/elements.md:44` adopts "Message branches" through this change.

## What Changes

- **Message tree.** `messages.parent_id`, `messages.run_id` and `conversations.active_leaf_id`. `save_message` appends at the active leaf in one `BEGIN IMMEDIATE` transaction, so existing save sites stay as they are. `load_history` returns the **active path** only.
- **Non-destructive compaction.** Compaction records a summary keyed by the last message it covers instead of deleting rows (`replace_history`, `conversations.rs:369`). One summary-applied history builder serves turns, `/status` and `/compact`. `replace_history` is removed.
- **Edit and regenerate.** `POST /api/conversations/{id}/messages/{message_id}/edit` and `.../regenerate` stream SSE through one shared runner extracted from `send_message`. They travel as a `TurnAnchor` on `TurnRequest`, and a re-delivered turn replaces its own partial rows. Edit carries the original message's attachments forward to the new version, through a link table.
- **One in-flight registry.** An orchestrator-owned turn registry replaces the three `active_turns` maps. Branch operations take an exclusive lease and return 409 while a turn runs. The exception is a turn that is only waiting for the user (a pending approval or question): a branch operation cancels that wait and proceeds. A plain send during an edit or regenerate gets 409, and a messenger turn is deferred and retried.
- **Regenerate with another model.** An optional `model`, limited to the curated `[llm] available_models` allowlist in `org.toml` and listed by `GET /api/models`. Any member with write access may use it. It applies to one turn. Assistant rows record their model.
- **Branch navigation.** `MessageSummary` carries `parent_id` and `branch {index, count, sibling_ids}`. `POST /api/conversations/{id}/switch-branch` moves the active leaf to the most recently created child at each fork.
- **Messenger-shared conversations.** Edit, regenerate and switch are allowed in conversations that messenger interfaces also use. The messenger's next turn continues the active branch.
- **Client.** `BranchPicker`, `MessageEditor` and `RegenerateMenuButton` in `assistant_ui`. Edit and Regenerate go into `buildMessageActions`, with the picker in `MessageActions.leading`. Edit is offered for messages with attachments too. Edit, Regenerate and the picker stay enabled while a turn waits for input. Retrying a failed message that the server has persisted regenerates it.
- ADR-0018.

## Capabilities

### New Capabilities

- `conversation-branching`: covers the server: tree storage, the active path, non-destructive compaction, the in-flight registry, anchored turns, the edit, regenerate, switch-branch and models endpoints, and branch metadata.
- `chat-branch-ui`: covers the Flutter side: the picker, inline editing, the regenerate actions, retry and their wiring.

### Modified Capabilities

None. Retry lives in `chat-branch-ui`, because `chat-message-retry` has no `## Requirements` section to apply a delta to.

## Impact

- **Crates:** storage (4 migrations: message tree, attachment links, summaries, message model; trait changes), core (`TurnRequest`, `Message`, `LlmConfig`), runtime, web-ui (4 routes, `openapi.json`), interface-cli (`active_turns` removal).
- **Flutter:** `assistant_ui` elements with widgetbook stories and goldens. `ChatNotifier` gains edit, regenerate and switch.
- **Visual change:** a picker, actions and an inline editor. After compaction, earlier messages stay visible. **Playwright baselines move.**
- **User-facing documentation: yes.** A branching section in `docs/web-ui.md` and `available_models` in `docs/operations/configuration.md`.

### Non-goals

- Switching provider kind or endpoint on regenerate. An org has one `LlmConfig` (`crates/core/src/types/llm.rs:97`).
- Branching from messenger interfaces. They keep appending to the active leaf.
- Deleting, pruning or comparing branches.
- Adding or removing attachments while editing, and editing assistant messages. Edit changes the text only; the attachments carry forward unchanged.
- A cross-process in-flight guard. The registry is per process (design D7).
- Posting an edited or regenerated reply back to a messenger thread. Those runs reply on the web only.
