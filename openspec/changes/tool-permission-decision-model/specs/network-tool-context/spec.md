## ADDED Requirements

### Requirement: Network destinations are not statically blocked

`web-fetch` and `workflow-http` SHALL NOT refuse a request because its
destination is loopback, private, link-local or otherwise internal. Reaching
local services (home automation, NAS, LAN APIs, localhost tools) is a supported
use case. Network destinations SHALL be governed only by the tool-permission
pipeline: user-configured `tool(url-pattern)` rules, then the decision model,
then the ask/deny fallback. Built-in static rules SHALL NOT include a
destination deny-list.

#### Scenario: Loopback fetch reaches the pipeline, not a block

- **WHEN** `web-fetch` is called with `http://127.0.0.1:8123/api/states` and no
  rule matches
- **THEN** the call SHALL be passed to the decision model (or the ask/deny
  fallback) rather than refused by the tool

#### Scenario: User rule governs a host

- **WHEN** config contains `allow = ["web-fetch(http://homeassistant.local:8123/*)"]`
- **THEN** a fetch of that URL SHALL be allowed without consulting the model

#### Scenario: User deny rule on a host

- **WHEN** config contains `deny = ["web-fetch(*://*.corp.example/*)"]`
- **THEN** a matching fetch SHALL be denied and SHALL NOT reach the model

### Requirement: Decision-model context describes the destination

For `web-fetch` and `workflow-http` calls, the decision-model request SHALL
include the destination host and whether it resolves to a private, loopback or
link-local address, so the model can weigh the destination. A resolution
failure SHALL be reported as "unknown" and SHALL NOT fail the request.

#### Scenario: Private destination flagged

- **WHEN** a fetch targets a host resolving to `192.168.1.20`
- **THEN** the decision request SHALL state the host and `private = true`

### Requirement: Redirects are reported honestly

`web-fetch` SHALL report the final URL after redirects in its result. Redirects
SHALL NOT be blocked on destination.

#### Scenario: Final URL reported

- **WHEN** a request is redirected to another URL
- **THEN** the tool result SHALL name the final URL
