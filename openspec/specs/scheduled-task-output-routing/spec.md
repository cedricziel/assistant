## ADDED Requirements

### Requirement: Persona MAY declare a home channel

A `PersonaRecord` SHALL optionally carry a `home_channel` consisting of an interface name and a channel address. When present, both fields MUST be non-empty strings. When absent, scheduler-originated turns SHALL behave exactly as before this change.

#### Scenario: Persona stored with home channel

- **WHEN** a persona is created or updated with both `home_channel_interface` and `home_channel_channel` set
- **THEN** the values are persisted and returned on subsequent persona lookups

#### Scenario: Persona stored without home channel

- **WHEN** a persona has no `home_channel` configured
- **THEN** scheduler-originated turns fire without output tools, as today

### Requirement: Scheduler injects platform tools from persona home channel

When the scheduler fires a turn (cron task or heartbeat) for a persona that has `home_channel` set, it SHALL look up the matching live adapter and inject platform tools bound to the home channel destination.

#### Scenario: Matching adapter is running

- **WHEN** a scheduler turn fires for a persona with `home_channel = { interface: "slack", channel: "#ops" }`
- **AND** a Slack adapter is registered in the adapter registry
- **THEN** the agent turn receives Slack platform tools pre-bound to `#ops`
- **AND** the agent can use those tools to post output to the channel

#### Scenario: Matching adapter is not running

- **WHEN** a scheduler turn fires for a persona with a home channel configured
- **AND** no adapter matching the configured interface is registered
- **THEN** the turn fires without output tools
- **AND** a warning is logged
- **AND** output is stored in conversation history only

#### Scenario: Heartbeat uses persona home channel

- **WHEN** a heartbeat fires for a persona with `home_channel` set
- **THEN** the same routing logic applies as for scheduled tasks

### Requirement: Adapter registry tracks live adapters

The runtime SHALL maintain a registry of currently running `ChannelAdapter` instances keyed by interface name. Adapters SHALL register on start and deregister on stop.

#### Scenario: Adapter registers on start

- **WHEN** a `ChannelRunner` starts a `ChannelAdapter`
- **THEN** the adapter is added to the registry under its interface name

#### Scenario: Adapter deregisters on stop

- **WHEN** a `ChannelAdapter` stops
- **THEN** the adapter is removed from the registry

#### Scenario: Registry lookup by interface name

- **WHEN** the scheduler looks up an adapter by interface name
- **THEN** the registry returns the live adapter if present, or `None` if absent
