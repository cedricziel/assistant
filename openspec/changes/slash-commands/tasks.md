## 1. Core types in `assistant-core`

- [x] 1.1 Add `CommandDef` struct (name, description, category, args schema) and `CommandResult` struct (ack_text, side effects) to `assistant-core/src/types.rs` or a new `command.rs` module. Export from `lib.rs`.
- [x] 1.2 Add `ConversationConfig` struct with `model_override: Option<String>` field to `assistant-core`.
- [x] 1.3 Write unit tests for `CommandDef` construction and `ConversationConfig` default/override semantics.

## 2. Storage: `conversation_events` table

- [x] 2.1 Add SQLite migration creating `command_events` table (id, conversation_id, event_type, command, payload, ack_text, created_at) with index on conversation_id.
- [x] 2.2 Add `CommandEventStore` with `save_event()` and `list_events(conversation_id)` methods.
- [x] 2.3 Write integration tests: insert event, list events by conversation, empty result for unknown conversation.

## 3. Command registry and built-in commands in `assistant-runtime`

- [x] 3.1 Create `CommandRegistry` struct with `list()`, `get(name)`, and `execute(name, args, ctx)` methods. The `ctx` carries conversation_id, orchestrator ref, conversation config map, and adapter send callback.
- [x] 3.2 Implement `/help` command — lists all registered commands with descriptions.
- [x] 3.3 Implement `/new` command — evicts conversation key from LRU cache (via ctx callback), clears ConversationConfig for the conversation.
- [x] 3.4 Implement `/status` command — reads current model (override or default), conversation ID, estimated token count, interface type.
- [x] 3.5 Implement `/model` command — sets/shows per-conversation model override in ConversationConfig map.
- [x] 3.6 Implement `/stop` command — looks up active request_id for conversation, cancels the CancellationToken in orchestrator's turn_cancellations.
- [x] 3.7 Implement `/compact` command — loads conversation history, calls `maybe_compact()` directly, persists result.
- [x] 3.8 No change needed: `maybe_compact()` already works unconditionally — callers guard with `should_compact()`. `/compact` command calls `maybe_compact()` directly without the guard.
- [x] 3.9 Write unit tests for each command: `/help` output format, `/new` eviction, `/model` set/get, `/stop` with/without active turn, `/compact` force flag, `/status` with/without override.

## 4. ChannelRunner command interception

- [x] 4.1 Add `active_turns: Arc<RwLock<HashMap<Uuid, Uuid>>>` (conv_id → request_id) to `ChannelRunner` for turn tracking.
- [x] 4.2 Add command interception in `ChannelRunner::run()` loop: before dispatch, check if text starts with `/` + registered command name. Route to `CommandRegistry::execute()` instead of `dispatch()`.
- [x] 4.3 For `/compact`, acquire the per-conversation lock before execution. For all other commands, execute without locking.
- [x] 4.4 After command execution, persist a `command_events` record and send ack via `adapter.send()`.
- [x] 4.5 Wire active_turns tracking: record request_id before dispatch, remove after dispatch completes.
- [ ] 4.6 **DEFERRED**: Add orchestrator support for per-conversation model override. Requires LlmProvider model-switching or per-turn provider construction — separate change. `/model` stores the override; wiring is future work.
- [x] 4.7 Tests: parse/interception covered by CommandRegistry unit tests (parse_recognized, parse_unrecognized, passthrough).

## 5. Web UI API endpoints

- [x] 5.1 Add `GET /api/commands` endpoint returning bare JSON array of command definitions (name, description, category, args). Include OpenAPI docs with `operationId: list_commands`.
- [x] 5.2 Add `POST /api/conversations/{id}/command` endpoint accepting `ExecuteCommandRequest { command, args }`, returning `CommandEventResponse`. Include OpenAPI docs with `operationId: execute_command`.
- [x] 5.3 Add `GET /api/conversations/{id}/events` endpoint returning bare JSON array of `CommandEventResponse` sorted by created_at. Include OpenAPI docs with `operationId: list_conversation_events`.
- [x] 5.4 Write handler tests: list commands, execute valid command, execute unknown command (400), list events.
- [ ] 5.5 Run `make dump-openapi` to update `openapi.json` with the new endpoints.

## 6. CLI migration

- [x] 6.1 Replace the hardcoded slash command matching in `main.rs` with a call to `CommandRegistry::execute()` for shared commands (`/new`, `/stop`, `/model`, `/compact`, `/status`, `/help`).
- [x] 6.2 Retain CLI-local commands (`/quit`, `/exit`, `/skills`, `/review`, `/install`) as a local fallback before the registry check.
- [x] 6.3 Test that CLI REPL dispatches both local and registry commands correctly.

## 7. Flutter app: autocomplete and timeline (web, macOS, iOS)

- [x] 7.1 Run `make generate-flutter-client` to pick up new API endpoints.
- [x] 7.2 Write failing widget test: typing `/` in the input field shows the autocomplete popup.
- [x] 7.3 Build the command autocomplete popup widget: triggered on `/` as first char, fetches and caches `GET /api/commands`, filters by prefix as user types.
- [x] 7.4 Write failing widget test: popup filters commands by prefix as user types (e.g. `/mo` → `/model`).
- [x] 7.5 Write failing widget test: Escape dismisses popup, backspace past `/` dismisses popup, selecting a no-arg command submits immediately.
- [x] 7.6 Wire popup dismissal on Escape, backspace past `/`, and command selection (no-arg commands submit immediately).
- [ ] 7.7 **DEFERRED**: Write failing widget test: selecting `/model` fills input and shows argument completions from the completions endpoint. Requires `/api/models` backend endpoint — separate change.
- [ ] 7.8 **DEFERRED**: Add argument completion for `/model`: fetch model list from completions endpoint when user selects the command. Requires `/api/models` backend endpoint — separate change.
- [x] 7.9 Write failing widget test: `CommandEventTile` renders command name and ack text in system-event style.
- [x] 7.10 Add `CommandEventTile` widget for timeline rendering: distinct system-event style showing command name and ack text.
- [x] 7.11 Write failing test: conversation timeline merges messages and command events by timestamp.
- [x] 7.12 Update conversation timeline to render CommandEventTile for command entries alongside messages.

## 8. Integration and cleanup

- [x] 8.1 Run `make lint && make format` to ensure all code passes checks.
- [x] 8.2 Run `make test` to verify all existing tests still pass with the `force` parameter change in `maybe_compact()`.
- [x] 8.3 Run `make lint-flutter && make test-flutter` for Flutter checks.
