# http-client-injection Specification

## Purpose
TBD - created by archiving change workspace-test-coverage-floor. Update Purpose after archive.
## Requirements
### Requirement: outbound HTTP types accept an injected `reqwest::Client`

Any type that issues outbound HTTP requests SHALL accept a
`reqwest::Client` at construction time. The type MUST NOT call
`reqwest::Client::new()` (or `reqwest::Client::builder().build()`)
inside method bodies or trait-method implementations.

In-scope types include but are not limited to:

- `assistant_workflow_http::HttpRequestActionExecutor`
- `assistant_web_ui::push::WebPushClient`
- `assistant_interfaces::nextcloud::adapter::NextcloudAdapter` and
  `assistant_interfaces::nextcloud::tools::*`
- `assistant_interfaces::matrix::client::MatrixClient`
- `assistant_interfaces::slack::client::SlackApiClient`
- `assistant_interfaces::mattermost::client::*`
- `assistant_interfaces::signal::client::*`
- `assistant_auth::oidc::OidcProvider`
- `assistant_llm_provider::*` providers
- `assistant_interface_cli::cmd_login` flow types

#### Scenario: constructor accepts a client

- **WHEN** an in-scope type is constructed
- **THEN** its primary constructor accepts a `reqwest::Client` (or a
  thin abstraction over it) as a parameter, or provides a
  `with_client(client, ...)` builder alongside any zero-argument
  convenience constructor

#### Scenario: method bodies do not construct clients

- **WHEN** any method on an in-scope type is invoked
- **THEN** the method uses `self.client` (or equivalent injected
  field) and does not instantiate a new `reqwest::Client`

### Requirement: configurable base URL for `wiremock` redirection

Any type that issues outbound HTTP to a fixed third-party host SHALL
expose a `with_base_url(...)` (or equivalent) builder so tests can
redirect traffic to a `wiremock::MockServer`. Hard-coded host strings
inside method bodies are forbidden.

#### Scenario: test redirects traffic to wiremock

- **WHEN** a test constructs an in-scope HTTP client with
  `Type::with_base_url(mock_server.uri())`
- **THEN** all subsequent HTTP calls from that instance target the
  mock server, allowing the test to assert request shape and inject
  mock responses

### Requirement: workspace lint enforces no in-method client construction

A `tests/workspace_http_client_lint.rs` integration test SHALL fail
the build when `reqwest::Client::new()` or
`reqwest::Client::builder()` appears in non-test code outside of:

- a constructor / `Default` impl
- a top-level binary entry point (`main.rs`)
- a documented `new_with_default_client(...)` helper

#### Scenario: lint catches in-method construction

- **WHEN** a contributor adds `let client = reqwest::Client::new();`
  to a method body in `crates/*/src/`
- **THEN** `cargo test -p assistant tests::workspace_http_client_lint`
  fails with a message naming the file and line

### Requirement: existing constructors remain back-compatible

The injection refactor SHALL preserve existing zero-argument
constructors via convenience wrappers that build a default
`reqwest::Client` internally. Production call sites do not need
to change unless they explicitly want to inject a custom client.

#### Scenario: existing `new()` still works

- **WHEN** existing production code calls `Type::new(...)` without
  passing a client
- **THEN** the call compiles and uses a default `reqwest::Client`
  internally

