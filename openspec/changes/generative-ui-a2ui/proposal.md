## Why

A Persona can only answer in Markdown, Mermaid and SVG (`crates/runtime/src/orchestrator/prompt.rs:157`). For structured input it asks in prose and parses the reply. assistant-ui and tool-ui let the model show a form, card or choice list that the user operates directly. We need the same in the web chat without letting the model send executable code.

The user chose the A2UI protocol rendered by Flutter's GenUI SDK. Research shows the SDK has moved on: `genui` 0.10.4 on pub.dev is marked **discontinued** in favour of `a2ui_flutter` (0.5.0, labs.flutter.dev, Dart ≥ 3.13) and `a2ui_core` (0.9.0, pure Dart). Those packages implement A2UI **v1.0**, the current spec (a2ui.org, last updated 2026-06-08), and still read v0.9. This change targets that successor.

## What Changes

- New pure crate `crates/a2ui` (`assistant-a2ui`): A2UI v1.0 message types, an **assistant catalog** (a strict subset of the v1.0 basic catalog), and a validator that checks schema, structure, size limits, regex safety and the URL policy.
- New built-in tool `render-surface`. The model passes a flat component list, a data model and a required `fallback_text`, or updates an existing surface with `surface_id`. The server assigns the `surface_id`, validates the input and builds the A2UI messages. On a validation failure the tool returns a structured `ToolOutput::error` so the model can correct itself.
- The tool is offered only on **interactive web turns at subagent depth 0** (`ExecutionContext.interactive`, added by `tool-approval-prompts`, plus `Interface::Web`). Quick-message, voice, messenger, CLI, MCP, A2A and scheduled turns never see it. The tool is registered only after the client can render surfaces, so `main` never offers a surface nobody can see.
- New `OrchestratorEvent::SurfaceUpdate` → SSE event `surface_update`. It is stored in the event log, so replay re-surfaces it. The streamed-turn runner keeps persisting frames after the client disconnects, so a reload during a wait does not lose later frames.
- New `ui_surfaces` table in space.db behind a `SurfaceStore` trait with InMemory and Sqlite implementations. Surfaces are anchored to their tool call (`tool_call_id`, or tool-call message id plus call index) and reload from history through `ConversationDetail.surfaces`, because the event log expires after 24h (`crates/storage/src/conversation_events.rs:20`).
- Link safety: `Image.url` allows any `https:` host unless the org narrows it with `[ui] image_hosts`. `openUrl` to a host outside the org allowlist `[ui] link_hosts` (empty by default) shows the destination and asks before opening. The client reads the allowlist from `GET /api/capabilities` (`ui.link_hosts`).
- A surface is answerable until the next user turn. After that it is read-only and labelled closed. The state is derived (no user message follows its anchor on the active path), not stored; the post-turn route answers `410` for a closed surface.
- User actions take one of two routes. **While the turn waits** (`wait_for_action: true`), the action travels on the interaction channel owned by `tool-approval-prompts`, as a new interaction kind `surface_action` that renders no card of its own. **After the turn has ended**, it arrives as a new user turn through `SendMessageRequest.surface_action`. The visible user message is server-built (`Submitted “<title>”`, shown with a "via form" marker); the structured block the LLM sees is synthesized from the `ui_surfaces` row on every history rebuild.
- Client: `SurfaceFrame` element plus the `SurfaceRecord` model in `assistant_ui` (pure). The A2UI renderer adapter lives in `app/lib/features/chat/a2ui/` because `a2ui_flutter` is outside the package allowlist. The allowlist is **not** amended. Surface content is Material on every platform, including iOS and macOS (accepted for v1).
- ADR-0019.

## Capabilities

### New Capabilities

- `generative-ui-surfaces`: the server side. Catalog, validation, the `render-surface` tool, gating, events, persistence and both action routes.
- `generative-ui-rendering`: the client side. Parsing, rendering, theming, URL enforcement, history and action dispatch.

### Modified Capabilities

- `turn-interactions` (introduced by `tool-approval-prompts`): the respond endpoint accepts a `surface_action` answer.

## Impact

- **Crates**: new `assistant-a2ui`; `core` (ToolSpec flag, `[ui]` org config with `image_hosts` and `link_hosts`), `tool-executor`, `runtime` (event, dispatch hook, projectors, history synthesis, interaction kind handler), `storage` (migration `049_ui_surfaces`, store), `web-ui` (DTOs, runner, capabilities `ui.link_hosts`, OpenAPI).
- **Not touched**: the `messages` table and the `Message` struct. Surface metadata lives only in `ui_surfaces`, so this change stays out of the contended Message-struct lane.
- **App**: `assistant_ui` (model, frame, stories, goldens); `app/` adds `a2ui_flutter` and `a2ui_core`, pinned exactly.
- **Visual change**: yes. Surfaces are new timeline entries. **Playwright baselines will move.**
- **Docs**: yes. User-facing `docs/chat/generative-ui.md`, the `[ui]` setting in `docs/operations/configuration.md`, and the ADR.

### Non-goals

- Custom components outside the assistant catalog (charts, maps) and Video, AudioPlayer, Modal.
- Agent-initiated `callRendererFunction` / function RPC.
- A2UI over A2A (the v0.9.1 extension) and messenger-native widgets such as Slack Block Kit.
- Surfaces from subagents.
- Persona-authored, saved UI templates.
- Updating or closing surfaces after the user has submitted them. There is no `closed` state and no `deleteSurface`.
- Cupertino-styled surface widgets (D10).
- Keeping a surface answerable after a later user turn, or reopening a closed one.
- Per-link trust prompts that remember a choice; the confirmation is asked every time for hosts outside the allowlist.
