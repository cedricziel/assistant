## Why

Users cannot control conversation lifecycle, switch models, or manage context without restarting the process or navigating external UI. Competing projects (OpenClaw, Hermes Agent) treat slash commands as first-class UX — autocomplete, cross-interface consistency, instant execution. Our current CLI has a handful of hardcoded commands (`/skills`, `/help`, `/quit`); messenger interfaces have none. Adding a unified command system makes every interface more capable without requiring users to leave the conversation.

## What Changes

- Add a `CommandRegistry` with typed `CommandDef` definitions in `assistant-core`, shared by all interfaces.
- Implement 6 built-in commands: `/new`, `/stop`, `/model`, `/compact`, `/status`, `/help`.
- Intercept slash commands in `ChannelRunner` **before** dispatch to the orchestrator — commands never reach the LLM.
- Add a `conversation_events` table to store command invocations (visible to users in the timeline, invisible to the model).
- Add per-conversation config overrides (`ConversationConfig`) so `/model` can change the model for a single conversation.
- Expose `GET /api/commands` and `POST /api/conversations/{id}/command` REST endpoints.
- Add Slack-style `/` autocomplete popup in the Flutter web UI with argument completion.
- Migrate the CLI's existing hardcoded command dispatch to the shared registry.

## Non-goals

- Extensible command registration (skills or plugins adding custom commands) — future work.
- OpenClaw-style inline directives (`/think high explain X`) — separate concern.
- Command authorization or per-user permissions.

## Capabilities

### New Capabilities

- `slash-command-registry`: Core command type definitions (`CommandDef`, `CommandResult`), registry, and built-in command implementations (`/new`, `/stop`, `/model`, `/compact`, `/status`, `/help`).
- `slash-command-dispatch`: Interception layer in `ChannelRunner` and CLI that routes `/`-prefixed messages to the command registry instead of the orchestrator. Includes locking semantics (most commands bypass the conversation lock; `/compact` acquires it).
- `slash-command-events`: `conversation_events` storage table and API endpoints for durable, timeline-visible command records that are excluded from LLM context.
- `slash-command-ui`: Flutter autocomplete popup triggered on `/`, argument completion (e.g. model list for `/model`), and distinct timeline rendering for command events.

### Modified Capabilities

- `channel-runner`: `ChannelRunner::run()` gains a command interception step between `on_message_received` and `dispatch`. Per-turn `CancellationToken` added for `/stop` support.
- `context-management`: `maybe_compact()` gains a `force` parameter so `/compact` can trigger compaction regardless of token threshold.

## Impact

- **Crates**: `assistant-core` (types), `assistant-runtime` (registry + dispatch + ChannelRunner), `assistant-storage` (migration + ConversationConfig), `assistant-web-ui` (API endpoints), `assistant-interface-cli` (migrate to registry).
- **API**: Two new endpoints added to OpenAPI spec.
- **Database**: New `conversation_events` table (migration).
- **Flutter**: New autocomplete widget, command event timeline rendering, new API client endpoints.
- **All messenger interfaces**: Gain command support automatically via `ChannelRunner`.
