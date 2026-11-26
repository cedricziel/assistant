# ADR 0019: Generative UI Through A2UI Surfaces

**Status**: Accepted
**Date**: 2026-11-26
**Builds on**: [ADR-0009](adr-0009-testability-architecture.md), [ADR-0017](adr-0017-turn-interactions.md), [ADR-0018](adr-0018-conversation-message-tree.md)
**Related**: [adr-0011-tool-permission-decision-model.md](adr-0011-tool-permission-decision-model.md) (two ADRs carry the number 0011; this is the tool-permission one)

## Context

A Persona can answer in Markdown, Mermaid and SVG, and nothing else. The
output-capabilities text for the web interface lists exactly those
(`output_capabilities` in `crates/runtime/src/orchestrator/prompt.rs`). When a
Persona needs structured input it asks in prose and parses the reply. Chat UIs
such as assistant-ui and tool-ui let the model show a form, a card or a choice
list that the user operates directly. We want the same in the web chat without
letting the model send executable code.

Facts about the current code that shape the design:

- **Tool specs are filtered per turn only for provider-hosted tools.**
  `filter_tool_specs` in `crates/runtime/src/orchestrator/dispatch.rs` drops
  `web-search` and `web-fetch` when the provider has them natively. It is called
  from three places: the two turn loops in `orchestrator/mod.rs` and
  `orchestrator/subagent.rs`.
- **`ExecutionContext.interactive` exists but is false for every web turn.**
  The turn loops set it to `false` or to `matches!(interface, Interface::Cli)`.
  `quick-message` (`POST /api/quick-message`, used by the Siri intent in
  `app/packages/AssistantIntents`) runs a synchronous turn as `Interface::Web`
  and returns only the answer text. `Interface::Web` therefore does not mean
  "a client that can show a form".
- **The tool-call message is persisted before the tool runs**, and
  `persist_tool_calls` returns nothing, so nothing downstream knows the message
  id. `ToolCallItem.id` is `Option<String>`: Ollama calls have no id
  (`crates/core/src/llm/types.rs`).
- **A tool's `ToolOutput.data` does not reach the client.** `finalize_tool`
  builds the observation string and emits `ToolResult` with a result truncated
  to 512 characters.
- **Projection is total.** `SseProjector`, `CliProjector` and the A2A projector
  have no `_` arm, so a new `OrchestratorEvent` must be handled in all three.
- **The streamed-turn loop stops on the first failed client send.** In
  `crates/web-ui/src/api/messages.rs`, a failed `sse_tx.send` breaks the loop,
  so later frames are neither persisted nor broadcast.
- **The event log is ephemeral.** Rows expire after 24 hours
  (`DEFAULT_EVENT_TTL_HOURS` in `crates/storage/src/conversation_events.rs`),
  so conversation history cannot be rebuilt from it.
- **`SendMessageRequest.message` is required and must be non-empty**, and
  `submit_turn` persists exactly the string it is given as the user message.
  History replay uses the stored content, with attachments joined from a side
  map keyed by message id (`build_attachment_map`, `messages_to_chat_history`).
- **The client ignores unknown SSE events.** The stream parser in
  `app/lib/api/api_client.dart` is an `if/else` chain with no final `else`, and
  `_applyTurnEvent` in `chat_provider.dart` ends in `default: return false`.
  A server that adds an event does not break an older app.
- **Attachment bytes need an authenticated request.**
  `GET /api/attachments/{id}` requires the bearer token, is space-scoped
  through the `X-Space-Id` header, and serves a file only through a conversation
  the caller owns. The app fetches them by hand.

External facts, checked on 2026-11-26:

- A2UI v1.0 is the current specification (a2ui.org, "A2UI v1.0 is the current
  specification"). It removed theme properties from `createSurface` and lets
  functions marked `requiresUserActivation`, such as `openUrl`, run only from a
  user gesture. v0.9.x is superseded.
- `genui` 0.10.4 on pub.dev (2026-09-29) is marked discontinued with
  `a2ui_flutter` as its replacement. `a2ui_flutter` 0.5.0 (2026-11-23, Dart
  3.13 or later) depends on `a2ui_core` ^0.9.0, `path_parsing` and
  `video_player`. `a2ui_core` 0.9.0 is pure Dart. `a2ui_flutter` went from 0.1.0
  to 0.5.0 in 19 days. Both are 0.x.
- `a2ui_core` evaluates the `regex` function with Dart `RegExp`, which
  backtracks, and compiles to JavaScript `RegExp` on the web. The Rust `regex`
  crate is linear-time and accepts `(a+)+$` without complaint.

## Decision

**The model calls a `render-surface` tool with a flat component list. The server
validates it against a trimmed A2UI v1.0 catalog, stores it, and streams the
protocol messages. The web client renders it with the Flutter A2UI packages. The
user's answer returns as an interaction response or as a new user turn.**

### 1. Protocol: A2UI v1.0, our own catalog id, `a2ui_flutter` on the client

We target v1.0 only. The Flutter side uses `a2ui_flutter` and `a2ui_core`, the
successors of `genui`, with exact version pins in `app/pubspec.yaml`. Adopting
the discontinued `genui` would mean starting on a package that already has a
replacement.

The assistant catalog keeps the v1.0 basic-catalog component shapes under its
own `catalogId`. It has fifteen components (Text, Image, Icon, Row, Column,
List, Card, Tabs, Divider, Button, CheckBox, TextField, DateTimeInput,
ChoicePicker, Slider) and the functions `required`, `regex`, `length`,
`numeric`, `email`, `formatString`, `formatNumber`, `formatCurrency`,
`formatDate`, `pluralize`, `and`, `or`, `not` and `openUrl`. Video, AudioPlayer
and Modal are out. The catalog is vendored with `common_types.json` inlined and
the `instructions` field dropped, in a new pure crate `assistant-a2ui`. A test
on each side asserts that its component set equals
`crates/a2ui/catalog/components.txt`.

_Rejected_: the basic catalog id as is (it admits Video, AudioPlayer and Modal);
v0.9 (superseded, and `openUrl` can fire without a gesture).

### 2. Tool-built surfaces, not streamed JSONL

`render-surface` has two modes. Create takes `title`, `fallback_text` and
`components`, with optional `data_model` and `send_data_model` (default true).
Update takes a `surface_id` and at least one of `components` (upserted by id)
or `data_updates` (JSON Pointer path and value; `null` deletes). The server
assigns the `surface_id` and builds the `createSurface`, `updateComponents` and
`updateDataModel` messages. There is no close or delete mode: a surface stays
`active` until it is submitted. A model may update only a surface created in
the current turn. Each successful call counts toward a cap of five per turn.
`fallback_text` is required so every non-rich consumer has something to show.

_Rejected_: the model streams raw A2UI JSONL in its answer text (nothing is
validated before the client sees it, two channels are mixed, and every provider
needs its own parser); the model authors full envelopes (more tokens, more
failure modes, model-chosen ids). The cost of the chosen design is that a
surface appears all at once.

### 3. Validation is server-side, pure and structured

`assistant_a2ui::validate` takes a draft and a `UrlPolicy` and returns either a
valid surface or a list of issues in A2UI's error shape (`code`, `path`,
`message`). It is synchronous and does no I/O, so the handler resolves anything
that needs a store first and passes plain data in (the ownership of attachment
references, in `UrlPolicy`). On failure the tool returns `ToolOutput::error`
with the issues, so the model can correct itself. The checks:

1. JSON Schema per component, from `jsonschema` (already a workspace
   dependency, used by `tool-executor`), and rejection of unknown components.
2. Structure: unique ids, exactly one `root`, every child reference resolves, no
   cycles, depth of at most 12, at most 100 components.
3. Limits: serialized surface of at most 64 KiB, string literals of at most
   4 KiB, `fallback_text` of 1 to 2000 characters.
4. **Regex safety.** Each `regex` pattern must be at most 200 characters, compile
   under the Rust `regex` crate, and pass a backtracking-safety check on the
   `regex-syntax` AST: no quantified group whose body contains a quantifier
   (`(a+)+`, `(\w+\s?)*`) and no quantified group containing an alternation
   (`(a|aa)+`). Compiling under Rust proves nothing about the client, which
   evaluates with a backtracking engine. This is a bound on the patterns we
   accept, not a proof of linear time on the client; the client also caps the
   input it passes to `regex` at 1,000 characters (see section 7).
5. The URL policy of section 7, applied to every literal URL and to initial
   data-model values bound to `Image.url` or an `openUrl` argument.
6. The `@` namespace holds only directives the specification defines.

`regex` and `regex-syntax` are not in `[workspace.dependencies]` yet; the crate
adds them at the versions already locked in `Cargo.lock`. The client validates
again through `a2ui_core` and reports renderer errors to logs only.

### 4. Gating: an interactive web turn at depth 0

`ToolHandler` gains `requires_rich_client()`, defaulting to `false`, mirrored on
`ToolSpec`. `filter_tool_specs` takes the turn's `ExecutionContext` and drops
such specs unless `ctx.interactive && ctx.interface == Interface::Web &&
depth == 0`. All three call sites change. The MCP server's `tool_list` drops
them too.

`interactive` is the single interactivity flag that
[ADR-0017](adr-0017-turn-interactions.md) sets for streamed web turns (`send`,
`edit` and `regenerate`). It stays false for voice and `quick-message`. The
`Interface::Web` term is still needed because a CLI REPL turn is interactive
but cannot render a surface. If the tool is somehow called outside that context
(a stale skill, a test), it returns success with the `fallback_text` as text to
present, creates nothing and emits no event.

_Rejected_: `Interface::Web && depth == 0` alone (a `quick-message` turn would
create a surface nobody sees, and a waiting surface would block until expiry); a
second `rich_ui` flag or a client-capabilities header (a second flag that
overlaps the first, since every interactive web turn is a streaming chat turn
whose client renders surfaces); registering the tool only in web runtimes
(`SpaceRuntimes` are shared across interfaces).

Stale app builds are handled by rollout order, not negotiation: the tool is not
registered, and not mentioned in the prompt, until the client PRs that render,
reload and answer surfaces have shipped. Until then the model cannot call a tool
whose result the user cannot see or act on.

### 5. Events and persistence

**Event.** A new `OrchestratorEvent::SurfaceUpdate` carries the surface id, the
anchor (see below), `title`, `fallback_text`, `status` and the A2UI messages. The
SSE projector emits `surface_update`, which is stored in the event log and so
replays on reconnect. The CLI and A2A projectors handle the variant explicitly
(print or send `fallback_text`) so projection stays total. Section 4 makes both
arms unreachable in practice.

**Hook.** The surface hook runs in `finalize_tool`, between the execution result
and the observation, and **takes** `ToolOutput.data` so the surface is not
copied into the `tool_result` frame or the stored tool data. `persist_tool_calls`
returns the tool-call message id so the hook can anchor the surface.

**Anchored persistence outside the event log.** Surfaces live in a new
`ui_surfaces` table in `space.db` (migration `049_ui_surfaces`), behind a
`SurfaceStore` trait with an `InMemorySurfaceStore` and a `SqliteSurfaceStore`,
the trait-pair pattern of [ADR-0009](adr-0009-testability-architecture.md). A
row holds the current components and data model, `status` (`active` or
`submitted`), the submitted action, and its anchor: the `tool_call_id`, or the
tool-call message id plus the call index. The event log cannot serve history
because it expires after 24 hours, and the tool-call arguments cannot either:
they are pre-validation and miss later updates and submissions.
`ConversationDetail` gains `surfaces`, each entry holding a synthesized single
`createSurface` for the current state. The client matches a surface to its tool
chip by `tool_call_id`, or for providers without ids by (message id, index in
`tool_calls`), never by turn order. Calls that fail validation or only update a
surface create no row, so they cannot take another surface's place. Surfaces
whose anchor message is not on the returned path drop out, which gives branches
the right behaviour once [ADR-0018](adr-0018-conversation-message-tree.md) returns
only the active path.

The change does not touch the `messages` table or the `Message` struct.

**Frames survive a disconnect.** The streamed-turn runner is changed so that a
failed client send stops only the forwarding to that client. It keeps draining
the event channel, persisting and broadcasting each frame until the turn ends.
Without this, a reload during a `wait_for_action` wait would lose the later
`submitted` status frame and the answer.

### 6. Two action routes over one surface state machine

```
active ──in-turn interaction resolved──────────────▶ submitted
active ──post-turn surface_action (no turn in flight)▶ submitted
submitted ──post-turn turn fails to start───────────▶ active (rolled back)
active ──interaction expired / cancelled / declined─▶ active
active ──a later user message on the active path────▶ closed (derived, not stored)
```

**In-turn.** With `wait_for_action: true` the hook persists the surface, emits
`surface_update`, then asks the interaction broker of
[ADR-0017](adr-0017-turn-interactions.md) for a new interaction kind,
`surface_action`. The kind renders no card of its own: the surface is the card,
and the client attaches the interaction to it by `surface_id`. The kind's
handler accepts only an action whose name a Button in that surface declares. A
resolved interaction marks the surface `submitted` and returns the action (and
the data model, when `send_data_model` is set) as the observation. Decline,
expiry and cancel leave the surface `active`, with matching observations for the
model. The expiry is the interaction timeout ADR-0017 defines.

**Post-turn.** `SendMessageRequest` gains an optional `surface_action` (surface
id, action name, source component id, context, optional data model). When it is
present, `message` must be empty. In order, the handler checks ownership, state
and the declared name (400); returns 410 when the surface is no longer
answerable; returns 413 above 64 KiB of context plus data model; returns 409
when a turn is in flight; then marks the surface submitted with one
compare-and-set that stores the action and the id of the user message about to
be written; persists the user message; and starts the turn. If the turn fails to
start, a compare-and-set on the same link reopens the surface. The in-flight
guard serializes the two routes, because a turn waiting on a surface holds the
conversation in flight. The client sends in-turn answers to the interaction
endpoint and falls back to this route only after a 410 there.

_Rejected_: letting a post-turn action resolve a pending interaction (a
transaction across the surface and interaction stores that the in-flight guard
makes unnecessary); storing the synthesized block in `messages.content` and
stripping it on the way out (every consumer must strip it, and it moves surface
data into the contended message lane); a new `messages` column (same reason).

**Answerable until the next user turn.** A surface can be answered while no user
message follows its anchor on the active path. After that it is read-only and
labelled "Closed". This is derived, not stored: one helper computes `answerable`
for history, and the post-turn route reuses it. The model's next reply was
written against the conversation after that message, so an answer to an older
form would arrive out of context, and the rule keeps one live form at a time. A
surface is single-shot: once submitted it is read-only and shows the submitted
values. A stored `closed` status was rejected because it adds a write to the hot
send path, a rollback case when the send fails and a second writer on
`ui_surfaces`.

**What the model sees, and what the user sees.** The visible user message is
built by the server as `Submitted “<title>”`, and the client shows it with a
"via form" marker. The model-authored `userMessage` that A2UI allows on an
action is never shown as user speech, because the user did not write it. The
block the model sees is not stored in the message. On every history rebuild,
`prepare_history` builds a `SurfaceActionMap` from `ui_surfaces` rows that carry
an action message id, the same way `build_attachment_map` joins attachments, and
`messages_to_chat_history` prepends `[surface_action surface_id=… name=…]` with
the context and data model to that user message.

### 7. Link and image safety

There is no code path for model-authored code: only catalog components and
functions render.

- **Images.** `Image.url` must be `https:` or a same-origin
  `/api/attachments/{id}` owned by the same conversation. The default allows any
  `https:` host. An org can narrow it with `[ui] image_hosts` in `org.toml`
  (a new `UiConfig` section on `OrgConfig`), which the server enforces.
- **Accepted trade-off: image beacons.** Allowing any `https:` host lets that
  host see the user's IP address and the time an image loads. We accept it
  because surfaces exist to show what the model found (charts, product shots,
  maps) and a default allowlist would break most of that for every org. The
  documentation tells orgs that care to set `image_hosts`.
- **Attachment images.** The stock `Image` has no header or loader hook, so the
  client overrides it with an implementation that loads attachments through the
  authenticated HTTP client and applies the URL policy.
- **`openUrl`.** Targets must be `https:` or `mailto:`; `javascript:`, `data:`
  and `http:` are disabled and never reach a prompt. A target whose host is not
  in the org's `[ui] link_hosts` (exact hostnames, case-insensitive, empty by
  default) shows the full URL with the host emphasised and asks before opening.
  `mailto:` always asks. The prompt appears only after the user gesture that
  triggered `openUrl`, and Open is a second gesture. The client reads the
  allowlist from `GET /api/capabilities` as `ui.link_hosts`; a failed or missing
  call counts as empty, so a failure produces more prompts, never fewer. The
  prompt is app chrome and follows the adaptive-widget rule.
  _Rejected_: never prompting (a model-written label can hide the
  destination); remembering a choice per host (needs per-user storage, and the
  org allowlist covers the trusted case).
- **Regex input.** The client passes at most 1,000 characters to `regex` and
  treats longer input as a failed check (see the second half of section 3).
- **Icons.** `Icon.svgPath` is path data only, capped at 4 KiB.
- **Actions** are accepted from callers that pass `caller_can_post` (the
  `conversations:write` scope check in `messages.rs`), for answerable surfaces of
  that conversation, with an action name declared in that surface.

### 8. The renderer lives in `app/`, and surfaces are Material everywhere

`a2ui_flutter` is not on the `assistant_ui` package's dependency allowlist and
pulls in `video_player`. The allowlist is not amended. `assistant_ui` gets only
pure pieces: a `SurfaceRecord` data model and a `SurfaceFrame` timeline element
with a `Widget? content` slot. The adapter (`A2uiSurfaceView`, the assistant
catalog built from the package's basic components, the authenticated `Image`
override, the `openUrl` wrapper and the action callback) lives in
`app/lib/features/chat/a2ui/`, one folder, so churn in a 0.x package has one
place to land. A contract test runs a shared fixture through `MessageProcessor`.

v1.0 has no surface theme, so styling comes from the host. `a2ui_flutter` draws
framework Material widgets, and the app's `MaterialUiCompatibilityBridge`
publishes the framework `ThemeData`, so surfaces inherit the app's light and dark
themes. The adapter overrides radius and spacing through a `Theme`.

**Deviation, accepted for v1.** The repository rule to use adaptive widgets
(Cupertino on iOS and macOS, in `openspec/config.yaml`) is not applied inside
surfaces: buttons, fields and the date picker are Material on every platform.
Overriding them means re-implementing most of the catalog and tracking a 0.x
package's internals. The surface frame and the link prompt are app chrome and do
follow the rule. A widget test pins the Material widget types under iOS and
macOS platform overrides, so a later switch is deliberate.

## Consequences

- A Persona can ask for structured input, and the answer comes back as data
  instead of parsed prose, on one path that every non-interactive interface
  never sees.
- The server is the trust boundary. The client validates a second time, but the
  surface that reaches it already passed the schema, structure, limit, regex and
  URL checks.
- Two new dependencies (`a2ui_flutter` and `a2ui_core`) at exact versions, both
  0.x. Breakage from a version bump is expected and contained in one folder.
- The regex check is conservative. Some safe patterns that nest quantifiers are
  rejected, and the model has to rewrite them. A pattern that passes can still
  be slow on the client in ways the check does not model, which the input cap
  bounds.
- With an empty `link_hosts`, every link asks. That is the safer default and
  cheap to loosen for an org.
- Any `https:` image host is allowed by default, with the beacon trade-off above.
- Surfaces render in Material on iOS and macOS, which differs from the rest of the
  app.
- Playwright baselines move once surfaces appear in the timeline.
- The design depends on sibling changes: gating on the interactivity flag and the
  interaction broker of ADR-0017, the post-turn route on the turn registry and
  runner extraction of ADR-0018, and the surface hook on the tool-result
  forwarding work. Server PRs that need none of them (types, validation,
  storage, event) land first.

## Non-goals

- Components outside the assistant catalog (charts, maps), and Video,
  AudioPlayer and Modal.
- Function RPC from the model (`callRendererFunction`).
- A2UI over A2A, and messenger-native widgets such as Slack Block Kit.
- Surfaces from Subagent Processes.
- Saved, Persona-authored UI templates.
- Updating or closing a surface after the user has submitted it, and a
  `deleteSurface` mode.
- Cupertino-styled surface widgets.
- Keeping a surface answerable after a later user turn, or reopening a closed one.
- Per-link trust prompts that remember a choice.
