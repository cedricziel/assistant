# Design — generative-ui-a2ui

## Context

| Fact                                                                                                    | Where                                                                                      |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Per-interface output text is static and compiled in                                                     | `crates/runtime/src/orchestrator/prompt.rs:157` (`output_capabilities`)                    |
| Tool specs are filtered per turn only by provider-hosted tools; three call sites                        | `dispatch.rs:42` (`filter_tool_specs`); `mod.rs:783,1186`; `subagent.rs:142`               |
| `ToolSpec { … }` struct literals                                                                        | 22 literals in 13 files (`grep -rn 'ToolSpec {' crates`)                                   |
| Tool-call message persisted **before** execution                                                        | `dispatch.rs:88` (`persist_tool_calls`, returns nothing today)                             |
| Observation is built from `exec_result`, then `ToolResult` is emitted                                   | `dispatch.rs:229–287`; `AudioReady` is sent after it (`dispatch.rs:295`)                   |
| `ToolResult` carries a truncated string only, `data` is dropped                                         | `dispatch.rs:283`                                                                          |
| Projection is total, with no `_` arm, across three projectors                                           | `crates/runtime/src/projection/mod.rs:22,48,225`; `crates/web-ui/src/a2a/projection.rs:67` |
| SSE frames are persisted and broadcast per projected frame, then sent to the client                     | `crates/web-ui/src/api/messages.rs:226–367`                                                |
| The loop **stops** on the first failed client send, so later frames are neither persisted nor broadcast | `messages.rs:351–353,360–362` (`if sse_tx.send(..).await.is_err() { break; }`)             |
| `quick-message` runs a synchronous, non-streaming turn as `Interface::Web`                              | `messages.rs:869` (used by `AssistantIntents/.../AssistantAPIClient.swift`)                |
| `SendMessageRequest.message` is required and must be non-empty                                          | `messages.rs:55–61,129–130`                                                                |
| The user message is persisted with exactly the string passed to `submit_turn`                           | `crates/runtime/src/orchestrator/mod.rs:1561–1565`                                         |
| History replay uses stored content; attachments use a side map keyed by message id                      | `crates/runtime/src/history.rs:29–48`; `mod.rs:1586–1591` (`build_attachment_map`)         |
| `tool_calls_json` stores `ToolCallItem { name, params, id: Option<String> }`                            | `crates/core/src/llm/types.rs:78–84`                                                       |
| `ToolCallSummary` has no `tool_call_id` today                                                           | `crates/web-ui/src/api/conversations.rs:86–98` (added by tool-result-renderers S4)         |
| Event log is ephemeral (24h TTL)                                                                        | `crates/storage/src/conversation_events.rs:20`                                             |
| Org settings are `OrgConfig` sections                                                                   | `crates/core/src/types/agent.rs:45`                                                        |
| Attachment bytes need bearer + space headers; the app fetches them by hand                              | `crates/web-ui/src/api/mod.rs:267`; `app/lib/features/chat/message_attachments.dart:196`   |
| Client streams send_message through a hand-written client that drops the HTTP status                    | `app/lib/api/api_client.dart:265–291` (`ErrorEvent(e.message)`)                            |
| Client ignores unknown SSE events: `if/else` chain with no `else`, plus `default: return false`         | `app/lib/api/api_client.dart:780`; `app/lib/features/chat/chat_provider.dart:111–160`      |
| `jsonschema` 0.49 is already a workspace dep                                                            | `Cargo.toml:150` (used by `tool-executor`)                                                 |
| Space migrations are numbered SQL files, last `045_skill_executions.sql`                                | `migrations/`, registered in `crates/storage/src/lib.rs:472`                               |
| App uses `material_ui`; framework `ThemeData` is published by a bridge                                  | `app/lib/shared/theme/legacy_theme.dart:1–10`                                              |
| App SDK `^3.13.0`, `url_launcher ^6.3.3`                                                                | `app/pubspec.yaml:6,30`                                                                    |

Sibling facts this design relies on:

- `tool-approval-prompts` adds `TurnRequest.interactive` / `ExecutionContext.interactive` (its PR 6), the `InteractionBroker` whose `request()` inserts the row, emits `interaction_requested` and awaits (its D2, PR 7), an `InteractionKindHandler` registry (`validate_response`, `observation`, `push_copy`, `progress_label`, `renders_card`) in PR 7/8, and the respond endpoint (PR 8).
- `message-branching` PR 5 extracts the streamed-turn runner `stream_turn_to_sse` taking `RunOptions { interactive, suggestions, push, anchor }`; PR 4 adds `conversation_turn_in_flight`; PR 7 pre-assigns `user_message_id` in `send_message`.
- `tool-result-renderers` S2/S3 rework `finalize_tool` to forward and persist `ToolOutput.data`; S4/C1 add `tool_call_id` to `ToolCallSummary` and the client `ToolCallRecord`.
- `chat-element-library` PR 3 (widgetbook and registry), PR 4 (render models incl. `TimelineEntryType` move into the package), PR 5 (timeline entry shell, `body` slot, `EntryState.awaitingInput`).

## External facts (versions cited)

- **A2UI v1.0** ("Current", a2ui.org/specification/v1.0-a2ui, updated 2026-06-08). Agent→renderer: `createSurface` (`surfaceId`, `catalogId`, `sendDataModel`, `components`, `dataModel`), `updateComponents`, `updateDataModel` (JSON Pointer `path`, `value`, null deletes), `deleteSurface`, plus function RPC. Renderer→agent: `action` (`name`, `surfaceId`, `sourceComponentId`, `timestamp`, `userMessage?`, `context`), `error`. Every message carries `"version": "1.0"`. Bindings use `{"@path": "/x"}`. Theme properties were removed in v1.0. `openUrl` runs only from a user-gesture action.
- **Basic catalog v1.0** `https://a2ui.org/specification/v1_0/catalogs/basic/catalog.json`: 18 components and 14 functions, JSON Schema 2020-12, `$ref`s an external `common_types.json`. `discriminator` is OpenAPI-only.
- **Flutter**: `genui` 0.10.4 is discontinued and replaced by `a2ui_flutter` 0.5.0 (Dart ≥ 3.13, deps `a2ui_core ^0.9.0`, `path_parsing`, `video_player`) and `a2ui_core` 0.9.0 (pure Dart). API: `MessageProcessor(catalogs: …)`, `processMessages(json)`, `model.onSurfaceCreated/onAction/onError`, `A2uiSurface(surface:)`, `basicCatalog(openUrl: …)`, `BasicComponents`, `CatalogImplementation`. The basic `Image` documents no header or loader hook. Widgets are framework Material. Both packages are 0.x and weeks old.
- **Regex engines**: the Rust `regex` crate (1.12) is linear-time, so it accepts `(a+)+$` without complaint. `a2ui_core` evaluates the `regex` function with Dart `RegExp`, which backtracks (and compiles to JS `RegExp` on web).

## Decisions

### D1. Protocol v1.0 with our own catalog id

We target v1.0 only. Our catalog keeps the basic-catalog component _shapes_ but ships under its own id, `https://assistant.invalid/a2ui/catalogs/assistant/v1/catalog.json`. Components: Text, Image, Icon, Row, Column, List, Card, Tabs, Divider, Button, CheckBox, TextField, DateTimeInput, ChoicePicker, Slider. Functions: required, regex, length, numeric, email, formatString, formatNumber, formatCurrency, formatDate, pluralize, and, or, not, openUrl.

_Alternatives_: (a) the basic catalog id as-is, which would allow Video, AudioPlayer and Modal; (b) v0.9, which is superseded and lets `openUrl` fire without a gesture.

The vendored catalog (with `common_types.json` inlined) lives in `crates/a2ui/catalog/`. A test on each side asserts the component set matches `crates/a2ui/catalog/components.txt`.

### D2. The model calls a tool, the server writes the protocol

`render-surface` has two modes:

- **Create** (no `surface_id`): `title`, `fallback_text` and `components` are required; `data_model`, `send_data_model` (default true) and, once enabled, `wait_for_action` (default false) are optional. The server builds `createSurface`.
- **Update** (`surface_id` set): at least one of `components` (upserted by id → `updateComponents`) or `data_updates` (`[{path: JSON Pointer, value}]`, `value: null` deletes → one `updateDataModel` per entry). `title` and `fallback_text` are optional and replace the stored values. The surface must be `active` and in the same conversation.

All validation limits (D3) apply to the **merged** surface after an update. Every successful call, create or update, counts toward the 5-per-turn cap.

There is no close or delete mode (see non-goals). A surface is `active` until submitted.

_Alternatives_: the model streams raw A2UI JSONL in its text (no server validation before the client sees it; mixed channels; a parser per provider). The model authors full envelopes (more tokens, more failure modes, model-chosen ids). Both rejected. The cost is that a surface appears all at once.

### D3. Validation is server-side, total and structured

`assistant_a2ui::validate(&SurfaceDraft, &UrlPolicy) -> Result<ValidSurface, Vec<ValidationIssue>>` is pure and synchronous:

1. JSON Schema per component against `#/components/<type>` (`jsonschema` 0.49, draft 2020-12). Unknown `component` rejected.
2. Structure: unique ids, exactly one `root`, every child reference resolves, no cycles, depth ≤ 12, ≤ 100 components.
3. Limits: serialized surface ≤ 64 KiB; string literal ≤ 4 KiB; `fallback_text` 1..=2000 chars.
4. Regex safety: each `regex` pattern ≤ 200 chars, compiles under the Rust `regex` crate (1.12, promoted to `[workspace.dependencies]`), and passes a **backtracking-safety check**: no quantified group whose body itself contains a quantifier (`(a+)+`, `(a*)*`, `(\w+\s?)*`) and no quantified group containing an alternation (`(a|aa)+`). The check walks `regex-syntax`'s AST, so it is not a string heuristic. The Rust engine is linear-time, but the client evaluates with backtracking Dart/JS `RegExp`, so compiling under Rust alone proves nothing about the client.
5. URL policy (D7) on every literal URL **and** on initial `data_model` values at paths bound to `Image.url` or an `openUrl` argument.
6. The `@` namespace contains only directives the spec allows.

`UrlPolicy` is plain data: `{ allowed_attachment_ids: HashSet<Uuid>, image_hosts: Option<Vec<String>> }`. The handler (async) pre-collects every `/api/attachments/{id}` reference from the draft, resolves ownership against the attachment store for the current conversation, and passes the owned set in. The validator stays pure.

Issues use A2UI's error shape (`code`, `path`, `message`) and come back as `ToolOutput::error`. The client validates again through `a2ui_core` and reports renderer `error`s only to logs.

### D4. Gating: rich-client tools need an interactive web turn

`ToolHandler` gains `fn requires_rich_client(&self) -> bool { false }`, mirrored on `ToolSpec` (adding the field touches all 22 `ToolSpec { … }` literals in 13 files). `filter_tool_specs` takes the turn's `ExecutionContext` and drops rich-client specs unless

```
ctx.interactive && ctx.interface == Interface::Web && depth == 0
```

`ctx.interactive` is the single interactivity flag from `tool-approval-prompts` PR 6: the shared runner sets it for `send_message`, `edit_message` and `regenerate_message`; it is false for voice and `quick-message`. The `Interface::Web` term is still needed because CLI REPL turns are interactive but cannot render a surface. All three call sites change (`mod.rs:783,1186`, `subagent.rs:142`); the subagent site always passes depth ≥ 1. `crates/mcp-server/src/server.rs:61,151` drops rich-client tools from `tools/list`.

If the tool still executes outside that context (a stale skill, a test), it returns success with "This interface cannot display UI; present the following to the user as text: <fallback_text>", creates nothing and emits no event.

_Alternatives_:

- `interface == Web && depth == 0` alone. Rejected: `quick-message` (Siri intent) is `Interface::Web` but returns only `turn.answer` as JSON. The surface would never be shown, and a `wait_for_action` call would block until expiry.
- A separate `rich_ui` turn flag or an `X-Client-Capabilities: a2ui-v1` header. Rejected as a second, overlapping flag: every turn that is interactive on web is a streaming chat turn whose client renders surfaces. Stale app builds are handled by registration order (tasks: the tool is registered only after the client PRs) rather than per-request negotiation.
- Register the tool only in web runtimes. Rejected: `SpaceRuntimes` are shared across interfaces (`crates/web-ui/src/lib.rs:326`).

### D5. Event: `surface_update`

`OrchestratorEvent::SurfaceUpdate { surface_id, tool_call_id: Option<String>, anchor_message_id, anchor_call_index, title, fallback_text, status, messages: Vec<Value> }`. Projectors handle it explicitly: SSE → `surface_update` (snake_case outer keys, A2UI messages untouched in `messages`). CLI → prints `fallback_text`; A2A → a text part with `fallback_text`. Because of D4 gating, the CLI and A2A arms are unreachable in practice and exist so projection stays total (no `_` arm).

**Hook placement**: the surface hook runs in `finalize_tool` between `exec_result` and observation construction (`dispatch.rs:229`), not next to `AudioReady` (`:295`). It **takes** `output.data` (`Option::take`) before tool-result-renderers' forwarding and persistence (its S2/S3), so the surface is not duplicated into the `tool_result` frame or `messages.tool_data_json`, and is not silently dropped by their 32 KiB cap. The OTel span's `duration_ms` is recorded before any wait.

A status change (`active → submitted`) re-emits `surface_update` with empty `messages`.

**Persistence beyond a disconnect**: the runner (`stream_turn_to_sse`, message-branching PR 5) is changed so that a failed client send stops only the client forwarding. It keeps draining `event_rx`, persisting each frame with its seq and broadcasting it to live tails until the turn ends. Without this, a reload during a `wait_for_action` wait would lose the later `submitted` status frame and all of the answer (`messages.rs:351–362`). This helps every sibling whose frames arrive late (interactions, suggestions).

### D6. Persistence: `ui_surfaces` in space.db

Columns: `id` (uuid), `conversation_id` (FK, cascade), `turn`, `anchor_message_id` (the tool-call message), `anchor_call_index`, `tool_call_id?`, `title`, `fallback_text`, `catalog_id`, `protocol_version`, `components_json`, `data_model_json`, `send_data_model`, `status` (`active|submitted`), `action_json?`, `action_message_id?`, `created_at`, `updated_at`. `SurfaceStore` trait with `InMemorySurfaceStore` and `SqliteSurfaceStore` (ADR-0009). Migration `NNN_ui_surfaces.sql`; the number comes from the pre-allocated migration plan shared by the sibling changes and is registered in every migration list in `crates/storage/src/lib.rs`.

`persist_tool_calls` (`dispatch.rs:88`) returns the tool-call message id so the hook can anchor the surface to `(anchor_message_id, anchor_call_index)`.

`ConversationDetail` gains `surfaces: Vec<SurfaceSummary>` (attachments-map preload pattern). Each carries a synthesized single `createSurface` with the current components and data model, the anchor fields, `tool_call_id`, `status`, `submitted_action` and `action_message_id`. Surfaces are filtered to those whose `anchor_message_id` is among the returned messages, so once message-branching returns only the active path, surfaces of abandoned branches drop out with no extra code.

**Matching to chips** is exact, never by turn order:

1. by `tool_call_id`, which tool-result-renderers S4/C1 expose on `ToolCallSummary`/`ToolCallRecord`;
2. for providers with no ids (Ollama), by `(message id, index in tool_calls)`.

Validation-failure calls and update calls create no surface row, so they never steal a later surface; the client shows failed `render-surface` calls as ordinary error chips and hides successful ones (their effect is visible in the surface).

_Alternative_: rebuild from `tool_calls_json` arguments. Rejected, because arguments are pre-validation and miss later updates and submissions.

### D7. Security

- No code: only catalog components and functions.
- URLs: `Image.url` must be `https:` or a same-origin `/api/attachments/{id}` owned by the same conversation. `openUrl` targets must be `https:` or `mailto:`. Literal and initial bound values are checked on the server (D3). Values that change later through user input are checked by the client's `UrlPolicy` on resolution.
- `[ui] image_hosts` (new `UiConfig` section on `OrgConfig`, `crates/core/src/types/agent.rs:45`): when set, `https:` image hosts must match the list. Default: unset (any `https:` host). See open question 1.
- Attachment images: the client overrides the catalog's `Image` with an app implementation that loads `/api/attachments/{id}` through the authenticated HTTP client (bearer + space headers, as `message_attachments.dart:196` does) and applies `UrlPolicy`. The stock Image has no header hook.
- `openUrl` goes through the app's `url_launcher` callback, only from a user gesture.
- Regex evaluation: the server rejects backtracking-prone patterns (D3). The client also caps the input it passes to `regex` at 1,000 characters and treats longer input as failing the check.
- `Icon.svgPath` is path data only, capped at 4 KiB.
- Actions are accepted only from callers that pass `caller_can_post` (`messages.rs:44`), for `active` surfaces of that conversation, with an action `name` declared by a Button in that surface.

### D8. Two action routes, one surface state machine

```
active ──in-turn interaction resolved──────────────▶ submitted
active ──post-turn surface_action (no turn in flight)▶ submitted
submitted ──post-turn turn fails to start───────────▶ active (rolled back)
active ──interaction expired / cancelled / declined─▶ active
```

**In-turn** (`wait_for_action: true`):

1. The hook persists the surface and emits `surface_update` (status `active`).
2. It then calls `broker.request(kind: "surface_action", request: {surface_id, tool_call_id})`. The broker inserts the row, emits `interaction_requested{kind: surface_action, surface_id, …}` and awaits. No broker split is needed: the client attaches the interaction to the surface by `surface_id`.
3. `surface_action` is registered through tool-approval's `InteractionKindHandler` registry:
   - `validate_response`: body must be `{kind: "surface_action", action, data_model?}`; the surface is `active` and `action.name` is in `declared_action_names()`; otherwise 400.
   - `observation`: `{"action": …, "data_model": …}` JSON (data model only when `send_data_model`).
   - `renders_card = false`: the client shows no interaction card; the `SurfaceFrame` is the card and enters `EntryState.awaitingInput`.
   - `progress_label`: "Waiting for your input"; `push_copy`: "Your input is needed".
   - Expiry uses the same `[permissions] interaction_timeout_secs`.
4. Outcomes: `resolved` → `mark_submitted`, status `surface_update`, observation. If `mark_submitted` loses (the surface was submitted another way in a race), the observation says the user already submitted it in a new message. `decline` (generic) → observation "the user dismissed the form"; surface stays `active`. `expired` → "the user has not answered yet"; stays `active`. `cancelled` (turn cancel) → stays `active`.
5. A respond that gets `410` (expired/cancelled) makes the client drop the interaction and fall back to the post-turn route.

**Post-turn**: `SendMessageRequest` gains `surface_action: Option<SurfaceActionRequest>` (`surface_id`, `name`, `source_component_id`, `context`, `data_model?`). With it, `message` must be empty. The handler, in order:

1. Validate ownership, `active`, declared name; `context` + `data_model` serialized ≤ 64 KiB (else 413).
2. If `conversation_turn_in_flight` (message-branching D7) → 409. A turn waiting on this surface's interaction is in flight, so this one guard also closes the race between the two routes; the client queues the action like any message sent during a turn (`chat-message-queue`).
3. `mark_submitted(surface_id, action, action_message_id = pre-assigned user_message_id)`: one compare-and-set (`status = 'active'`) that stores the action and the link. A loser gets 409.
4. Persist the user message with **server-built** visible content `Submitted “<title>”`, and start the turn through the runner with a lead frame: a status `surface_update` written right after `run_started`.
5. If the turn fails to start (submit error before `run_started`), `SurfaceStore::reopen(surface_id, action_message_id)` rolls the status back to `active` (CAS on the same link) and the handler returns the error. The surface stays answerable.

**What the LLM sees**: `prepare_history` builds a `SurfaceActionMap` (`message_id → block`) from `ui_surfaces` rows with an `action_message_id`, the same way `build_attachment_map` works (`mod.rs:1586`). `messages_to_chat_history` prepends `[surface_action surface_id=… name=…]` plus the context and data model JSON to those user messages, including the current one. The stored content stays the visible text, and every later rebuild re-synthesizes the block. The model-authored `userMessage` is not shown as user speech; the client renders the bubble with a "Submitted via form" marker using `SurfaceSummary.action_message_id`.

_Alternatives_ for storage: (a) store the block in `messages.content` and strip it in the DTO and client. Rejected: every consumer must strip it, and it puts surface metadata into the contended `messages` lane. (b) A new `messages` column. Rejected for the same lane reason.

_Alternative_ for the route race: let a post-turn action resolve a pending interaction. Rejected: it would need a cross-store transaction between `ui_surfaces` and `interactions`; the in-flight guard already serializes the routes, and the client routes in-turn answers to the respond endpoint anyway.

Surfaces are **single-shot**: once submitted they are read-only and show the submitted values.

### D9. Where the renderer lives

`a2ui_flutter` is not on the `assistant_ui` allowlist (pinned by chat-element-library PR 1 as `{flutter, flutter_smooth_markdown, material_ui}`, plus any entry `chat-a11y-and-code-blocks` adds) and pulls in `video_player`. So:

- `assistant_ui` gets `SurfaceRecord` (pure data), `TimelineEntryType.surface` (the enum moves into the package in chat-element-library PR 4) and `SurfaceFrame`, rendered in the timeline entry shell's `body` slot (chat-element-library PR 5). The frame takes `title`, `status`, `fallbackText`, an error flag, a pending flag, an `awaitingInput` flag (mapped to `EntryState.awaitingInput`) and a `Widget? content` slot. Stories and goldens use a placeholder child.
- `app/lib/features/chat/a2ui/` gets `A2uiSurfaceView` (owns one `MessageProcessor`), `assistantCatalog()` (from `BasicComponents` under our id, with the authenticated `Image` override and `openUrl` wrapped in `UrlPolicy`) and the action dispatch callback. Goldens live in `app/test/`.
- **The allowlist is not amended.**

### D10. Theming

v1.0 has no surface theme, so styling comes from the host. `a2ui_flutter` renders framework-Material widgets; the app's `MaterialUiCompatibilityBridge` publishes the framework `ThemeData` (`legacy_theme.dart`), so components inherit `assistantLightTheme()`/`assistantDarkTheme()`. `A2uiSurfaceView` wraps the surface in a `Theme` override for `AssistantRadius`/`AssistantSpacing`. Goldens cover light and dark.

**Accepted deviation**: the repo's adaptive-widgets rule (`openspec/config.yaml`, Cupertino on iOS/macOS) is not applied inside surfaces. Buttons, fields and the `DateTimeInput` picker are Material on every platform. Overriding them would mean re-implementing most of the catalog and tracking a 0.x package's internals. The surface chrome (`SurfaceFrame`) follows the app rules. See open question 3.

## Risks

- **0.x dependency churn.** `a2ui_flutter` went 0.1→0.5 in 20 days. Exact pins, one adapter folder, a contract test that runs the shared fixture through `MessageProcessor`.
- **Material split**: if `a2ui_flutter` stops honouring the bridged `ThemeData`, theming breaks visibly. Goldens catch it.
- **Prompt tokens**: the inlined component schemas are large. Trimmed catalog, no `instructions`, a spec-size test (budget 6 KiB).
- **Strict tool schemas** (OpenAI strict mode) reject `oneOf`. `params_schema` keeps `components` as `array<object>` with a `component` enum; full validation is server-side.
- **Sibling dependencies**: gating needs tool-approval PR 6; the hook lands on tool-result-renderers' `finalize_tool`; the post-turn route needs message-branching PR 4/5/7; the in-turn route needs tool-approval PR 7/8. Server PRs that need none of these (types, validation, storage, event) start immediately.
- **Contended files**: `messages.rs` runner, `stream_event.dart`/`api_client.dart` sealed events, `conversations.rs` detail. This change edits the runner only after message-branching PR 5, adds one `StreamEvent` subclass and updates every exhaustive switch over it, and adds `surfaces` as an additive preload.

## Open questions

1. Image hosts: allow any `https:` host by default, or only own attachments plus the org allowlist? (Arbitrary hosts let a surface beacon to third parties.)
2. Should `openUrl` to a host outside the allowlist ask for confirmation showing the destination?
3. Is Material-styled surface content on iOS/macOS acceptable (D10), or must surfaces be Cupertino there before this ships?
4. The submitted-action bubble reads `Submitted “<title>”` with a "via form" marker. Should it instead show the model-authored `userMessage` (friendlier, but text the user did not write)?
