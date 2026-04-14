## 1. Storage — home_channel on PersonaRecord

- [x] 1.1 Add `HomeChannel` struct to `crates/storage/src/personas.rs` with fields `interface: String` and `channel: String`
- [x] 1.2 Add `home_channel: Option<HomeChannel>` to `PersonaRecord`
- [x] 1.3 Write DB migration adding `home_channel_interface TEXT` and `home_channel_channel TEXT` (both nullable) to the `personas` table
- [x] 1.4 Update `PersonaStore` insert/update queries to persist the new fields
- [x] 1.5 Update `PersonaStore` select queries to deserialize the new fields
- [x] 1.6 Write unit tests for round-trip with and without `home_channel`

## 2. Adapter Registry

- [x] 2.1 Add `AdapterRegistry` type to `crates/runtime/src/` — `Arc<RwLock<HashMap<String, Arc<dyn ChannelAdapter>>>>` with `register`, `deregister`, and `get` methods
- [x] 2.2 Hold `AdapterRegistry` on `Orchestrator` and expose it
- [x] 2.3 Update `ChannelRunner::run` to register the adapter on start and deregister on stop/error
- [x] 2.4 Write unit tests for register/deregister/get behaviour

## 3. Scheduler — home channel tool injection

- [x] 3.1 In `run_due_tasks` and `run_heartbeat`, load the active persona via `PersonaStore::get(agent_id)` and read `home_channel`
- [x] 3.2 If `home_channel` is set, look up the adapter by `interface` in `AdapterRegistry`
- [x] 3.3 If adapter found, construct a synthetic `ChannelMessage` from `home_channel.channel` and call `adapter.platform_tools(&synthetic_msg, conv_id)`
- [x] 3.4 Pass the resulting tools as `extension_tools` in the `TurnRequest` published to the bus
- [x] 3.5 If adapter not found, log a warning and fire without output tools
- [x] 3.6 Write unit tests: adapter present, adapter absent, no home channel configured

## 4. Adapter audit — synthetic ChannelMessage compatibility

- [x] 4.1 Review `SlackAdapter::platform_tools` — ensure it falls back to `sender.platform_id` when `metadata["channel_id"]` is absent
- [x] 4.2 Review `NextcloudAdapter::platform_tools` — same check
- [x] 4.3 Fix any adapter that breaks with a synthetic message (no metadata, no thread_id)

## 5. Cleanup

- [x] 5.1 Run `make lint` and `make format` — fix any warnings
- [x] 5.2 Run `make test` — confirm all existing and new tests pass
- [ ] 5.3 Manual smoke test: configure a persona with `home_channel = slack/#test`, fire a scheduled task, confirm the agent posts to the channel
