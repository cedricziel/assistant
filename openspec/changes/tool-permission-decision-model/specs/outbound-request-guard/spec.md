## ADDED Requirements

### Requirement: Outbound HTTP tools block internal addresses

`web-fetch` and `workflow-http` SHALL resolve the target host and SHALL refuse
requests whose resolved address is loopback, private (RFC 1918), link-local
(including the cloud metadata address `169.254.169.254`), unique-local
(`fc00::/7`), unspecified, multicast, or an IPv4-mapped IPv6 form of any of
those. The connection SHALL be made to the validated address.

#### Scenario: Loopback refused

- **WHEN** `web-fetch` is called with `http://127.0.0.1:8080/`
- **THEN** the tool SHALL return an error and SHALL NOT connect

#### Scenario: Metadata endpoint refused

- **WHEN** the URL host resolves to `169.254.169.254`
- **THEN** the request SHALL be refused

#### Scenario: Hostname resolving to a private address refused

- **WHEN** a public-looking hostname resolves to `10.0.0.5`
- **THEN** the request SHALL be refused

### Requirement: Redirects are re-validated per hop

Automatic redirect following SHALL be disabled. Each redirect target SHALL pass
the same scheme and address checks before it is requested, and the chain SHALL
be capped at five hops.

#### Scenario: Redirect to internal address refused

- **WHEN** an allowed URL responds `302` with `Location: http://169.254.169.254/`
- **THEN** the second request SHALL NOT be sent AND the tool SHALL return an
  error

#### Scenario: Redirect cap

- **WHEN** a chain exceeds five hops
- **THEN** the tool SHALL return an error
