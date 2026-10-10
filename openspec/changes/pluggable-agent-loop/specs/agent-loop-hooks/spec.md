## ADDED Requirements

### Requirement: Plugin trait defines async lifecycle hooks with default implementations

The `Plugin` trait in `assistant-agent-loop` SHALL declare async lifecycle methods — `on_session_start`, `on_session_end`, `on_turn_start`, `on_turn_end`, `transform_context`, `before_llm_request`, `after_llm_response`, `before_tool_call`, `after_tool_call` — all with default no-op implementations so implementors only override the hooks they need. The trait SHALL be `Send + Sync` and usable behind `Arc<dyn Plugin>`.

#### Scenario: Plugin with no overrides compiles and runs

- **WHEN** a struct implements `Plugin` with only the `name()` method and registers with `PluginRegistry`
- **THEN** all hook dispatch calls succeed and the loop proceeds normally

#### Scenario: Plugin overrides before_tool_call to block a tool

- **WHEN** a plugin's `before_tool_call` returns `BeforeToolCallOutcome::Block { reason }`
- **THEN** the tool is not executed, the block reason is recorded in the tool result sent back to the LLM, and subsequent plugins' `before_tool_call` is not called

### Requirement: PluginRegistry dispatches hooks sequentially in registration order

`PluginRegistry` SHALL maintain an ordered `Vec<Arc<dyn Plugin>>` and provide typed dispatch methods for each hook. Hooks SHALL be called in registration order. For transform hooks (`transform_context`, `before_llm_request`, `after_llm_response`, `after_tool_call`), each plugin's output SHALL be passed as the input to the next plugin (pipeline composition). For lifecycle hooks (`on_turn_start`, `on_turn_end`, etc.), all plugins SHALL be called regardless of individual errors; errors SHALL be logged at `warn` level but SHALL NOT abort the loop.

#### Scenario: Transform pipeline composes plugin outputs

- **WHEN** two plugins are registered and both override `transform_context`
- **THEN** the second plugin's `transform_context` receives the message list already modified by the first plugin

#### Scenario: Block from first plugin skips remaining plugins

- **WHEN** plugin A returns `Block` from `before_tool_call` and plugin B is registered after A
- **THEN** plugin B's `before_tool_call` is never called for that tool invocation

#### Scenario: Lifecycle hook error does not abort the turn

- **WHEN** plugin A's `on_turn_start` returns an `Err`
- **THEN** plugin B's `on_turn_start` is still called and the turn proceeds normally; the error is logged at warn level

### Requirement: before_tool_call can inspect and is given ToolCallContext

`ToolCallContext` SHALL expose `tool_name: &str`, `call_id: &str`, and `args: &serde_json::Value` as read-only fields. The hook SHALL NOT mutate args (arg mutation is not required by this change; it may be added in a future change).

#### Scenario: Plugin reads tool name in before_tool_call

- **WHEN** the LLM calls the `bash` tool and a plugin implements `before_tool_call`
- **THEN** the plugin receives a `ToolCallContext` with `tool_name == "bash"` and the args as JSON

### Requirement: after_tool_call can replace the tool result content

`after_tool_call` SHALL receive a mutable `ToolCallResult` with fields `content: String` and `is_error: bool`. A plugin SHALL be able to modify `content` or `is_error` in place; subsequent plugins see the already-modified result.

#### Scenario: Plugin redacts sensitive output

- **WHEN** a plugin overrides `after_tool_call` and sets `result.content = "[redacted]"`
- **THEN** the LLM receives `[redacted]` as the tool result and subsequent plugins see `[redacted]`

### Requirement: transform_context allows pre-LLM message list modification

`transform_context` SHALL receive the full `Vec<ChatHistoryMessage>` that would be sent to the LLM and SHALL return a (possibly modified) `Vec<ChatHistoryMessage>`. Plugins MAY filter, reorder, or inject messages. The returned list is what gets sent to the LLM.

#### Scenario: Plugin injects a system message before LLM call

- **WHEN** a plugin's `transform_context` prepends a `ChatHistoryMessage` with role `System`
- **THEN** the LLM request includes that message as the first item in the history

### Requirement: before_llm_request and after_llm_response allow provider payload hooks

`LlmRequest` SHALL expose mutable `temperature: Option<f32>`, `max_tokens: Option<u32>`, and `extra: serde_json::Map<String, Value>` fields for provider-specific overrides. `LlmResponse` SHALL expose `content: String` as mutable for post-processing. Plugins MAY modify these in `before_llm_request` and `after_llm_response` respectively.

#### Scenario: Plugin lowers temperature for a specific session

- **WHEN** a plugin sets `request.temperature = Some(0.0)` in `before_llm_request`
- **THEN** the LLM is called with `temperature=0.0` for that turn
