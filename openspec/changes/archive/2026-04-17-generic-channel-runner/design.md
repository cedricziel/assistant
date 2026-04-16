## Context

Five messenger interface crates each contain a `run()` method that reimplements the same dispatch machinery: a tokio-tungstenite or HTTP stream is consumed, messages are keyed into conversations via an LRU cache, per-conversation turns are serialized with a mutex, and `run_turn_with_tools` is called. The only genuine differences are the conversation key formula, the set of platform-specific tools, and optional lifecycle side-effects (reactions, typing indicators).

The `ChannelAdapter` trait already exists in `crates/core` with `start()`, `send()`, and `stop()`. It is the right abstraction boundary. What is missing is the generic runtime that drives it.

## Goals / Non-Goals

**Goals:**

- Single `ChannelRunner` struct in `crates/runtime` that works with any `ChannelAdapter`
- Extend `ChannelAdapter` with optional hooks covering the variation points (conversation key, platform tools, turn lifecycle)
- Delete per-adapter runner boilerplate; each interface's `run()` reduces to constructing `ChannelRunner` and calling its `run()`
- Signal remains untouched (presage non-Send constraint is fundamental)

**Non-Goals:**

- Multi-channel daemon / supervisor process (that is a follow-on change)
- Changing the `run_turn_with_tools` API
- Changing how tools are declared or registered globally
- Rewriting Slack history seeding (keep it as a hook implementation)

## Decisions

### D1 — Hooks on `ChannelAdapter`, not a separate trait

**Options:**

1. Add optional hook methods directly to `ChannelAdapter` with default no-ops
2. Separate `ChannelAdapterHooks` trait that `ChannelRunner` also accepts
3. Builder-style configuration struct passed to `ChannelRunner`

**Choice: Option 1** — keeps the single abstraction, avoids extra generic parameters, and default no-ops mean existing adapter implementations compile without changes. All hooks are `async`, consistent with the rest of the trait.

### D2 — `conversation_key(&ChannelMessage) -> String`

The per-conversation key formula varies per platform:

- Slack: `{channel_id}:{thread_ts}` (or message ts for new threads)
- Mattermost: `{channel_id}:{root_id}` (or post id for root messages)
- Matrix: `{room_id}` (room is the conversation unit)
- Nextcloud: `{conversation_token}`

The default implementation on the trait uses `{sender.platform_id}:{thread_id ?? platform_message_id}`, which works correctly for Matrix and Nextcloud. Slack and Mattermost adapters override it.

### D3 — `platform_tools(&ChannelMessage, Uuid) -> Vec<Arc<dyn ToolHandler>>`

Returns interface-specific tools for a given message. Called once per dispatch, before `run_turn_with_tools`. Default returns `vec![]`. Slack adapter returns `build_slack_tools(...)`, Mattermost returns `build_mattermost_tools(...)`, etc.

The `conv_id: Uuid` parameter is included so tools can reference the conversation for history/context purposes.

### D4 — `ChannelRunner` owns conversation state, not the adapter

The LRU cache and per-conversation mutex live in `ChannelRunner`, not in individual adapters. This keeps adapters stateless and focused on I/O only.

```rust
pub struct ChannelRunner {
    adapter: Arc<dyn ChannelAdapter>,
    orchestrator: Arc<Orchestrator>,
    conversations: Mutex<LruCache<String, Uuid>>,         // key → conv_id
    conv_locks: Mutex<HashMap<Uuid, Arc<Mutex<()>>>>,     // conv_id → turn lock
}
```

### D5 — Nextcloud adapter gets a real `start()` stream

The current Nextcloud runner embeds an axum server inline. The `NextcloudAdapter::start()` already exists as a stub; it needs to spawn the axum server internally and return messages via `mpsc` → `ReceiverStream`, consistent with how the adapter was designed in the `thin-messenger-http-clients` change.

### D6 — Per-interface `InterfaceRunner` impl becomes a one-liner

```rust
impl InterfaceRunner for SlackInterface {
    async fn run(&self) -> Result<()> {
        ChannelRunner::new(
            Arc::new(SlackAdapter::new(self.config.clone())),
            self.orchestrator.clone(),
        ).run().await
    }
}
```

The existing `SlackInterface` struct is kept as the public entry point for config + construction, but the body of `run()` is replaced.

## Risks / Trade-offs

**[Risk] Nextcloud axum server port conflicts** — Spawning the server inside `start()` means the port is bound when the stream is created, not when the runner starts. If `start()` is called more than once (reconnect logic), a second bind attempt will fail.
→ Mitigation: Guard with `Arc<AtomicBool>` inside the adapter; only bind once. Reconnect is not applicable to webhook adapters (the server stays up).

**[Risk] Slack history seeding timing** — Slack currently seeds thread history before calling `run_turn_with_tools`. This happens inside the runner's dispatch function. Moving to a hook (`on_turn_start`) loses access to the orchestrator needed to inject history into the conversation.
→ Mitigation: `platform_tools()` hook receives `conv_id`; the Slack adapter can call `orchestrator.seed_history(...)` if given a reference. The adapter constructor accepts `Arc<Orchestrator>` where needed, or history seeding moves into a specialized `SlackAdapter` method called from `on_turn_start`.

**[Risk] Compile-time feature gating per interface** — The CLI currently uses `#[cfg(feature = "slack")]` etc. `ChannelRunner` is feature-agnostic but the adapters are still gated.
→ Non-issue: the gating stays on the adapter construction site, not on `ChannelRunner`.

## Migration Plan

1. Add hook methods to `ChannelAdapter` in `crates/core` (default no-ops)
2. Implement `ChannelRunner` in `crates/runtime`
3. Migrate Mattermost → `ChannelRunner` (simplest, no ambient tools)
4. Migrate Matrix → `ChannelRunner`
5. Migrate Nextcloud → `ChannelRunner` (fix `start()` to spawn axum)
6. Migrate Slack → `ChannelRunner` (most complex: reactions, ambient tools, history)
7. Delete dead runner boilerplate
8. Verify `make lint && make test` pass

Rollback: each step is a separate, independently compilable change. Reverting a step means restoring the previous runner body.

## Open Questions

- Should `ChannelRunner` expose a `run_with_boot_hook(boot_id)` variant, or should boot be part of `ChannelAdapter::start()` pre-conditions?
- History seeding in Slack: pass `Arc<Orchestrator>` to the adapter, or leave seeding in a thin wrapper above `ChannelRunner`?
