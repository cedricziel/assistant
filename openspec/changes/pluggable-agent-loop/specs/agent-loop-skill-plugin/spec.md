## ADDED Requirements

### Requirement: SkillPlugin injects skill content and registers skill tools as a Plugin implementation

`SkillPlugin` SHALL implement the `Plugin` trait and use a `SkillRegistry` injected at construction to inject skill system-prompt content via `transform_context` and register skill-defined tools into the loop's `ToolExecutor`. It SHALL be available only when the `skill-plugin` Cargo feature of `assistant-agent-loop` is enabled, which introduces a dependency on `assistant-skills`. The base crate SHALL compile without this feature.

#### Scenario: Skill content injected before LLM call

- **WHEN** `SkillPlugin` is registered and skills exist in the `SkillRegistry` for the current session context
- **THEN** `transform_context` prepends the skill system content as a `System`-role `ChatHistoryMessage` before the LLM call

#### Scenario: No skills without the plugin

- **WHEN** `SkillPlugin` is not registered
- **THEN** no skill content is injected into the context; the loop proceeds with the messages as provided

### Requirement: SkillPlugin is constructed with constructor injection

`SkillPlugin::new(registry: Arc<SkillRegistry>) -> Self` SHALL be the only constructor.

#### Scenario: SkillPlugin used without storage

- **WHEN** `SkillPlugin` is registered but `StoragePlugin` is not
- **THEN** skill content is injected correctly; the absence of storage does not affect skill injection
