## Context

Scheduled tasks and heartbeats fire through the agent runtime but their output has nowhere to go — it lands in the conversation store and no human sees it. Inbound messenger turns (Slack, Signal, Matrix, etc.) inject platform-specific tools via `ChannelAdapter::platform_tools(&msg, conv_id)`, pre-bound to the originating message context. The scheduler never calls `platform_tools()` because there is no inbound message.

`PersonaRecord` already holds per-persona behavioral config (`turn_timeout_secs`, `skill_access_mode`). `home_channel` fits naturally alongside these — it is an operator-set property of the persona, not something the agent reasons about at runtime.

## Goals / Non-Goals

**Goals:**

- Add `home_channel` (interface + channel address) to `PersonaRecord`
- When the scheduler fires any turn (cron task or heartbeat) for a persona that has `home_channel` set, inject platform tools bound to that destination
- Operator sets it via DB/API — agent is unaware
- Graceful degradation when no adapter matches or `home_channel` is unset

**Non-Goals:**

- Per-task output destination override
- Agent choosing an interface at runtime
- A management command for setting `home_channel` (follow-on)
- Cross-platform fanout / broadcast
- Heartbeat-specific vs task-specific routing (both use the same persona `home_channel`)

## Decisions

### 1. `home_channel` on `PersonaRecord`

Add two nullable columns to the `personas` table:

```sql
ALTER TABLE personas ADD COLUMN home_channel_interface TEXT;
ALTER TABLE personas ADD COLUMN home_channel_channel   TEXT;
```

Both must be set together or not at all. `PersonaRecord` gains:

```rust
pub home_channel: Option<HomeChannel>,

pub struct HomeChannel {
    pub interface: String,  // e.g. "slack", "signal", "matrix"
    pub channel: String,    // platform-native address: "#ops", "+1234…", "!room:server"
}
```

Using `String` for `interface` keeps `storage` free of adapter-crate dependencies.

**Alternative considered**: Putting `home_channel` in `config.toml` under `[agent]`. Rejected — `config.toml` is a deployment-level file, not a persona-level one. `PersonaRecord` is already the home for per-persona behavioral config.

### 2. Live adapter registry

Add `AdapterRegistry` to `crates/runtime` — a `Arc<RwLock<HashMap<String, Arc<dyn ChannelAdapter>>>>` newtype keyed by the adapter's `name()` string (which matches `ChannelType::to_string()`).

`ChannelRunner` registers the adapter on start and deregisters on stop. The scheduler holds a reference to the registry and looks up the adapter by `home_channel.interface` at fire time.

If no adapter is found the scheduler logs a warning and fires the turn without output tools — same behaviour as today.

**Alternative considered**: Passing adapters directly into the scheduler. Rejected — the scheduler is spawned before adapters start and adapters can come and go at runtime.

### 3. Synthetic `ChannelMessage` for `platform_tools()` injection

To reuse the existing `platform_tools(&msg, conv_id)` API without a trait change, the scheduler constructs a minimal synthetic `ChannelMessage` from `home_channel`:

```rust
ChannelMessage {
    channel_type: resolved_channel_type,
    platform_message_id: None,
    sender: ChannelUser {
        platform_id: home_channel.channel.clone(),
        display_name: None,
    },
    content: ChannelContent::Text(String::new()),
    thread_id: None,
    timestamp: Utc::now(),
    metadata: HashMap::new(),
}
```

Adapters that read `metadata` fields (e.g. Slack reads `channel_id` from metadata) must be audited to fall back to `sender.platform_id` when metadata is absent.

**Alternative considered**: A new `output_tools(&HomeChannel, conv_id)` method on `ChannelAdapter`. Cleaner long-term but requires a trait change and migration of all adapters. Deferred.

### 4. Persona lookup at scheduler fire time

The scheduler already has access to `StorageLayer`. At fire time it calls `PersonaStore::get(orchestrator.agent_id)` to retrieve the active persona and read `home_channel`. This is a single indexed query per scheduled turn — negligible overhead.

## Risks / Trade-offs

- **Synthetic ChannelMessage mismatches adapter expectations** → Slack and Nextcloud currently override `platform_tools()` and read from `metadata`. Both need a small fix to fall back to `sender.platform_id`. Low risk, bounded scope.
- **Adapter not running at fire time** → Graceful degradation: turn fires without output tools, output lands in conversation history. Acceptable.
- **agent_id / persona_id conflation** → Today `orchestrator.agent_id` is used to look up both the filesystem workspace and the persona. This is existing tech debt; this change does not worsen it and does not depend on resolving it.
- **DB migration** → Additive nullable columns. No data loss, no rollback needed.

## Open Questions

- Should `home_channel_channel` encode enough for threading (e.g. Slack `channel_id:thread_ts`)? For now, non-threaded posting is acceptable.

## Known Limitations

- **Single adapter per interface type**: The `AdapterRegistry` is keyed by interface name (`"slack"`, `"signal"`, etc.). Running two adapters of the same type (e.g. two Slack workspaces) would require adapter instance IDs and a richer `home_interface` → `home_adapter_id` model. This is out of scope; the one-adapter-per-type constraint is acceptable for the common case.
