## ADDED Requirements

### Requirement: PersonaEntity conforms to AppEntity

The system SHALL define a `PersonaEntity` conforming to `AppEntity` with properties `id` (String), `displayRepresentation` (using `name`), and `description` (String). The entity SHALL provide a `defaultQuery` that fetches personas from `GET /api/personas`.

#### Scenario: Dynamic persona list in Shortcuts picker

- **WHEN** a user configures an intent parameter of type `PersonaEntity` in the Shortcuts editor
- **THEN** the system queries `GET /api/personas` on the active server
- **THEN** the picker displays each persona's name

#### Scenario: Persona lookup by ID

- **WHEN** the system needs to resolve a `PersonaEntity` by its ID
- **THEN** the entity query calls `GET /api/personas/{id}` (or filters from list)
- **THEN** the entity is returned with the correct name and description

#### Scenario: Server unreachable during entity query

- **WHEN** the server is not reachable during an entity query
- **THEN** the query returns an empty array (no crash)

---

### Requirement: WorkflowEntity conforms to AppEntity

The system SHALL define a `WorkflowEntity` conforming to `AppEntity` with properties `id` (String), `displayRepresentation` (using `name`), `description` (String), and `active` (Bool). The entity SHALL provide a `defaultQuery` that fetches workflows from `GET /api/workflows`.

#### Scenario: Dynamic workflow list in Shortcuts picker

- **WHEN** a user configures an intent parameter of type `WorkflowEntity` in the Shortcuts editor
- **THEN** the system queries `GET /api/workflows` on the active server
- **THEN** the picker displays each workflow's name

#### Scenario: Workflow lookup by ID

- **WHEN** the system needs to resolve a `WorkflowEntity` by its ID
- **THEN** the entity query fetches the workflow and returns it with correct name, description, and active status

#### Scenario: Server unreachable during entity query

- **WHEN** the server is not reachable during a workflow entity query
- **THEN** the query returns an empty array

---

### Requirement: ConversationEntity conforms to AppEntity

The system SHALL define a `ConversationEntity` conforming to `AppEntity` with properties `id` (String), `displayRepresentation` (using `title`), and `updatedAt` (String, RFC 3339). The entity SHALL provide a `defaultQuery` that fetches conversations from `GET /api/conversations`.

#### Scenario: Dynamic conversation list in Shortcuts picker

- **WHEN** a user configures an intent parameter of type `ConversationEntity` in the Shortcuts editor
- **THEN** the system queries `GET /api/conversations` on the active server
- **THEN** the picker displays each conversation's title

#### Scenario: Conversation lookup by ID

- **WHEN** the system needs to resolve a `ConversationEntity` by its ID
- **THEN** the entity query calls `GET /api/conversations/{id}`
- **THEN** the entity is returned with correct title and updated timestamp

#### Scenario: Server unreachable during entity query

- **WHEN** the server is not reachable during a conversation entity query
- **THEN** the query returns an empty array

---

### Requirement: Entities support string search filtering

Each entity query SHALL support the `suggestedEntities()` method (returning all entities) and `entities(matching:)` method that filters entities by name/title using case-insensitive substring matching.

#### Scenario: Search filters personas by name

- **WHEN** a user types "code" in the persona picker search field
- **THEN** only personas whose name contains "code" (case-insensitive) are shown

#### Scenario: Search filters workflows by name

- **WHEN** a user types "morning" in the workflow picker search field
- **THEN** only workflows whose name contains "morning" (case-insensitive) are shown

#### Scenario: Empty search returns all entities

- **WHEN** the search string is empty
- **THEN** all entities are returned
