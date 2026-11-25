## Context

- Web turns stream from `send_message` (`crates/web-ui/src/api/messages.rs:111`). Once the turn ends, the spawned task persists the terminal `done` or `agent_error` row (`:455`), broadcasts it, sends it as SSE (`:466-470`), calls `run_broadcaster.finish_run` (`:474`) and only then fires the Web Push notification (`:477-485`). Push fires for every terminal kind. When the task ends, `sse_tx` drops and the POST stream closes.
- `message-branching` PR 5 extracts that post-`run_started` loop once, as `stream_turn_to_sse(…, RunOptions)` (`openspec/changes/message-branching/design.md`, "shared turn runner"). `chat-stop-cancels-turn` PR 6 moves `send_voice_message` onto the same runner with `RunOptions { suggestions: false, … }`. Every later change, this one included, edits the runner, not `send_message`. `RunOptions` fields are added by the first PR that reads them, so `suggestions` is added by PR 3 here.
- The replay endpoint `stream_run_events` closes right after replay if a `done` or `agent_error` row exists (`already_complete`, declared at `:698`, used at `:716`). While tailing it also stops at the first terminal frame (`is_terminal`, `:733-737`). Its doc comment says so (`:599-601`). A pruned run returns `404`, not `410` (`:674-679`, test `stream_run_events_returns_404_for_pruned_run`, `:1807`).
- Turn status works out `state` from the kind of the **last** row (`crates/web-ui/src/api/turns.rs:177-181`), and `docs/operations/turn-status-endpoint.md:63-67` documents that rule. Any row written after `done` would turn a finished turn back into `running`.
- Rows in `conversation_events` expire after 24 h (`crates/storage/src/conversation_events.rs:20`).
- The client stops reading the POST stream at `DoneEvent`: `_streamMessage` returns at `chat_provider.dart:790-799`, and `_replayRun` does the same at `:516`. `_drainQueue` awaits `_streamMessage` (`:574`). All of this bookkeeping goes through one shared `ChatStreamSession` (`chat_stream_session.dart`): `beginTurn()` resets `runId`/`lastSeq`/`cancelled` (`:58-63`), and `advance()` moves the replay cursor (`:66-68`). `retryMessage` replays with `_session.runId!, _session.lastSeq` (`chat_provider.dart:453-460`). The parser drops unknown event types without error: `parseSseByteStream` (`app/lib/api/api_client.dart`) has no else branch for them.
- iOS Dio / `dart:io HttpClient` can buffer a chunked body until the connection closes (`ChatStreamSession.initialStallTimeout` doc, `chat_stream_session.dart:17-27`). The 12 s stall probe then recovers through turn status (`_probeOrRecover`, `chat_provider.dart:913-941`). Any time the POST stream stays open after `done` is time a buffering client waits for the whole answer.
- An earlier feature already makes an extra LLM call after a turn. `title_generator.rs` (`generate_title`, `:50`; prompt packing, `:85`) runs on the org's main LLM (`SpaceRuntime.llm`, `crates/runtime/src/space_runtime.rs:45`). It is configured by `[titling]` (`crates/core/src/types/features.rs:197-237`, wired at `crates/core/src/types/agent.rs:62`).
- The web-ui test harness builds runtimes over an Ollama-shaped wiremock only (`crates/web-ui/src/api/test_helpers.rs:143-160`, `mount_llm_reply`; `state_over_in`, `:174-217`, builds `OrgConfig::default()` with `memory.enabled = false`). There is no `ScriptedLlmProvider` seam there.
- Persona rows hold no prompt metadata (`crates/storage/src/personas.rs:21-38`). There is no PATCH on the Persona resource itself: the router has `GET /api/personas/{id}`, the file slots, `POST /api/personas/active` and `PATCH /api/personas/{id}/skill-access` (`crates/web-ui/src/api/personas.rs:195-213`). `personaDetailProvider` already loads `getPersona` (`app/lib/features/personas/persona_detail_provider.dart:17-52`). The chat screen reads the active Persona from `personasProvider` (`chat_screen.dart:312`) and renders `const EmptyChat()` (`:467`); the screen is 623 lines, over the ~300-line guideline in `openspec/config.yaml`.
- chat-element-library supplies the slots: `ChatThreadLayout.aboveComposer` (its PR 10) and `ThreadEmptyState.starters` (its PR 9).

## Goals / Non-Goals

**Goals**: show follow-up chips right after a web turn without delaying the answer, the composer or the queue, also on clients that buffer the POST body; replay must survive a reconnect; an org can turn the feature off; per-Persona starters that are editable in the app.

**Non-goals**: listed in proposal.md.

## Decisions

### D1. Generate with a separate LLM call after `done`, not from the model's answer

The server runs `follow_ups::generate(last_user, answer, max, llm)` after the turn result arrives. It sends one non-streamed `llm.chat(SYSTEM, [user_msg], &[])` with no tools, as `generate_title` does. The input is the last user message and the final answer, each cut to 1 000 characters (Unicode scalar values). The prompt asks for at most N short prompts that the _user_ could send next, one per line, in the user's language, with no numbering.

`parse_suggestions(raw, max)` is a pure function. It strips bullets, numbering and wrapping quotes, drops empty lines and lines over 120 characters, removes case-insensitive duplicates, and keeps the first `max`. Tool-call or thinking-only responses produce no suggestions.

The call runs inside a `gen_ai` span (`gen_ai.operation.name = "chat"`, `gen_ai.request.model`, `gen_ai.usage.input_tokens`/`output_tokens` when the provider reports them) with `assistant.feature = "follow_up_suggestions"`, so its cost is visible next to the turn's own spans (see the `opentelemetry-genai-spec` skill). The outcome (`emitted`, `empty`, `timeout`, `error`, `aborted_no_listener`) is a span attribute.

Alternatives considered:

- **The model emits suggestions in its answer** (a trailing `<suggestions>` block, or a `suggest-follow-ups` tool). This costs no extra call, but the block would stream into the visible answer and have to be stripped mid-stream. Small Ollama models follow the format unreliably. A tool adds a ReAct iteration (a full LLM round-trip) before `done`, so the answer arrives later. Every Persona's system prompt would also carry the instruction, including for messengers, where it is unwanted. Rejected.
- **The client asks on demand** (`POST .../messages/{id}/suggestions`). A call is only made when someone is looking, but this adds an endpoint plus a client request after every turn, and the binding cross-change decision says the result is an SSE event that survives replay. Rejected.
- **A bus worker on `turn.completed`**, like titling. It runs for every interface, so messengers would need filtering. Its at-least-once retries with backoff (`title_generator.rs:200-206`) arrive long after the moment has passed, and it has no path back into the run's SSE stream. Rejected.

Cost and latency: one extra short call per completed web turn. It runs after `done` reaches the client, so the answer and the composer do not wait for it. Chips appear about 1–3 s later on hosted models; a local model may take longer. On a single-GPU Ollama setup the extra call can briefly hold up the user's next message (see D4 for the abort that limits this). `enabled = false` is the remedy, and docs say so.

### D2. Explicit opt-in on the shared turn runner

The suggestion phase lives in the shared turn runner (`stream_turn_to_sse`) and runs only when the caller passes `RunOptions.suggestions = true`. PR 3 adds that field. Per entry point:

| Entry point                          | `suggestions` | Owner                       |
| ------------------------------------ | ------------- | --------------------------- |
| `send_message`                       | `true`        | this change, PR 3           |
| `edit_message`, `regenerate_message` | `true`        | message-branching PR 7      |
| `send_voice_message`                 | `false`       | chat-stop-cancels-turn PR 6 |

`quick_message` (`:783`) does not use the runner. Messenger, CLI, MCP, scheduler and A2A turns never reach it.

Voice is excluded as a product decision, not a technical one: once chat-stop-cancels-turn gives voice a run log, the runner could deliver chips there too. Voice is a hands-free flow where the reply is spoken, and tappable text chips do not fit it. Quick-message callers are machines. The gate is a flag, so either decision can be reversed with a one-line change and a test.

Alternative: infer the gate from `Interface::Web`. Voice is also `Interface::Web`, so this would turn suggestions on for voice as soon as it moves onto the runner. Rejected.

### D3. `[suggestions]` in `org.toml`

```toml
[suggestions]
enabled = true        # master switch
max_suggestions = 3   # 1..=5; values outside are clamped
timeout_secs = 10     # 1..=15; values outside are clamped; whole-phase budget
```

`SuggestionsConfig` sits next to `TitlingConfig` in `features.rs` and becomes `OrgConfig.suggestions`, with `#[serde(default)]`. `effective_max()` and `effective_timeout()` clamp. The upper timeout bound of 15 s is a protocol constant (`SUGGESTIONS_MAX_TIMEOUT_SECS`) that the client's listen window (D8) is derived from. A space reads it from `SpaceRuntime.config`. The call uses the org's main LLM. A cheaper per-feature model waits for the same `LlmProvider` model-selection knob that titling is waiting for (see the `TitlingConfig` doc comment).

### D4. Post-`done` phase: the POST stream closes at `done`, suggestions arrive on the run tail

In the runner, after the terminal frame:

1. Send the terminal frame on `sse_tx` and remember whether that send succeeded (`client_present_at_done`).
2. Fire the Web Push notification. Push now runs ahead of `finish_run` for **every** terminal kind, as before, so suggestion latency never delays it. The behaviour is otherwise unchanged.
3. Drop `sse_tx`. The POST stream closes right after `done`, as it does today, so a client that buffers the body until close gets the whole answer with no extra delay.
4. If `options.suggestions`, `cfg.enabled`, the terminal is `done` (not `agent_error` of any reason), `done` carried a `message_id`, and someone is listening (`client_present_at_done || broadcast_tx.receiver_count() > 0`), run `follow_ups::generate` under `tokio::time::timeout(cfg.effective_timeout())`, raced in a `tokio::select!` against a listener watchdog: after an attach grace of 2 s, the watchdog polls `broadcast_tx.receiver_count()` every 500 ms and resolves when it is `0`. When the watchdog wins, the LLM future is dropped (aborting the HTTP call) and nothing is emitted.
5. If the result is non-empty, persist `suggestions` at `seq + 1`, then broadcast it.
6. `finish_run`.

The client reads suggestions from the run's replay stream, which it opens itself right after `done` (D8). The attach grace covers the gap between the POST closing and the replay subscription. If the suggestions are ready before the client subscribes, the row is already persisted, and replay returns it.

Errors, timeouts and empty results log at `debug`/`warn` and emit nothing; there is no error frame. Skipping when nobody is listening saves the call when the tab is gone. In exchange, a client that reconnects later gets no chips, which is acceptable for an ephemeral hint.

Alternatives considered:

- **Keep the POST stream open until the phase ends and send `suggestions` on it.** The browser gets chips without a second request, but a buffering client (iOS Dio) then sees `done` only when the stream closes, up to `timeout_secs` later, and the stall probe may route it into conversation-fetch recovery. Rejected: it delays the answer on iOS.
- **Same, and document the iOS degradation.** Smaller change, but it trades a real latency regression on one platform for one fewer request. Rejected.
- **Check "nobody listening" once, before the call.** The original draft. A client that has already moved on still looks present right after `done`, and hyper only notices a dropped connection on its next write. The watchdog makes a departed client abort the call. Rejected.

### D5. Payload and naming

```json
event: suggestions
id: 57
data: {"message_id":"<uuid>","suggestions":[{"prompt":"Show me an example"}]}
```

Each item is an object, so a later `title` or `label` (assistant-ui `ThreadSuggestion{prompt,title?,label?}`) can be added without breaking anything. `message_id` ties the chips to the assistant message. The client shows them only while that message is the last one in the thread, which also covers message-branching's regenerate and edit flows. The payload is documented in OpenAPI as `SseSuggestionsEvent { message_id, suggestions: [SseSuggestion { prompt }] }` next to the existing `Sse*Event` schemas (`messages.rs:509-590`).

### D6. Persist in the run log; no `messages` column

The `suggestions` row goes into `conversation_events` like any other frame, so replay with `since=N` returns it and the 24 h TTL applies. Suggestions are not stored on `messages`, and `GET` conversation history does not return them. Reopening a conversation hours later should not offer stale next steps. This is the product choice flagged in the open questions.

### D7. The run, not `done`, bounds the replay stream; status reads "has a terminal row"

- `stream_run_events`: close after replay when `!is_active` (the broadcaster has no entry). Otherwise tail until `RecvError::Closed`. The `already_complete` short-circuit and the `is_terminal` early return are deleted. The tail loop also `select!`s on `sse_tx.closed()`, so a departed replay client drops its broadcast receiver promptly, which is what D4's watchdog reads. Run teardown already drops the sender (`finish_run`), so the tail ends within the suggestion budget.
- The POST stream is unchanged: it still closes at the terminal frame (D4 step 3).
- Turn status, in one precedence order shared with tool-approval-prompts: `errored` if any `agent_error` row exists, else `completed` if any `done` row exists, else `awaiting_input` if any `interaction_requested` row has no matching `interaction_resolved` (added by tool-approval-prompts PR 8, rebased on this rule), else `running`. `last_event_kind` and `last_event_at` still describe the last row, which may be `suggestions` (or a sibling's `interaction_requested`, `interaction_resolved`, `surface_update`).
- This is a protocol invariant other changes build on (generative-ui-a2ui, tool-approval-prompts, chat-stop-cancels-turn), so it gets a short ADR ("The run, not `done`, bounds the replay stream"; number assigned from the cross-change stack plan, the next free after the reserved adr-0017..0019 and the sibling-claimed adr-0020).

Alternative: keep `done` last and send suggestions on the conversation-list SSE (`GET /api/conversations/stream`). That stream carries conversation upserts, not per-run frames, and it is not replayable by `since`. Rejected.

### D8. The client finishes the turn at `done` and listens for suggestions on its own subscription

`_streamMessage` and `_replayRun` keep their current shape and still return at `DoneEvent`, so `_drainQueue` moves on at once and the queue drains at `done` (or at chat-stop-cancels-turn's stopped reconciliation, whichever ends the turn). Neither loop is converted to a `StreamIterator`.

On `DoneEvent` with a `messageId`, the view model starts a `FollowUpListener` (a small class in `features/chat/`, not part of `ChatStreamSession`) with **captured** values: `conversationId`, `runId`, `since = done.sequenceId + 1`, `messageId`. The listener:

- opens `GET /api/conversations/{id}/runs/{run_id}/events/stream?since=…` with its own `CancelToken`;
- reads until a `SuggestionsEvent`, the stream end, or `kSuggestionsListenWindow` (20 s: the server's 15 s maximum budget from D3 plus 5 s of network margin), then cancels;
- never reads or writes `_session`: no `advance`, no `runId`, no `cancelled`, no detach state. A newer turn's replay cursor therefore cannot be moved by an older run's late frame;
- writes `ChatState.suggestions` only if `messageId` is still the last assistant message and no send has started since.

The view model cancels the listener (closing the HTTP connection, which ends the server's replay tail and lets D4's watchdog abort the call) when `sendMessage`/`_streamMessage` starts a new turn, when the conversation is loaded, cleared or switched, and on dispose. The listener's stream is wrapped by chat-stop-cancels-turn's `_untilDetached` like every other source, so a detach also ends it. `sendMessage`, `loadConversation`, `clearConversation` and conversation switches clear `suggestions`. message-branching PR 12, which moves stream consumption into `chat_stream_session.dart`, extracts this design as it stands after PR 7b.

Alternatives considered:

- **Convert both loops to `StreamIterator` and keep reading the same stream in the background.** Requires the POST stream to stay open (rejected in D4), and the background reader shares `_session` with the next turn: `beginTurn()` resets `runId`/`lastSeq`, and a late `advance()` from the old run would set the new run's replay cursor to the wrong `since`. Rejected.
- **Keep `await for` until the stream closes.** Every queued message would then wait out the suggestion phase. Rejected.

### D9. Widgets

These live in `app/packages/assistant_ui`, are pure, and use only `flutter`/`material_ui`:

- `SuggestionChips(prompts: List<String>, onSelected: ValueChanged<String>, enabled: bool = true)`: a wrapped row of outlined chips, keyed `suggestion_chip_<i>`, inside `Semantics(container: true, label: 'Suggested follow-ups')`. Each chip has button semantics labelled with its prompt and a minimum 48 dp target. Each chip is cut to 2 lines, with the full prompt in its tooltip.
- `StarterPrompts(prompts, onSelected)`: a column of tappable cards (on wide layouts, a 2-column grid chosen by a `LayoutBuilder` width check, not by platform), keyed `starter_prompt_<i>`.

The app wiring stays in `features/chat`, in small adapters rather than in the 623-line `chat_screen.dart`: `follow_up_chips.dart` passes `SuggestionChips` into `aboveComposer` when `chatState.suggestions` is non-empty and `!isSending`. `chat_empty_starters.dart` passes `StarterPrompts` into `ThreadEmptyState.starters` from `activePersonaStarterPromptsProvider`, which derives from `personaDetailProvider(activePersona.id)` and handles loading, error and empty states.

### D10. Storing and editing starter prompts

Migration `NNN_persona_starter_prompts.sql` runs `ALTER TABLE personas ADD COLUMN starter_prompts TEXT NOT NULL DEFAULT '[]'`, a JSON array. Its number comes from the cross-change stack plan (otherwise the next free after `045_skill_executions.sql` at implementation time), and it is registered in `run_migrations` (`crates/storage/src/lib.rs:297`). `seed_database_through_016` (`:562`) is a fixed test fixture and stays unchanged. `PersonaRecord.starter_prompts: Vec<String>`. `PersonaStore::set_starter_prompts(id, &[String])` is implemented for both `Sqlite` and `InMemory` (ADR-0009). Malformed JSON in the column reads as an empty list with a `warn!`; it never panics.

API: `PersonaDetail.starter_prompts: Vec<String>`, always present and possibly empty. `PATCH /api/personas/{id}` (`operationId: update_persona`) takes the body `UpdatePersonaRequest { starter_prompts?: Vec<String> }` and returns `200 PersonaDetail`. If a field is absent, it is left unchanged, so `{}` is a no-op that still returns `200`; `[]` clears the list. Validation fails with `400 {"error": ...}` when there are more than 4 prompts, or when a prompt is empty after trimming or longer than 200 characters. Lengths count Unicode scalar values (Rust `chars().count()`), after trimming. Prompts are stored trimmed. Write access follows the existing Persona write rules (`Access::Write` gives `403`, `get_accessible` gives `404`). `PersonaSummary` stays flat (openspec/config.yaml forbids nested collections on summaries).

Client: the editor builds on `personaDetailProvider` instead of fetching `getPersona` a second time. After a successful `updatePersona`, it invalidates `personaDetailProvider(id)` and `activePersonaStarterPromptsProvider`, so the detail screen and the chat empty state show the new list.

Alternative: a `STARTERS.md` file slot (`PERSONA_FILE_SLOTS`, `personas.rs:41`). It costs no migration, but it is unstructured, needs ad-hoc line parsing, and file slots resolve under the install root rather than the space. Rejected.

## Risks / Trade-offs

- **Ollama contention**: the suggestion call can hold up the user's next turn on a single GPU. Mitigations: the clamped budget (at most 15 s), the listener watchdog that aborts when the client leaves or starts a new turn (D4, D8), and `enabled = false`, all documented.
- **Disconnect detection**: the watchdog relies on the replay tail dropping its broadcast receiver when the client goes away. The tail `select!`s on `sse_tx.closed()`, but hyper may notice a dropped connection only on its next write (keep-alive pings every 15 s, `crates/web-ui/src/api/mod.rs:73`). The timeout bounds the worst case.
- **One extra request per turn**: the client opens a replay subscription after every `done` when suggestions are enabled. It is short-lived (closes at `finish_run`) and cheap.
- **Off-topic or unsafe suggestions**: suggestions are only text the user must tap, and they are sent as an ordinary user message, so nothing runs without the user's action. The parser caps length and count.
- **Lifecycle change ripples**: the POST stream keeps its "closes at the terminal frame" behaviour. Only the replay stream changes: a replay client now waits up to `timeout_secs` longer after `done` before it closes. A client that stops at `done` still works.
- **Shared runner interlock**: the phase lives in `stream_turn_to_sse` from message-branching PR 5. If this change landed first and put the phase in `send_message`, the extraction would move it, and voice (chat-stop-cancels-turn PR 6) would inherit it. PR 3 therefore depends on message-branching PR 5 and adds the `suggestions` flag that voice sets to `false`.
- **Turn-status precedence**: tool-approval-prompts PR 8 adds `awaiting_input` at the same lines (`turns.rs:177`). The combined order is fixed in D7, and tool-approval-prompts PR 8 depends on PR 2 here. Archive order: chat-stop-cancels-turn, then follow-up-suggestions, then tool-approval-prompts.
- **Client stream lane**: chat-stop-cancels-turn PR 4/5, this change's PR 7b and message-branching PR 12 all edit the stream loop. Order: chat-stop PR 4, chat-stop PR 5, follow-up-suggestions PR 7b, message-branching PR 12. Each PR that adds a `StreamEvent` subclass updates every exhaustive switch.

## Migration Plan

One additive column (D10). There is no backfill and no compatibility shim. Existing Personas start with `[]`.

## Open Questions

1. Should `[suggestions].enabled` default to `true` (like `[titling]`) or `false`, given the extra cost per turn? The draft says `true`.
2. Should chips come back when a conversation is reopened later? That needs a `messages.suggestions` column and a history field. The draft says no.
3. Should Personas with no starters fall back to org-level default starters? The draft says no: the empty state keeps today's copy.
4. Should voice turns get suggestions once voice has a run log? The draft says no (D2).
