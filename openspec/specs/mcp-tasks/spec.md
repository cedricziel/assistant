# mcp-tasks Specification

## Purpose
Define how long-running MCP tool calls use the Tasks extension, both when the assistant serves them and when it calls external servers.

## Requirements

### Requirement: Server returns tasks for run-prompt
The server SHALL advertise the Tasks extension and, when the client declares it, SHALL answer `run-prompt` with a task instead of blocking. The task SHALL be retrievable through `tasks/get` before the response is sent. Clients that do not declare the extension SHALL get a blocking result.

#### Scenario: Task-aware client
- **WHEN** a client declaring the Tasks extension calls `run-prompt`
- **THEN** the result has `resultType: "task"` with a task id and status `working`

#### Scenario: Client without the extension
- **WHEN** a client not declaring the extension calls `run-prompt`
- **THEN** it receives the completed tool result directly

### Requirement: Task lifecycle
`tasks/get` SHALL report `working`, `completed`, `failed` or `cancelled`, include the tool result once completed, and treat a tool result with `isError: true` as `completed`. `tasks/cancel` SHALL stop the turn and move the task to `cancelled`. Unknown or expired task ids SHALL return invalid-params.

#### Scenario: Poll to completion
- **WHEN** a client polls a `run-prompt` task until the turn finishes
- **THEN** the final `tasks/get` has status `completed` and carries the tool result

#### Scenario: Cancel a task
- **WHEN** a client cancels a working task
- **THEN** a later `tasks/get` reports `cancelled`

### Requirement: Task isolation
Task ids SHALL be unguessable, and a task SHALL only be visible to the identity that created it.

#### Scenario: Another user's task
- **WHEN** user B calls `tasks/get` with user A's task id
- **THEN** the server returns invalid-params as if the task did not exist

### Requirement: Client consumes tasks
When calling external servers, the client SHALL declare the Tasks extension and, when a tool call returns a task, SHALL poll it at the server's suggested interval until it is terminal, bounded by the tool-call timeout, and SHALL cancel it when the turn is cancelled.

#### Scenario: External long-running tool
- **WHEN** an external tool returns a task that completes after several polls
- **THEN** the Persona receives the task's final tool result as the tool output

#### Scenario: Turn cancelled while task runs
- **WHEN** the user stops the turn while an external task is working
- **THEN** the client sends `tasks/cancel` for that task
