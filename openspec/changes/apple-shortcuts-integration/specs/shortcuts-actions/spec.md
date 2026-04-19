## ADDED Requirements

### Requirement: AskAssistant intent accepts optional persona and context

The `AskAssistantIntent` SHALL accept three parameters: `question` (String, required), `persona` (PersonaEntity, optional), and `context` (String, optional). When `persona` is provided, the intent SHALL include `persona_id` in the `POST /api/quick-message` request. When `context` is provided, the intent SHALL include `context` in the request body. The intent SHALL return the assistant's answer as a dialog result.

#### Scenario: Ask with default persona (no persona specified)

- **WHEN** the user invokes AskAssistant with only a question
- **THEN** the intent sends `{ "message": "<question>" }` to `POST /api/quick-message`
- **THEN** the server uses its active persona
- **THEN** the answer is returned as a Siri dialog

#### Scenario: Ask with specific persona

- **WHEN** the user invokes AskAssistant with a question and selects a PersonaEntity
- **THEN** the intent sends `{ "message": "<question>", "persona_id": "<id>" }`
- **THEN** the server routes the question to the specified persona
- **THEN** the answer is returned as a Siri dialog

#### Scenario: Ask with context

- **WHEN** the user invokes AskAssistant with a question and provides context text (e.g., clipboard contents)
- **THEN** the intent sends `{ "message": "<question>", "context": "<context>" }`
- **THEN** the server includes the context when processing the question

#### Scenario: Server unreachable

- **WHEN** the server cannot be reached
- **THEN** the intent returns a dialog: "I couldn't reach your assistant server. Please check your connection."

#### Scenario: No credentials configured

- **WHEN** no server credentials exist in the Keychain
- **THEN** the intent returns a dialog: "Please open the app and connect to your assistant server first."

#### Scenario: Timeout

- **WHEN** the server does not respond within 25 seconds
- **THEN** the intent returns a dialog: "I'm still working on that. Check the app for the full answer."

---

### Requirement: RunWorkflow intent triggers a workflow

The `RunWorkflowIntent` SHALL accept a `workflow` parameter (WorkflowEntity, required). It SHALL call `POST /api/workflows/{id}/test-run` with the workflow's ID. The intent SHALL return a dialog indicating the workflow was triggered and include the run ID.

#### Scenario: Successful workflow trigger

- **WHEN** the user selects a workflow and runs the intent
- **THEN** the intent calls `POST /api/workflows/{id}/test-run`
- **THEN** the intent returns a dialog: "Workflow '<name>' started. Run ID: <run_id>"

#### Scenario: Workflow not found

- **WHEN** the workflow ID no longer exists on the server (deleted since entity was cached)
- **THEN** the intent returns a dialog: "Workflow not found. It may have been deleted."

#### Scenario: Server unreachable

- **WHEN** the server cannot be reached
- **THEN** the intent returns a dialog: "I couldn't reach your assistant server. Please check your connection."

---

### Requirement: ListPersonas intent returns persona entities

The `ListPersonasIntent` SHALL accept no required parameters. It SHALL call `GET /api/personas` and return the result as an array of `PersonaEntity` values. This allows the output to be piped into other Shortcut actions.

#### Scenario: Personas returned successfully

- **WHEN** the user runs the ListPersonas intent
- **THEN** the intent returns an array of PersonaEntity with id, name, and description

#### Scenario: No personas exist

- **WHEN** the server returns an empty personas list
- **THEN** the intent returns an empty array

#### Scenario: Server unreachable

- **WHEN** the server cannot be reached
- **THEN** the intent returns a dialog error message

---

### Requirement: ListWorkflows intent returns workflow entities

The `ListWorkflowsIntent` SHALL accept an optional `activeOnly` parameter (Bool, default false). It SHALL call `GET /api/workflows` and optionally filter to only active workflows. It SHALL return the result as an array of `WorkflowEntity` values.

#### Scenario: All workflows returned

- **WHEN** the user runs ListWorkflows without activeOnly
- **THEN** all workflows are returned as WorkflowEntity values

#### Scenario: Only active workflows returned

- **WHEN** the user runs ListWorkflows with activeOnly = true
- **THEN** only workflows where `active == true` are returned

#### Scenario: Server unreachable

- **WHEN** the server cannot be reached
- **THEN** the intent returns a dialog error message

---

### Requirement: ListConversations intent returns conversation entities

The `ListConversationsIntent` SHALL accept an optional `limit` parameter (Int, default 20). It SHALL call `GET /api/conversations` and return up to `limit` conversations as an array of `ConversationEntity` values, ordered by most recent first.

#### Scenario: Recent conversations returned

- **WHEN** the user runs ListConversations with default limit
- **THEN** the 20 most recent conversations are returned as ConversationEntity values

#### Scenario: Custom limit

- **WHEN** the user runs ListConversations with limit = 5
- **THEN** at most 5 conversations are returned

#### Scenario: Server unreachable

- **WHEN** the server cannot be reached
- **THEN** the intent returns a dialog error message

---

### Requirement: AppShortcutsProvider registers Siri phrases

The `AssistantShortcutsProvider` SHALL register discoverable Siri phrases for all intents. At minimum, the following phrases SHALL be registered:

- "Ask [app name] a question"
- "Ask [app name] something"
- "Run [app name] workflow"
- "Show my [app name] personas"
- "Show my [app name] workflows"
- "Show my [app name] conversations"

#### Scenario: Siri discovers ask phrases

- **WHEN** the user says "Ask Assistant a question"
- **THEN** Siri invokes the AskAssistantIntent and prompts for the question

#### Scenario: Siri discovers workflow phrase

- **WHEN** the user says "Run Assistant workflow"
- **THEN** Siri invokes the RunWorkflowIntent and prompts for which workflow

#### Scenario: All actions visible in Shortcuts app

- **WHEN** the user opens the Shortcuts app and searches for the app name
- **THEN** all registered actions (Ask, RunWorkflow, ListPersonas, ListWorkflows, ListConversations) appear

---

### Requirement: Backend quick-message accepts optional persona_id and context

The `POST /api/quick-message` endpoint SHALL accept an enhanced request body with optional fields `persona_id` (string) and `context` (string). When `persona_id` is provided, the handler SHALL resolve that persona for the turn instead of the server's active persona. When `context` is provided, the handler SHALL prepend it to the user message as additional context. When neither field is provided, behavior SHALL be identical to the current implementation.

#### Scenario: Request with persona_id routes to specified persona

- **WHEN** a client sends `{ "message": "hello", "persona_id": "code-reviewer" }`
- **THEN** the server creates a conversation under the "code-reviewer" persona
- **THEN** the response is generated by the "code-reviewer" persona's configuration

#### Scenario: Request with invalid persona_id falls back to active persona

- **WHEN** a client sends `{ "message": "hello", "persona_id": "nonexistent" }`
- **THEN** the server falls back to the active persona
- **THEN** the response is generated normally (no error)

#### Scenario: Request with context includes context in processing

- **WHEN** a client sends `{ "message": "summarize this", "context": "Lorem ipsum..." }`
- **THEN** the server processes the message with the context text included

#### Scenario: Request without optional fields is backward-compatible

- **WHEN** a client sends `{ "message": "hello" }` (no persona_id, no context)
- **THEN** the behavior is identical to the current implementation
- **THEN** the server uses its active persona

#### Scenario: OpenAPI spec updated

- **WHEN** inspecting the OpenAPI specification at `/api/openapi.json`
- **THEN** the `QuickMessageRequest` schema includes optional `persona_id` (string) and `context` (string) fields
