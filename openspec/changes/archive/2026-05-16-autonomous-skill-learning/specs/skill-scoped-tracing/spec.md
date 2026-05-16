## ADDED Requirements

### Requirement: Tool spans tagged with active skill

When the `load-skill` tool is invoked during a turn, the system SHALL record the loaded skill name on all subsequent tool execution spans within that same turn as an `active_skill` OTel attribute.

#### Scenario: Skill loaded then tools executed

- **WHEN** the agent calls `load-skill` with name "coding-agent" and subsequently calls `bash` and `file-read` in the same turn
- **THEN** both the `bash` and `file-read` spans SHALL have attribute `active_skill = "coding-agent"`

#### Scenario: No skill loaded

- **WHEN** the agent executes tools without calling `load-skill` in the turn
- **THEN** tool spans SHALL NOT have an `active_skill` attribute (or it SHALL be null)

#### Scenario: Turn boundary resets active skill

- **WHEN** a turn completes with `active_skill = "coding-agent"` and a new turn begins
- **THEN** the new turn's tool spans SHALL NOT carry the previous turn's `active_skill` value

### Requirement: SQLite exporter persists active_skill

The SQLite OTel span exporter SHALL extract the `active_skill` attribute from spans and persist it in a dedicated column in the `distributed_traces` table.

#### Scenario: Span with active_skill attribute exported to SQLite

- **WHEN** a span with attribute `active_skill = "coding-agent"` is exported
- **THEN** the `distributed_traces` row SHALL have `active_skill = 'coding-agent'`

#### Scenario: Span without active_skill attribute

- **WHEN** a span without an `active_skill` attribute is exported
- **THEN** the `distributed_traces` row SHALL have `active_skill = NULL`

### Requirement: Iceberg exporter persists active_skill

The Iceberg OTel span exporter SHALL extract the `active_skill` attribute and write it as a string column in the `assistant_spans` Parquet table.

#### Scenario: Span with active_skill exported to Iceberg

- **WHEN** a span with attribute `active_skill = "coding-agent"` is exported to Iceberg
- **THEN** the Parquet row SHALL contain `active_skill = "coding-agent"`

### Requirement: Stats query uses active_skill column

The `stats_for_skill()` function SHALL query traces by the `active_skill` column (not `tool_name`) to aggregate skill-level performance statistics.

#### Scenario: Query stats for a skill with execution history

- **WHEN** `stats_for_skill("coding-agent", 50)` is called and there are 20 spans with `active_skill = "coding-agent"`
- **THEN** the returned `TraceStats` SHALL reflect aggregates over those 20 spans

#### Scenario: Query stats for a skill with no history

- **WHEN** `stats_for_skill("new-skill", 50)` is called and no spans have `active_skill = "new-skill"`
- **THEN** the returned `TraceStats` SHALL have `total = 0`

### Requirement: SkillStatsProvider trait abstracts backend

The system SHALL provide a `SkillStatsProvider` trait with implementations for both SQLite and Iceberg backends, allowing the learning subsystem to query skill stats regardless of configured exporter.

#### Scenario: SQLite backend configured

- **WHEN** `exporter = "sqlite"` and skill stats are requested
- **THEN** the system SHALL query the `distributed_traces` SQLite table

#### Scenario: Iceberg backend configured

- **WHEN** `exporter = "iceberg"` and skill stats are requested
- **THEN** the system SHALL scan Parquet files from the warehouse directory
