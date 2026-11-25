## Why

After a reply the chat offers no next step. A new conversation opens on the same fixed copy for every Persona (`app/lib/features/chat/empty_chat.dart:19-31`). assistant-ui has both features: follow-up suggestions generated once a turn settles (its `SuggestionAdapter.generate({messages})`) and starter suggestions on an empty thread. Without them, a Persona built for one job cannot point users at that job.

## What Changes

- **Server suggestions.** After a web turn sends `done`, the shared turn runner (`stream_turn_to_sse`, extracted by message-branching PR 5 from `send_message`, `crates/web-ui/src/api/messages.rs:111`) fires Web Push, closes the POST stream as today, and then makes one short extra call to the org's LLM using the last user message and the answer. It persists a new SSE event `suggestions` `{message_id, suggestions: [{prompt}]}` in `conversation_events` and broadcasts it to the run's replay subscribers. The call is skipped on error, on cancel, and when nobody is listening; it is aborted when the client leaves or on timeout. The phase runs only when the runner's caller opts in (`RunOptions.suggestions`).
- **`[suggestions]` in `org.toml`**: `enabled` (default `true`), `max_suggestions` (default `3`, clamped to `1..=5`), `timeout_secs` (default `10`, clamped to `1..=15`). Web text send, edit and regenerate opt in. Messenger, CLI, MCP, scheduler, A2A, voice and quick-message turns never get suggestions.
- **Run lifecycle.** The run, not `done`, bounds the replay stream: replay closes when the run finishes, so `suggestions` can follow `done`. The POST stream still closes at `done`. Turn status reads `completed` whenever a `done` row exists, where today it reads only the last row (`crates/web-ui/src/api/turns.rs:177-181`). A short ADR records the new invariant.
- **Client chips.** A new `SuggestionsEvent` joins the sealed `StreamEvent`. The turn finishes at `done` as today. The view model then opens the run's replay stream from `done.seq + 1` on its own subscription for up to 20 s, never touching the shared stream session. Chips render in `ChatThreadLayout.aboveComposer`. A tap sends the prompt as a user message. Chips clear, and the subscription is cancelled, on the next send or when the conversation changes.
- **Persona starter prompts.** A new `personas.starter_prompts` column (at most 4 prompts). The list appears on `PersonaDetail` and is edited through the new `PATCH /api/personas/{id}` and an editor in the Persona screens. Starters render in `ThreadEmptyState.starters`, and a tap sends the prompt.
- **New pure widgets** `SuggestionChips` and `StarterPrompts` in `app/packages/assistant_ui`, with widgetbook stories and goldens. No new dependencies.

## Capabilities

### New Capabilities

- `follow-up-suggestions`: generating, delivering, replaying and rendering suggestions after a turn.
- `persona-starter-prompts`: per-Persona starters, from storage through the API and editor to the empty state.

### Modified Capabilities

- `conversation-event-log`: replay closes when the run finishes, and `suggestions` may follow `done`.
- `turn-status-api`: state comes from whether a terminal row exists, not from the last row; terminal states outrank sibling non-terminal states.

## Impact

- **Code**: `crates/core` (config), `crates/runtime` (new `follow_ups.rs`), `crates/storage` (personas, one migration), `crates/web-ui` (turn runner, replay, turns, personas, test helpers), `openapi.json` and the regenerated `assistant_api`, `app/lib/api`, `app/lib/features/{chat,personas}`, `app/packages/assistant_ui`.
- **Cost**: when enabled, one short non-streamed LLM call per completed web turn (about 2 KB in, under 300 tokens out), traced in a GenAI span, plus one short-lived replay request from the client. The answer is not delayed, including on clients that buffer the response body until close (iOS Dio), because the POST stream still closes at `done`. On a single-GPU Ollama setup the call can briefly delay the user's next turn.
- **Visual change**: yes, for the chips, the empty-state starters and the Persona detail screen. Playwright baselines move.
- **User-facing docs**: needed. Update `docs/operations/configuration.md` (`[suggestions]`), `docs/operations/turn-status-endpoint.md` (terminal-row rule) and `docs/web-ui.md`. New ADR in `docs/adr/`.

## Non-goals

- Suggestions for voice turns or messengers (voice is excluded as a product choice; see design.md D2).
- Showing old suggestions again when a conversation is reopened.
- Suggestions written by the model inside its answer, or a separate cheaper model for suggestions.
- Org-wide default starters.
- Click-to-fill. A tap always sends.
