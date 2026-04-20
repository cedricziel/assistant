## ADDED Requirements

### Requirement: Post-turn heuristic gate filters trivial turns

After each successful turn, the system SHALL apply heuristic filters before invoking the LLM evaluator. A turn SHALL be skipped if any of the following are true: a skill was already active during the turn, fewer than the configured minimum tool calls were made, or the turn had tool execution errors.

#### Scenario: Skill already active

- **WHEN** a turn completes with `active_skill = "coding-agent"` and 5 tool calls
- **THEN** the post-turn evaluator SHALL NOT invoke the LLM judge

#### Scenario: Too few tool calls

- **WHEN** a turn completes with 2 tool calls and `min_tool_calls_for_skill = 3`
- **THEN** the post-turn evaluator SHALL NOT invoke the LLM judge

#### Scenario: Turn had errors

- **WHEN** a turn completes with tool execution errors
- **THEN** the post-turn evaluator SHALL NOT invoke the LLM judge

#### Scenario: Turn passes all gates

- **WHEN** a turn completes with no active skill, 4 tool calls, no errors, and `min_tool_calls_for_skill = 3`
- **THEN** the post-turn evaluator SHALL invoke the LLM judge

### Requirement: LLM judge decides skill creation

The system SHALL ask the LLM to evaluate whether a completed turn should become a reusable skill. The LLM SHALL return a structured decision including whether to create, a kebab-case name, a description, and the full SKILL.md body.

#### Scenario: LLM recommends skill creation

- **WHEN** the LLM judge returns `create: true` with name "deploy-docker-compose" and a body
- **THEN** the system SHALL proceed to register the skill

#### Scenario: LLM recommends no skill creation

- **WHEN** the LLM judge returns `create: false`
- **THEN** no skill SHALL be created and no further action is taken

#### Scenario: LLM returns invalid response

- **WHEN** the LLM judge returns unparseable output
- **THEN** the system SHALL log a warning and skip skill creation without error

### Requirement: Auto-created skills registered in SkillRegistry

When the LLM judge recommends skill creation, the system SHALL write a SKILL.md file to the skills directory and register it in the SkillRegistry so it is immediately available for future turns.

#### Scenario: New skill registered successfully

- **WHEN** the LLM recommends creating "deploy-docker-compose"
- **THEN** a file SHALL be written at the configured skills directory with proper frontmatter and body, and the skill SHALL appear in `registry.list()`

#### Scenario: Skill name collision

- **WHEN** the LLM recommends name "coding-agent" and that skill already exists
- **THEN** the system SHALL append a numeric suffix (e.g. "coding-agent-2") and register with the deduplicated name

### Requirement: Auto-created skills marked with source metadata

Auto-created skills SHALL include `source: auto` in their SKILL.md frontmatter to distinguish them from hand-authored skills.

#### Scenario: Inspect auto-created skill frontmatter

- **WHEN** a skill is created by the post-turn evaluator
- **THEN** the SKILL.md frontmatter SHALL contain `source: auto`

### Requirement: Post-turn evaluator runs asynchronously

The post-turn evaluator SHALL run as a fire-and-forget background task that does not block the turn response to the user.

#### Scenario: User receives response before evaluation completes

- **WHEN** a turn completes and the post-turn evaluator is triggered
- **THEN** the turn result SHALL be returned to the user immediately without waiting for the evaluator

### Requirement: Learning disabled by default

The autonomous skill creation feature SHALL be disabled by default. Users MUST explicitly set `learning.enabled = true` and `learning.auto_create_skills = true` in configuration.

#### Scenario: Default configuration

- **WHEN** no `[learning]` section exists in config.toml
- **THEN** the post-turn evaluator SHALL NOT run

#### Scenario: Learning enabled

- **WHEN** `learning.enabled = true` and `learning.auto_create_skills = true`
- **THEN** the post-turn evaluator SHALL run after each qualifying turn
