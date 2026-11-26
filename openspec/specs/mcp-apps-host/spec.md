# mcp-apps-host Specification

## Purpose
Define how the chat renders MCP Apps (`ui://` resources) declared by external servers' tools.

## Requirements

### Requirement: App resolution
When a called external tool declares a `ui://` resource URI, the assistant SHALL fetch that resource from the same server, accept only the `text/html;profile=mcp-app` MIME type, and attach the app to the tool call so the chat can render it, including after reload from history.

#### Scenario: Tool with an app
- **WHEN** an external tool with a `ui://` resource URI is called
- **THEN** the tool call carries the app's HTML, CSP and permission metadata

#### Scenario: Wrong MIME type
- **WHEN** the `ui://` resource is served as `text/html` without the app profile
- **THEN** no app is attached and the tool call renders as a plain chip

### Requirement: Sandboxed rendering
The chat SHALL render the app in a sandbox isolated from the assistant's origin and credentials, with a Content Security Policy built only from the domains the resource declares and the spec's restrictive default when none are declared.

#### Scenario: Undeclared network access
- **WHEN** app code fetches a domain not in its declared connect domains
- **THEN** the request is blocked

#### Scenario: Assistant credentials
- **WHEN** app code tries to read the assistant's cookies or storage
- **THEN** it cannot access them

### Requirement: Host bridge
The chat SHALL answer `ui/initialize` with host info, theme and display context, send the tool input and tool result after the app reports initialized, resize the frame on size-change notifications, and send teardown before removing the frame.

#### Scenario: App receives the result
- **WHEN** an app finishes initializing for a completed tool call
- **THEN** it receives the tool input and then the tool result

### Requirement: App-initiated calls
Tool calls and resource reads from an app SHALL be proxied only to the app's own server, only for tools whose visibility includes `app`, and SHALL go through the permission policy; tools not marked read-only SHALL require user confirmation. Link-open requests SHALL require user confirmation.

#### Scenario: App calls a mutating tool
- **WHEN** an app calls a tool without a read-only hint
- **THEN** the user is asked to confirm before the call is sent

#### Scenario: App calls another server's tool
- **WHEN** an app calls a tool belonging to a different server
- **THEN** the call is rejected with an error
