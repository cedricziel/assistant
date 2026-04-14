## Why

Scheduled tasks and heartbeats run through the agent runtime but their output evaporates — there is no way to route results to a messaging channel. An agent that monitors systems, sends daily digests, or runs background checks needs a way to deliver output to a specific platform (Slack, Signal, Matrix, etc.) without requiring a human to initiate the conversation.

## What Changes

- Add `home_channel` (interface + channel) to `PersonaRecord` as the persona-level default output destination for all scheduler-originated turns (scheduled tasks and heartbeats)
- Add a DB migration to persist `home_channel` on the `personas` table
- Extend the scheduler to look up the active persona's `home_channel` at fire time and inject platform tools bound to that destination (mirroring how `ChannelRunner` injects `platform_tools()` for inbound turns)
- Add a live adapter registry so the scheduler can look up a running `ChannelAdapter` by interface name at fire time
- Update `list-tasks` to surface whether a home channel is configured
- **Out of scope**: a management command for setting `home_channel` (follow-on)

## Capabilities

### New Capabilities

- `scheduled-task-output-routing`: Persona-level `home_channel` config routes scheduler output (cron tasks + heartbeats) to a configured platform adapter; scheduler injects platform tools so the agent can post without any routing logic of its own

### Modified Capabilities

- (none — no existing spec-level requirements change)

## Impact

- `crates/storage` — `PersonaRecord`, `PersonaStore`, DB migration
- `crates/tool-executor` — `list-tasks` builtin tool
- `crates/runtime` — scheduler dispatch logic, new `AdapterRegistry` for live adapter lookup
- No breaking changes — personas without `home_channel` behave exactly as today
