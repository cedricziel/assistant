## Why

Conversations today get a "title" only when the first message arrives through a `web-ui` endpoint — and even then it's a dumb 57-character truncation of that first message. Conversations created by Slack, Matrix, Mattermost, Nextcloud, Signal, CLI, MCP, and the scheduler all land in the database with `title = NULL` and render as **"Untitled"** in every conversation list. The result is a list of Untitled rows interleaved with stub-truncated rows, and no way to tell threads apart at a glance.

We want a single mechanism that produces a meaningful title for every conversation regardless of which interface created it, once enough material exists for a model to summarise.

## What Changes

- Add a background **title-generator worker** that consumes `turn.result` from the existing `MessageBus`, gating on a configurable turn threshold (default: titled after turn 2 completes, or after turn 1 if the user message exceeds ~200 chars).
- Worker calls the conversation's LLM provider with a short summarisation prompt and writes the result via `ConversationStore::update_title` — this already broadcasts `Upserted` so reactive UIs pick it up live.
- Add a `title_locked BOOLEAN NOT NULL DEFAULT 0` column on `conversations`. Set to `1` whenever a title is written (auto or manual). The worker skips any conversation where `title_locked = 1`. Manual rename (`PATCH /api/conversations/{id}`) also locks. **No retroactive retitling, ever.**
- **BREAKING (internal)**: remove the 57-char auto-truncation in `web-ui/api/mod.rs::send_message` and `quick_message`. Remove the `"New Chat"` default in `POST /api/conversations`. Conversations stay `title = NULL` (rendered as "Untitled" — already supported in the read path) until the worker fills them in.
- Add `[titling]` block to `org.toml`: `enabled` (default true), `min_turns`, `long_first_message_chars`. The worker always uses the conversation's primary LLM provider; a per-org model override is deferred until `LlmProvider` exposes a per-call model knob.
- Retries: bounded via the existing bus redelivery cap (`MAX_TURN_REDELIVERIES`). Permanent failure leaves the conversation Untitled — user can rename manually.

## Capabilities

### New Capabilities

- `conversation-titling`: when, how, and under what conditions a conversation acquires a title; the worker contract; idempotency via `title_locked`; configuration surface.

### Modified Capabilities

None. The existing `conversation-event-log` spec governs the broadcaster used to deliver title updates; we reuse it without changes.

## Non-goals

- Backfilling titles for conversations that already exist as "Untitled" — out of scope. Operators can rename manually, or we can revisit in a separate change.
- LLM-driven **retitling** of conversations after scope drifts — explicitly rejected per discovery.
- Localisation of the title prompt — English-only for v1; a future change can templatise.
- Per-user (vs per-org) configuration of titling — org-level is sufficient for v1.

## Impact

- **Crates**: `assistant-runtime` (new `title_generator` module + worker spawn in startup), `assistant-storage` (migration adding `title_locked`, plumbing the flag through `ConversationStore`), `assistant-core` (no new envelope — reuses `TurnResult`), `assistant-web-ui` (remove truncation logic and `"New Chat"` default).
- **Config**: `org.toml` gains an optional `[titling]` block; missing block defaults to enabled.
- **OpenAPI**: no shape changes — `title` field stays `Option<String>`; the conversation list event already carries the new title via `Upserted`.
- **Tests**: TDD throughout. Unit tests for the worker's eligibility gate, in-memory bus integration test for the consume → LLM (wiremock) → update_title → Upserted path, storage migration test for the new column.
- **Cost**: ~$0.001/conversation on Claude Haiku; free on local Ollama. One extra LLM call per conversation, one-shot.

## User-facing documentation

**Yes — required.** Add a short section to `docs/operations/` describing the `[titling]` config block and how to disable it for cost-sensitive deployments. No end-user docs (titles just appear; no behavior change is observable to chat users besides "conversations now have titles").
