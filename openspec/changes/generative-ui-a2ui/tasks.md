# Tasks — generative-ui-a2ui

This change ships as a stack of PRs. Each is under ~500 changed lines, starts with a failing test and leaves `main` shippable. Server-only PRs that need neither the client nor a sibling come first.

**Product decisions (final, from `openspec/chat-ui-parity-decisions.md`)**: any `https:` image host by default; `openUrl` outside the org link allowlist asks first and shows the destination (PR 7b, 14b); a surface is answerable until the next user turn (PR 8, 9, 11, 13, 15, 16, 17); the submit bubble reads `Submitted “<title>”` with a "via form" marker (PR 11, 16); Material surface widgets on iOS and macOS are accepted for v1 (PR 14).

**Shippable `main`**: `render-surface` is implemented and tested in PRs 7–12 but is **not registered in `register_builtins` and not advertised in the prompt** until PR 19, which depends on the client stack that renders, reloads, answers and link-confirms surfaces, and on tool-approval-prompts PR 9 (the PR that sets `interactive` on web turns, which lands after tool-approval's client cards, with no feature flag). `wait_for_action` is not in the tool schema until PR 20, which depends on the client that answers surface interactions. Until then the model can never call a tool whose output the user cannot see or act on.

**DB migration numbering**: `ui_surfaces` uses the number pre-allocated to this change (A2UI PR 4) in the cross-change stack plan: `049_ui_surfaces.sql`. Register it in every migration list in `crates/storage/src/lib.rs` (`:472`). This change adds no other migration: the answerable window (D8) is derived, not stored.

**Contended files** (coordinate, do not race):

- `crates/web-ui/src/api/messages.rs` runner: this change edits `stream_turn_to_sse` (message-branching PR 5) only, never the pre-extraction `send_message` loop.
- `app/lib/api/models/stream_event.dart` / `api_client.dart`: PR 15 adds one `StreamEvent` subclass. It lands in its own wave of the sibling stack plan and updates every exhaustive `switch` over `StreamEvent`.
- `crates/web-ui/src/api/conversations.rs`: `surfaces` is an additive preload (attachments-map pattern) and rebases freely.
- `ServerCapabilities` (`/api/capabilities`): PR 7b adds one additive `ui` object.
- `messages` table / `Message` struct: **not touched** by this change.
- Playwright PNG baselines: never merge PNGs by hand. After rebasing, regenerate baselines (`.claude/skills/e2e-testing`) and record the diff in the PR body.

Start immediately: PR 1, 2, 4 (and 6 once 2 lands). PR 13–14 run in parallel with the whole server stack as soon as chat-element-library PR 3–5 land. PR 11 and PR 12 run in parallel. PR 7b and PR 14b form a small server/client pair that runs alongside PR 8–18.

---

## PR 1 — docs(adr): ADR-0019 generative UI via A2UI

Layer: docs
Depends on: none
Parallel with: everything; lands in wave 1 with tool-approval PR 1 (ADR-0017) and message-branching PR 0 (ADR-0018)

- [ ] 1.1 Write `docs/adr/adr-0019-generative-ui-a2ui.md`. Record the protocol choice (A2UI v1.0), the `genui` → `a2ui_flutter`/`a2ui_core` succession with versions, tool-built surfaces versus streamed JSONL (D2), server-side validation incl. regex safety (D3), interactive-web gating (D4), anchored persistence outside the event log (D6), the two action routes over the `surface_action` interaction kind (D8), the answerable-until-next-user-turn window (D8), the synthesized LLM block, the renderer living in `app/` (D9), the Material-everywhere styling deviation (D10), and the link-confirmation policy with the `[ui] link_hosts` allowlist and the any-`https:` image-host default with its beacon trade-off (D7). Reference `adr-0009-testability-architecture.md`, `adr-0011-tool-permission-decision-model.md` (by filename: two ADR-0011 files exist), `adr-0017` and `adr-0018`.
- [ ] 1.2 Add ADR-0019 to the ADR index if one exists, and to `docs/glossary.md` (term: _Surface_). Glossary conflicts with ADR-0017/0018 are trivial; rebase.

## PR 2 — feat(a2ui): A2UI v1.0 types and assistant catalog

Layer: server
Depends on: none
Parallel with: 1, 4, 6

- [ ] 2.1 RED: in the new crate `crates/a2ui`, add tests that (a) round-trip the v1.0 `createSurface`, `updateComponents` and `updateDataModel` examples from the spec through serde types with `version: "1.0"`, and (b) assert that the embedded catalog's component names equal `crates/a2ui/catalog/components.txt`. Confirm they fail.
- [ ] 2.2 Scaffold `assistant-a2ui` (workspace deps only: `serde`, `serde_json`, `jsonschema`, `url`, `anyhow`, plus `regex` and `regex-syntax` promoted into `[workspace.dependencies]` at the versions already in `Cargo.lock`; manual-replay lints with `unwrap_used`/`expect_used`/`panic` = deny). Add it to `[workspace.dependencies]`.
- [ ] 2.3 Vendor the trimmed catalog as `catalog/assistant-v1.json`: v1.0 basic components minus Video, AudioPlayer and Modal; `common_types.json` inlined; `instructions` dropped; own `catalogId`. Add `components.txt`. Embed both with `include_str!`.
- [ ] 2.4 Implement the message types and `SurfaceMessages::create(draft, surface_id)` / `::update(base, patch)` builders (no `deleteSurface` builder). Tests GREEN.
- [ ] 2.5 RED→GREEN: per-component JSON Schema validation via `jsonschema` 0.49 against `#/components/<type>`. Cases: unknown type, missing required prop, wrong enum, valid Text.
- [ ] 2.6 `cargo clippy -p assistant-a2ui -D warnings`; `cargo machete --with-metadata`.

## PR 3 — feat(a2ui): structural validation, limits, regex safety and URL policy

Layer: server
Depends on: PR 2
Parallel with: 4, 5, 6

- [ ] 3.1 RED: tests for each scenario in "Server validates every surface", "Regex patterns are safe" and "URL policy": dangling child, duplicate id, missing or extra root, cycle, depth 13, 101 components, 64 KiB overflow, 4 KiB string, unknown `@` key, multiple issues together; regex > 200 chars, not compiling, `(a+)+$`, `(a|aa)+`, `^[0-9]{5}$` accepted; `javascript:`/`data:`/`http:` URLs, `mailto:` allowed, attachment id not in the owned set, host outside `image_hosts`, any `https:` host with no list, an unsafe initial `data_model` value at a path bound to `Image.url` and to an `openUrl` argument.
- [ ] 3.2 Implement `validate(&SurfaceDraft, &UrlPolicy) -> Result<ValidSurface, Vec<ValidationIssue>>` (pure, sync). `UrlPolicy { allowed_attachment_ids: HashSet<Uuid>, image_hosts: Option<Vec<String>> }`. Add `SurfaceDraft::attachment_ids()` so the async caller can resolve ownership first. The regex check walks the `regex-syntax` AST. GREEN.
- [ ] 3.3 RED→GREEN: `ValidSurface::merge(update) -> Result<ValidSurface, Vec<ValidationIssue>>` validates the merged surface (101 components after merge fails; `data_updates` `{path, value}` with `null` deleting).
- [ ] 3.4 RED: `fallback_text` empty and 2001 chars fail at `/fallback_text`; 1 and 2000 pass. GREEN.
- [ ] 3.5 RED→GREEN: `ValidSurface::declared_action_names()` (Button `action.event.name` set). Used by PR 11 and PR 12.

## PR 4 — feat(storage): SurfaceStore with ui_surfaces table

Layer: server
Depends on: none
Parallel with: 1, 2, 3, 6

- [ ] 4.1 RED: contract tests run against both `InMemorySurfaceStore` and `SqliteSurfaceStore`: create (with anchor fields), get, update components/data model, `mark_submitted(id, action, action_message_id)` as a compare-and-set (second call → conflict), `reopen(id, action_message_id)` (only from `submitted` with the same link → `active`, action cleared), `list_for_conversation` ordered by `(turn, created_at)`, `action_message_map(conversation_id)`.
- [ ] 4.2 RED: a **Sqlite-only** test (`StorageLayer::new_in_memory()`) that deleting a conversation removes its `ui_surfaces` rows. The in-memory store is separate from the in-memory conversation store, so cascade is not part of the shared contract.
- [ ] 4.3 Add `migrations/049_ui_surfaces.sql` (columns per design D6, `status` CHECK `IN ('active','submitted')`, FK `ON DELETE CASCADE`, index on `conversation_id`). `049` is the number pre-allocated to this change; register it in every list in `crates/storage/src/lib.rs`. Closure after the next user turn is derived, so there is no third status.
- [ ] 4.4 Implement the trait and both stores. Expose `StorageLayer::surface_store()`. Re-export `InMemorySurfaceStore` through `assistant_test_support::prelude`. GREEN.

## PR 5 — feat(runtime): drop rich-client tools outside interactive web turns

Layer: server
Depends on: tool-approval-prompts PR 6 (`ExecutionContext.interactive`)
Parallel with: 2, 3, 4, 6

- [ ] 5.1 RED: `crates/runtime/src/orchestrator/tests/dispatch.rs`. `filter_tool_specs` drops a `requires_rich_client` spec for `Interface::Slack`; for `Interface::Web` with `interactive = false` (quick-message, voice); for an interactive CLI turn; for Web at depth 1; for `Interface::Scheduler`. It keeps it for an interactive Web turn at depth 0. Add an MCP test in `crates/mcp-server/tests/dispatch.rs` that `tools/list` omits it.
- [ ] 5.2 Add `requires_rich_client()` to `ToolHandler` (`crates/core/src/tool.rs:106`) and `ToolSpec` (`crates/core/src/llm/tool_spec.rs:7`). Update all 22 `ToolSpec { … }` literals in 13 files (`grep -rn 'ToolSpec {' crates`); this is mechanical churn, budget for it in the diff. Extend `filter_tool_specs` (`dispatch.rs:42`) to take the `ExecutionContext` and depth, and update all three call sites (`mod.rs:783`, `mod.rs:1186`, `subagent.rs:142`). Filter in `crates/mcp-server/src/server.rs:61,151`. GREEN.

## PR 6 — feat(runtime): SurfaceUpdate event and projections

Layer: server
Depends on: PR 2
Parallel with: 3, 4, 5

- [x] 6.1 RED: projector tests in `crates/runtime/src/projection/mod.rs`. `SseProjector` maps `SurfaceUpdate` to `surface_update` with the exact snake_case payload (`surface_id`, `tool_call_id?`, `anchor_message_id`, `anchor_call_index`, `title`, `fallback_text`, `status`, `messages`). `CliProjector` prints `fallback_text`. `A2aProjector` (`crates/web-ui/src/a2a/projection.rs:67`) emits a text part.
- [x] 6.2 Add `OrchestratorEvent::SurfaceUpdate` (`stream_event.rs`). Project it in all three projectors and every other exhaustive `match` over `OrchestratorEvent` (no `_` arm). Comment that the CLI/A2A arms exist for totality only. Nothing emits it yet. GREEN.

## PR 7 — feat(tool-executor): render-surface handler and [ui] org settings

Layer: server
Depends on: PR 3, PR 5
Parallel with: 7b, 8 prep, 13, 14

- [x] 7.1 RED: `OrgConfig` parsing tests. `[ui] image_hosts = ["a.example"]` and `[ui] link_hosts = ["docs.example"]` load into `UiConfig`; an absent section yields `None`; an absent key yields an empty list (`image_hosts` empty means any `https:` host, `link_hosts` empty means every `openUrl` asks first); a non-string entry fails to load; hosts are lower-cased on load.
- [x] 7.2 Add `UiConfig { image_hosts, link_hosts }` to `OrgConfig` (`crates/core/src/types/agent.rs:45`). GREEN.
- [x] 7.3 RED: `RenderSurfaceHandler` unit tests with an in-memory attachment store. Valid create returns success with `data = {"surface": ValidSurface}`. Invalid draft returns `ToolOutput::error` with the JSON issue list. An attachment owned by another conversation fails. A host outside `image_hosts` fails. Update without `components`/`data_updates` fails. A context with `interactive = false` or a non-web interface returns the text-fallback observation with no `data`. `params_schema` ≤ 6 KiB, `components` is `array<object>` with a `component` enum, and **no `wait_for_action` property** yet.
- [x] 7.4 Implement `crates/tool-executor/src/builtins/render_surface.rs`: collect attachment ids, resolve ownership through the attachment store, build `UrlPolicy` from `UiConfig`, call the pure validator. `requires_rich_client() == true`. **Do not register it in `register_builtins`** (PR 19 does). GREEN.

## PR 7b — feat(web-ui): expose the link allowlist on /api/capabilities

Layer: server
Depends on: PR 7
Parallel with: 8, 9, 10, 13, 14

- [ ] 7b.1 RED: handler tests for `GET /api/capabilities`. With `[ui] link_hosts = ["docs.example"]` in the selected space's org, the response carries `ui.link_hosts == ["docs.example"]`. With no `[ui]` section it carries an empty list. The list is the selected space's own org, never another org's.
- [ ] 7b.2 Add an additive `ui: UiCapabilities { link_hosts: Vec<String> }` to `ServerCapabilities` (utoipa, snake_case). Do not expose `image_hosts`: the server enforces it, the client has no use for it. GREEN.
- [ ] 7b.3 `make dump-openapi && make generate-flutter-client`; commit `openapi.json`.

## PR 8 — feat(runtime): emit and persist surfaces from the surface hook

Layer: server
Depends on: PR 4, PR 6, PR 7, tool-result-renderers S2 + S3 (the hook lands on their reworked `finalize_tool`)
Parallel with: 10, 13, 14

- [ ] 8.1 RED: orchestrator test with `ScriptedLlmProvider` and the handler registered **in the test's executor only**. Assert one `SurfaceUpdate` with status `active` and the right anchor (`anchor_message_id`, `anchor_call_index`, `tool_call_id`); a row in `InMemorySurfaceStore`; the `tool_result` frame and the persisted tool row carry **no** surface data; a 6th successful call (creates + updates) is rejected; an update to a foreign, non-`active` or earlier-turn `surface_id` errors (the earlier-turn error says the surface is no longer open and the model must create a new one); a validation failure then retry anchors only the retry.
- [ ] 8.2 Make `persist_tool_calls` (`dispatch.rs:88`) return the tool-call message id and thread it with the call index into `finalize_tool`.
- [ ] 8.3 Implement the surface hook in `finalize_tool` between `exec_result` and observation construction (`dispatch.rs:229`). It `take()`s `output.data` before tool-result-renderers' forwarding and persistence, persists or merges through `SurfaceStore` (an update is accepted only for an `active` surface created in the current turn, the model-side form of the answerable window), emits `SurfaceUpdate`, and builds the observation. Record the span `duration_ms` before anything that could wait. Wire `SurfaceStore` into the orchestrator builder. GREEN.
- [ ] 8.4 RED→GREEN: web-ui test that `stream_run_events` replays a persisted `surface_update` frame byte-identically (pattern: `messages.rs` test `stream_run_events_replays_completed_run`).

## PR 9 — feat(web-ui): surfaces in conversation history

Layer: server
Depends on: PR 8, tool-result-renderers S4 (`ToolCallSummary.tool_call_id`)
Parallel with: 10, 11

- [ ] 9.1 RED: handler tests. `GET /api/conversations/{id}` includes `surfaces` with a synthesized `createSurface`, anchor fields, `tool_call_id`, `status`, `answerable`, `submitted_action` and `action_message_id`. A surface whose anchor message is not among the returned messages is omitted. `answerable` is true only for an `active` surface with no user message after its anchor message on the returned (active) path; a later user message, a `submitted` status or a branch switch makes it false. An update made in the surface's own turn shows the latest components at the original anchor.
- [ ] 9.2 Add `SurfaceSummary` (utoipa, `format: uuid`, snake_case, optional fields omitted, `answerable: bool`) and `surfaces` on `ConversationDetail` as an additive preload. Add the shared helper `surface_answerable(path_messages, surface)` in `web-ui`, used here and by PR 11. GREEN.
- [ ] 9.3 `make dump-openapi && make generate-flutter-client`; commit `openapi.json`.

## PR 10 — fix(web-ui): keep persisting run frames after the client disconnects

Layer: server
Depends on: message-branching PR 5 (`stream_turn_to_sse`)
Parallel with: 2–9

- [x] 10.1 RED: runner test with a scripted turn that emits three frames; drop the SSE receiver after the first. Assert all frames and the terminal frame are in the event log with consecutive seq ids and reach a live-tail subscriber.
- [x] 10.2 In `stream_turn_to_sse`, a failed `sse_tx.send` sets a `client_gone` flag and stops forwarding only; the loop keeps draining `event_rx`, persisting and broadcasting until the turn ends (today it `break`s, `messages.rs:351–362`). GREEN. Coordinate with tool-approval PR 7.4 (`abandon_run` before the terminal frame) and follow-up-suggestions PR 3; they edit the same runner after this.

## PR 11 — feat(web-ui): post-turn surface actions

Layer: server
Depends on: PR 8, PR 9, message-branching PR 4 (`conversation_turn_in_flight`), PR 5 (runner), PR 7 (pre-assigned `user_message_id`)
Parallel with: 12

- [ ] 11.1 RED: runtime test for `messages_to_chat_history` with a `SurfaceActionMap`: a linked user message gets the `[surface_action surface_id=… name=…]` block plus context and data model JSON before its text; unlinked messages are unchanged. A two-turn test: the action turn and a later turn both see the block, and the stored content stays `Submitted “<title>”`.
- [ ] 11.2 Add `build_surface_action_map` next to `build_attachment_map` (`mod.rs:1586`) and the map parameter to `messages_to_chat_history` (`history.rs:29`; update `command_registry.rs:276,296` and tests). Apply it to the current user message too. GREEN.
- [ ] 11.3 RED: handler tests. A valid `surface_action` with empty `message` streams `run_started`, then `surface_update{status: submitted}`, and stores `Submitted “<title>”`. Non-empty `message` + action → 400. Foreign surface → 400. Undeclared name → 400. A surface with a later user message on the active path (not answerable) → 410 and the surface is unchanged, checked before the size and in-flight checks. Context + data model > 64 KiB → 413. Turn in flight (incl. one waiting on this surface) → 409. Submitted surface → 409. No write scope → 403. Submit failure before `run_started` → surface back to `active`, action cleared.
- [ ] 11.4 Add `SurfaceActionRequest` and `surface_action` on `SendMessageRequest` (`messages.rs:55`). Implement the order in design D8: validate (ownership, `active`, declared name), answerable check via `surface_answerable` (410), size check (413), in-flight check (409), `mark_submitted` CAS with the pre-assigned `user_message_id`, then the runner with the status frame as a lead frame after `run_started` (add `lead_frames` to `RunOptions`); `reopen` on start failure. GREEN.
- [ ] 11.5 `make dump-openapi && make generate-flutter-client`.

## PR 12 — feat(runtime): surface_action interaction kind for in-turn waits

Layer: server
Depends on: PR 8, PR 10, tool-approval-prompts PR 7 (broker, `InteractionKindHandler` registry) and PR 8 (respond endpoint, extensible `RespondToInteractionRequest`)
Parallel with: 11

- [ ] 12.1 RED: orchestrator tests against the broker with `InMemoryInteractionStore` and a paused clock, the handler registered in-test with `wait_for_action: true` passed directly. The log has `surface_update` before `interaction_requested{kind: surface_action, surface_id}`. A resolution with `submit_form` → observation with name and context, surface `submitted`, status frame. `decline` → "dismissed" observation, surface `active`. `expired` → "not answered yet", `active`. Turn cancel → `cancelled`, `active`. A lost `mark_submitted` race → "already submitted in a new message" observation. An interaction cancelled because the user started an edit or regenerate (message-branching auto-cancels the pending interaction; reason `branched`) resolves `cancelled` like a turn cancel: the observation says the user moved on and the surface stays `active` (it then leaves the active path, so it is no longer returned). This PR only asserts the kind handler treats the cancel like any other; message-branching PR 7 produces it.
- [ ] 12.2 RED: respond-endpoint tests: `{kind: "surface_action", action}` → 200; undeclared name → 400; `surface_action` body on a `tool_approval` interaction → 400; turn status is `awaiting_input` during the wait.
- [ ] 12.3 Register the `surface_action` `InteractionKindHandler`: `validate_response` (active + `declared_action_names()`), `observation`, `renders_card = false`, `progress_label` "Waiting for your input", `push_copy` "Your input is needed"; expiry from `[permissions] interaction_timeout_secs`. Add the `surface_action { action, data_model? }` variant to `RespondToInteractionRequest` (turn-interactions MODIFIED delta). Implement the await in the hook. GREEN.
- [ ] 12.4 RED→GREEN: the end-to-end "client disconnects during a wait" scenario over PR 10's runner: drop the SSE client, respond, then `events/stream?since=N` returns the `submitted` status frame and the rest of the turn.
- [ ] 12.5 `make dump-openapi && make generate-flutter-client`.

## PR 13 — feat(assistant_ui): surface model and SurfaceFrame element

Layer: client
Depends on: chat-element-library PR 3 (widgetbook, `kAssistantUiElements`), PR 4 (`TimelineEntryType` in the package), PR 5 (shell `body` slot, `EntryState.awaitingInput`)
Parallel with: all server PRs

- [ ] 13.1 RED: `app/packages/assistant_ui/test/surface_frame_test.dart`. Fallback-only rendering, error flag shows fallback, submitted label with the child in a disabled scope, a non-answerable `active` surface shows the child in a disabled scope with a "Closed" label (no actions fire), pending state, `awaitingInput` entry state. Confirm RED.
- [ ] 13.2 Add `SurfaceRecord` (surfaceId, toolCallId, anchorMessageId, anchorCallIndex, title, fallbackText, status, `answerable`, messages as `List<Map<String, Object?>>`, interactionId, submittedAction) and `TimelineEntryType.surface`. Add `SurfaceFrame`. GREEN.
- [ ] 13.3 Goldens for active, awaiting input, pending, submitted, closed, error and fallback-only, light and dark. Widgetbook use-cases. Register in `kAssistantUiElements` and `ELEMENTS.md`.
- [ ] 13.4 The boundary test passes with the allowlist unchanged (`{flutter, flutter_smooth_markdown, material_ui}` plus any chat-a11y entry).

## PR 14 — feat(app): A2UI renderer adapter

Layer: client
Depends on: PR 13 (fixture-driven; parallel with the server stack)
Parallel with: all server PRs except 7b (which 14b needs)

- [ ] 14.1 RED: `app/test/features/chat/a2ui/assistant_catalog_test.dart`. Catalog component names equal the fixture copied from `crates/a2ui/catalog/components.txt`; a fixture `createSurface` renders Text and a Button through `A2uiSurfaceView`. Confirm RED.
- [ ] 14.2 Add `a2ui_flutter: 0.5.0` and `a2ui_core: 0.9.0` (exact pins) to `app/pubspec.yaml`. Check the macOS `com.apple.security.network.client` entitlement. Implement `assistantCatalog()`. GREEN.
- [ ] 14.3 RED→GREEN: "attachment image renders with auth headers": the `Image` override loads `/api/attachments/{id}` through an injected authenticated client (fake asserts `Authorization` and space headers) and applies `UrlPolicy`.
- [ ] 14.4 `openUrl` is wired to an injected `LinkOpener` seam whose default does nothing; PR 14b supplies the policy-checked, confirming implementation. A test asserts that no `url_launcher` call happens without the seam.
- [ ] 14.5 RED→GREEN: theme goldens (light and dark Card, Button, TextField, ChoicePicker, DateTimeInput) through the material bridge; `AssistantRadius`/`AssistantSpacing` mapping (D10). Material on all platforms, including iOS and macOS, is the accepted v1 deviation (D10); add a widget test that pins the Material `TextField` and button types under `TargetPlatform.iOS` and `TargetPlatform.macOS` overrides so a later change is deliberate.
- [ ] 14.6 RED→GREEN: `A2uiSurfaceView` exposes `onAction` with the A2UI action (plus the data model when `sendDataModel`). Renderer `onError` flips the frame to fallback.

## PR 14b — feat(app): surface URL policy, regex cap and link confirmation

Layer: client
Depends on: PR 14, PR 7b
Parallel with: 15, 16, 17 (disjoint files under `app/lib/features/chat/a2ui/`)

- [ ] 14b.1 RED→GREEN: Dart `UrlPolicy` mirrors the server cases. A bound `data:` image changed by user input renders a placeholder. A `javascript:` `openUrl` is disabled. Any `https:` image host renders (the default; the server enforces `[ui] image_hosts`).
- [ ] 14b.2 RED→GREEN: the `regex` function rejects input over 1,000 chars without evaluating (test with `(a+)+$`-style input on a permissive pattern and a 5,000-char string).
- [ ] 14b.3 RED: widget tests for the confirmation. Tapping an `openUrl` Button whose `https:` host is not in `ui.link_hosts` shows a dialog with the full destination URL and the host emphasised, plus Open and Cancel; Cancel launches nothing; Open launches once through the injected `url_launcher` fake. A host in `ui.link_hosts` (case-insensitive exact match) launches with no dialog. A `mailto:` target always asks and shows the address. A policy-violating link is disabled and never reaches the dialog. The dialog follows the adaptive-widgets rule (Cupertino on iOS and macOS); it is app chrome, not surface content. Confirm RED.
- [ ] 14b.4 Add `uiLinkHostsProvider` reading `ui.link_hosts` from the generated capabilities client for the selected space; a failed or missing capabilities call yields an empty list, so the client asks for every link. Implement the confirming `LinkOpener` and install it in `assistantCatalog()`. GREEN.

## PR 15 — feat(app): stream surface_update into the live timeline

Layer: client
Depends on: PR 6, PR 14, tool-result-renderers C1, chat-stop-cancels-turn PR 4 + 5 (client stream-loop lane order; follow-up-suggestions PR 7 and message-branching PR 12 rebase on top)
Parallel with: 16 prep

- [ ] 15.1 RED: parser tests in `app/test/api/`. `surface_update` yields `SurfaceUpdateEvent`; malformed payload skipped; an unknown event still ignored.
- [ ] 15.2 Add `SurfaceUpdateEvent` to the sealed `StreamEvent` (`stream_event.dart:5`) and the parser (`api_client.dart:780`). Update every exhaustive `switch` over `StreamEvent`. GREEN.
- [ ] 15.3 RED: `ChatNotifier` tests. A live `SurfaceUpdateEvent` adds a surface entry; a status event updates it in place; sending a user message (send, edit, regenerate, or a queued message when it is actually sent) flips every `active` surface to non-answerable in place, which the frame shows as closed; an `ok` `render-surface` tool result renders no chip, an `error` one renders an error chip; replay from `since=0` yields an identical timeline.
- [ ] 15.4 Handle the event in `_applyTurnEvent` (`chat_provider.dart:111`). Render surface entries via `SurfaceFrame` + `A2uiSurfaceView` in the timeline adapter. GREEN.

## PR 16 — feat(app): surfaces in conversation history

Layer: client
Depends on: PR 9, PR 15, tool-result-renderers C1 (`ToolCallRecord.toolCallId`)
Parallel with: 17 prep

- [ ] 16.1 RED: `chatMessagesFromHistory` (`chat_history_mapping.dart`) tests. A surface replaces the chip whose `tool_call_id` matches; without ids it is placed by `anchor_message_id` + `anchor_call_index`; a failed call before a retry stays an error chip; submitted values show; `answerable` from the server decides whether an `active` surface is operable or closed; the user message named by `action_message_id` carries the "Submitted via form" marker while its text is the server-built `Submitted “<title>”` (the model-authored `userMessage` is never shown as user speech). No turn-order matching.
- [ ] 16.2 Implement. GREEN.

## PR 17 — feat(app): post-turn surface actions

Layer: client
Depends on: PR 11, PR 16
Parallel with: 18 prep

- [ ] 17.1 RED: `ApiClient.streamMessages` tests: a `surfaceAction` parameter posts `surface_action` with an empty `message`; a `DioException` with a response yields an `ErrorEvent` carrying `statusCode`.
- [ ] 17.2 Extend `streamMessages` (`api_client.dart:265`) and `ErrorEvent`. GREEN.
- [ ] 17.3 RED: notifier tests. A post-turn action goes through the message queue when a turn is running, shows the pending state, flips to `submitted` on the status `surface_update` after `run_started`, on a 409 `ErrorEvent` marks the surface submitted without retry, and on a 410 `ErrorEvent` marks it closed (a later user turn started) without retry.
- [ ] 17.4 Implement. GREEN.

## PR 18 — feat(app): answer in-turn surface interactions

Layer: client
Depends on: PR 12, PR 17, tool-approval-prompts PR 15 (interaction stream events, generated client) and PR 16 (interaction state, `renders_card = false` kinds)
Parallel with: 19

- [ ] 18.1 RED: notifier tests. `interaction_requested{kind: surface_action, surface_id}` attaches the interaction to that surface, sets `awaitingInput` and renders **no** interaction card; the progress label is "Waiting for your input". An action posts `{kind: "surface_action", action, data_model?}` to the respond endpoint and does not start a message. A 410 drops the interaction and sends on the post-turn route. After `interaction_resolved` (cancelled/expired) or turn end, actions use the post-turn route.
- [ ] 18.2 Implement on top of tool-approval's interaction state. GREEN.

## PR 19 — feat(tool-executor): register render-surface and advertise it on web

Layer: server
Depends on: PR 5, PR 8, PR 14b, PR 15, PR 16, PR 17, tool-approval-prompts PR 9 (clients that render, reload, answer and link-confirm surfaces are on `main`; tool-approval PR 9 sets `interactive` on web turns and itself lands after tool-approval's client cards, with no feature flag)
Parallel with: 18

- [ ] 19.1 RED: `register_builtins` includes `render-surface`; `output_capabilities` (`prompt.rs:157`) for an interactive Web turn mentions it; other interfaces and non-interactive Web turns do not.
- [ ] 19.2 Register the handler and add the prompt paragraph. GREEN.

## PR 20 — feat(tool-executor): offer wait_for_action

Layer: server
Depends on: PR 12, PR 18, PR 19
Parallel with: —

- [ ] 20.1 RED: `params_schema` contains `wait_for_action` (boolean, default false) and stays ≤ 6 KiB; the prompt paragraph explains when to wait.
- [ ] 20.2 Add the property and prompt text. GREEN.

## PR 21 — docs(chat): generative UI docs, baselines and final checks

Layer: both
Depends on: PR 19 (PR 20 when merged)
Parallel with: —

- [ ] 21.1 Write `docs/chat/generative-ui.md`: what surfaces are, where they appear (interactive web chat only), text fallback elsewhere, the URL policy (any `https:` image host by default, with the third-party-request trade-off and how `[ui] image_hosts` narrows it; `openUrl` asks before opening anything outside `[ui] link_hosts`, showing the destination), that a surface is answerable until the next user turn and then reads "Closed", the `Submitted “<title>”` bubble with its "via form" marker, and the Material-only styling on iOS and macOS. Add `[ui] image_hosts` and `[ui] link_hosts` to `docs/operations/configuration.md`.
- [ ] 21.2 Add a Playwright spec with a scripted surface turn, covering the external-link confirmation and a closed surface after a follow-up message. Regenerate screenshot baselines after rebasing (never merge PNGs by hand) and record the visual diff in the PR body.
- [ ] 21.3 `make lint && make format && make test`.
- [ ] 21.4 `make lint-flutter && make test-flutter`.
- [ ] 21.5 Archive after tool-approval-prompts (this change MODIFIES its `turn-interactions` capability): `openspec archive generative-ui-a2ui`.
