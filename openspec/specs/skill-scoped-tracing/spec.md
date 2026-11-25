# skill-scoped-tracing Specification

## Purpose

TBD - created by archiving change autonomous-skill-learning. Update Purpose after archive.

## Requirements

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

### Requirement: SkillStatsProvider trait abstracts the statistics source

The system SHALL provide a `SkillStatsProvider` trait so the learning subsystem obtains per-skill execution statistics through an abstraction rather than a concrete store. Telemetry is exported over OTLP only and the assistant keeps no local trace store, so the statistics source is a product-owned one — see `docs/adr/adr-0012-otlp-only-telemetry.md`.

#### Scenario: No statistics source is configured

- **WHEN** skill stats are requested and the `ToolExecutor` was built with the default provider
- **THEN** the provider SHALL return an error naming the skill rather than a zero-execution result
- **AND** the `self-analyze` tool SHALL report that failure to the model

#### Scenario: Alternative provider injected

- **WHEN** a `ToolExecutor` is constructed via `with_stats_provider` with a `SkillStatsProvider`
- **THEN** the `self-analyze` tool SHALL obtain its statistics from that provider without any change to the tool itself
