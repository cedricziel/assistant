## ADDED Requirements

### Requirement: StoragePlugin persists conversations as a Plugin implementation

`StoragePlugin` SHALL implement the `Plugin` trait and persist conversation messages via a `ConversationStore` injected at construction. It SHALL be available only when the `storage-plugin` Cargo feature of `assistant-agent-loop` is enabled, which introduces a dependency on `assistant-storage`. The base crate SHALL compile without this feature.

#### Scenario: Turn completion triggers persistence

- **WHEN** a turn completes and `StoragePlugin` is registered in the `PluginRegistry`
- **THEN** `on_turn_end` appends the assistant's answer and tool results to the `ConversationStore` for the given `session_id`

#### Scenario: No storage without the plugin

- **WHEN** `StoragePlugin` is not registered and a turn completes
- **THEN** no messages are written to any store; the loop has no knowledge of persistence

### Requirement: StoragePlugin is constructed with constructor injection

`StoragePlugin::new(store: Arc<ConversationStore>) -> Self` SHALL be the only constructor. No global state or singletons.

#### Scenario: Two loops share one store

- **WHEN** two `AgentLoop` instances each register a `StoragePlugin` wrapping the same `Arc<ConversationStore>`
- **THEN** both write to the same store without data races (enforced by `Arc` + async-safe store internals)
