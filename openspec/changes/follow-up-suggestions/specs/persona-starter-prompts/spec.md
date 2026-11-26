## ADDED Requirements

### Requirement: Personas store starter prompts

The `personas` table in `space.db` SHALL have a `starter_prompts` column holding a JSON array of strings, defaulting to `[]`. `PersonaRecord` SHALL expose `starter_prompts: Vec<String>`. `PersonaStore` SHALL provide `set_starter_prompts(id, prompts)` in both the SQLite and the in-memory implementation. If the column holds a value that cannot be parsed, it SHALL read as an empty list.

#### Scenario: New persona has no starters

- **WHEN** a Persona is created
- **THEN** its `starter_prompts` SHALL be empty

#### Scenario: Round-trip

- **WHEN** `set_starter_prompts("p1", ["Plan my week", "Summarize inbox"])` is called
- **THEN** `get("p1")` SHALL return those prompts in that order

#### Scenario: Malformed column reads as empty

- **GIVEN** a row whose `starter_prompts` is `not json`
- **WHEN** the Persona is read
- **THEN** `starter_prompts` SHALL be empty AND the read SHALL succeed

### Requirement: Persona API exposes and updates starter prompts

`PersonaDetail` SHALL include `starter_prompts` (an array, always present) and `can_edit_starter_prompts` (a boolean that is true exactly when the caller may `PATCH` this Persona's starters). `PATCH /api/personas/{id}` (`operationId: update_persona`) SHALL take `UpdatePersonaRequest { starter_prompts?: string[] }`. It SHALL store the prompts trimmed and return `200` with the updated `PersonaDetail`. A field that is absent SHALL leave its value unchanged. More than 4 prompts, or a prompt that is empty after trimming or longer than 200 characters, SHALL return `400` with `{"error": "..."}`. Lengths SHALL count Unicode scalar values after trimming. An unknown or inaccessible Persona SHALL return `404`. A caller without space write access SHALL get `403`. For a personal Persona (owner set) only its owner SHALL be allowed to update starters. For an org-owned Persona (no owner) only a space admin (or org admin) SHALL be allowed. Any other caller with write access SHALL get `403` and nothing SHALL be stored. The route SHALL be documented in `openapi.json`.

#### Scenario: Update starters

- **WHEN** `PATCH /api/personas/p1` is sent with `{"starter_prompts": ["  Plan my week "]}`
- **THEN** the response SHALL be `200` with `starter_prompts == ["Plan my week"]`

#### Scenario: Too many starters

- **WHEN** the body contains 5 prompts
- **THEN** the response SHALL be `400` with an `error` field AND nothing SHALL be stored

#### Scenario: Empty list clears

- **WHEN** the body is `{"starter_prompts": []}`
- **THEN** the Persona's starters SHALL be empty

#### Scenario: Empty body leaves starters unchanged

- **GIVEN** a Persona with starters `["Plan my week"]`
- **WHEN** the body is `{}`
- **THEN** the response SHALL be `200` with `starter_prompts == ["Plan my week"]`

#### Scenario: Blank or over-long prompt is rejected

- **WHEN** the body contains a prompt that is only whitespace, or one of 201 characters after trimming
- **THEN** the response SHALL be `400` with an `error` field AND nothing SHALL be stored

#### Scenario: Length counts characters, not bytes

- **WHEN** the body contains a prompt of 200 non-ASCII characters (more than 200 bytes in UTF-8)
- **THEN** the response SHALL be `200`

#### Scenario: Unknown Persona

- **WHEN** `PATCH /api/personas/missing` is sent
- **THEN** the response SHALL be `404` with an `error` field

#### Scenario: Owner edits a personal Persona

- **GIVEN** a Persona owned by the caller, who is a space member
- **WHEN** the caller sends `PATCH /api/personas/p1` with valid starters
- **THEN** the response SHALL be `200` AND `can_edit_starter_prompts` SHALL be `true`

#### Scenario: Space admin edits an org-owned Persona

- **GIVEN** an org-owned Persona and a caller who is a space admin
- **WHEN** the caller sends `PATCH /api/personas/p1` with valid starters
- **THEN** the response SHALL be `200`

#### Scenario: Member cannot edit an org-owned Persona

- **GIVEN** an org-owned Persona and a caller who is a space member but not an admin
- **WHEN** the caller sends `PATCH /api/personas/p1`
- **THEN** the response SHALL be `403` AND nothing SHALL be stored
- **AND** `GET /api/personas/p1` for that caller SHALL report `can_edit_starter_prompts == false`

#### Scenario: Read-only caller

- **GIVEN** a caller with read-only access to the space
- **WHEN** the caller sends `PATCH /api/personas/p1`
- **THEN** the response SHALL be `403` AND nothing SHALL be stored

### Requirement: Starter prompts are editable in the Persona screens

The Persona detail screen SHALL show a "Starter prompts" section that lists the current prompts and opens an editor. The editor SHALL allow adding (up to 4), editing, reordering and removing prompts. It SHALL validate the same limits as the API before saving, and it SHALL save through `update_persona`. When `can_edit_starter_prompts` is false, the section SHALL list the prompts read-only and SHALL NOT offer the editor. The view model SHALL expose loading, error and success states. After a successful save, the Persona detail and the chat empty state SHALL show the new list without a manual refresh.

#### Scenario: Save from the editor

- **WHEN** the user adds "Plan my week" and saves
- **THEN** `update_persona` SHALL be called with that list AND the detail screen SHALL show it

#### Scenario: Saved starters reach the empty chat

- **GIVEN** the active Persona's starters were edited and saved
- **WHEN** the user opens a new chat
- **THEN** the empty state SHALL show the saved starters

#### Scenario: No edit permission hides the editor

- **GIVEN** an org-owned Persona viewed by a member who is not an admin
- **THEN** the "Starter prompts" section SHALL list the prompts AND SHALL NOT offer an edit action

#### Scenario: Fifth prompt is blocked

- **GIVEN** 4 prompts in the editor
- **THEN** the add affordance SHALL be disabled

### Requirement: Empty chat shows the active Persona's starter prompts

When the thread is empty, the chat screen SHALL render `StarterPrompts` with the active Persona's `starter_prompts` in `ThreadEmptyState.starters`. Tapping a starter SHALL call `ChatNotifier.sendMessage(prompt)`. If the Persona has no starters, or while the Persona is loading or failed to load, only the default empty-state copy SHALL render. There SHALL be no org-level fallback list: starters come only from the active Persona.

#### Scenario: Starter tap sends

- **GIVEN** the active Persona has starter "Plan my week"
- **WHEN** the user opens a new chat and taps it
- **THEN** a conversation SHALL be created AND "Plan my week" SHALL be sent as the first user message

#### Scenario: No starters keeps default copy

- **GIVEN** the active Persona has no starters, whatever other Personas in the org have
- **THEN** the empty state SHALL show "Start a conversation" and no starter cards

### Requirement: StarterPrompts element

`app/packages/assistant_ui` SHALL provide a pure `StarterPrompts(prompts, onSelected)` that renders one card per prompt, keyed `starter_prompt_<index>`. It SHALL use one column on narrow layouts and two columns when the available width is at least 600 dp, chosen from the layout constraints and never from the platform. An empty list SHALL render nothing. The widget SHALL have a widgetbook use-case and goldens for both layouts, and SHALL appear in `kAssistantUiElements`.

#### Scenario: Two columns when wide

- **GIVEN** 4 prompts and a 720 dp-wide constraint
- **THEN** the cards SHALL be arranged in 2 columns

#### Scenario: Tap emits the prompt

- **WHEN** `starter_prompt_0` is tapped
- **THEN** `onSelected` SHALL fire with the first prompt
