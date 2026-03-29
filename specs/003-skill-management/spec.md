# Feature Specification: Skill Management via Web UI and CLI

**Feature Branch**: `003-skill-management`
**Created**: 2026-03-29
**Status**: Draft
**Input**: User description: "users should be able to manage skills for the agent through the web-ui and the cli. skills can be global or scoped to a persona. for now, we want to basically crud skills and provide users with the ability to generate a skill with the help of an agent that knows the agentskills.io spec (meta skill?)."

## Clarifications

### Session 2026-03-29

- Q: How should persona-scoped skill access be modeled — separate per-persona skill storage, or a shared registry with access rules? → A: Single shared registry; per-persona whitelist/blacklist access rules; default is all skills allowed for all personas.
- Q: Are both whitelist and blacklist rule modes supported per persona, or only one? → A: Each persona operates in exactly one of three modes: "all" (every skill available, default), "whitelist" (only explicitly listed skills available), or "blacklist" (all skills except explicitly listed ones).
- Q: How does the AI generation agent source the agentskills.io specification? → A: Embedded as a builtin skill loaded at startup — always available, no network required.
- Q: Where is the authoritative copy of a skill stored when edited via web UI or CLI? → A: Both — edits write to `~/.assistant/skills/<name>/SKILL.md` on disk and sync to SQLite, keeping them in lockstep.
- Q: How should CLI skill and persona access commands be namespaced? → A: `assistant skill <action>` for skill CRUD; `assistant persona <action>` for persona access mode management.

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Browse and Manage Global Skills (Priority: P1)

A user opens the web UI and navigates to a Skills section where they can see all globally available skills (user-level and installed). They can view each skill's name, description, and source. They can create a new skill by filling out a form, edit an existing skill's SKILL.md content, or delete a skill they no longer want.

**Why this priority**: Global skill management is the foundation — all other stories build on it. It delivers immediate value even without persona scoping or AI generation.

**Independent Test**: Can be fully tested by navigating to /skills in the web UI, verifying the list renders, creating a new skill, editing it, and deleting it — with no persona or AI involvement required.

**Acceptance Scenarios**:

1. **Given** the web UI is running, **When** the user navigates to the Skills section, **Then** a list of all global skills is displayed with name, description, and source (builtin/user/installed) for each
2. **Given** the skills list is visible, **When** the user clicks "New Skill", fills in a name, description, and body, and submits, **Then** the new skill appears in the list and is immediately available to the agent
3. **Given** an existing user or installed skill, **When** the user clicks "Edit" and modifies the body, **Then** the changes are saved and reflected in the agent's skill context on next load
4. **Given** an existing user or installed skill, **When** the user clicks "Delete" and confirms, **Then** the skill is removed from the list and the agent no longer has access to it
5. **Given** a builtin skill, **When** the user views it, **Then** edit and delete controls are absent or disabled (builtins are read-only)

---

### User Story 2 - Manage Skills via CLI (Priority: P2)

A user interacts with the assistant CLI using new subcommands to list, create, show, edit, and delete skills without opening a browser. This supports scripting, headless servers, and developer workflows.

**Why this priority**: The CLI is the primary interface for many users and scripting scenarios; web UI alone is insufficient.

**Independent Test**: Can be fully tested in a terminal by running `assistant skill list`, `assistant skill create`, `assistant skill show <name>`, `assistant skill edit <name>`, and `assistant skill delete <name>`.

**Acceptance Scenarios**:

1. **Given** the CLI is installed, **When** the user runs `assistant skill list`, **Then** a table of all skills (name, source, description) is printed to stdout
2. **Given** the CLI, **When** the user runs `assistant skill create --name my-skill --description "..." --body-file ./body.md`, **Then** the skill is created and the user sees a success message
3. **Given** an existing skill named `my-skill`, **When** the user runs `assistant skill show my-skill`, **Then** the full skill details (frontmatter + body) are printed
4. **Given** an existing user skill, **When** the user runs `assistant skill delete my-skill --yes`, **Then** the skill is removed with a confirmation message
5. **Given** the CLI, **When** the user runs `assistant skill list --persona work`, **Then** the effective skill set for persona "work" (after applying its access mode) is listed

---

### User Story 3 - Control Which Skills a Persona Can Access (Priority: P3)

All skills are stored in a single shared registry. Each persona is configured in one of three access modes — "all" (default), "whitelist", or "blacklist" — which determines which skills from the registry are active when that persona is running.

**Why this priority**: A single-registry model avoids duplication; a three-mode access model covers the full range of use cases without per-skill storage complexity. Builds on global skill management (P1).

**Independent Test**: Can be tested by switching a persona to "blacklist" mode, adding one skill to its blacklist, running the agent, and confirming that skill is absent from context — without modifying the skill itself.

**Acceptance Scenarios**:

1. **Given** the web UI, **When** the user opens a persona's skill access settings, **Then** the current mode ("all", "whitelist", or "blacklist") is shown and can be changed
2. **Given** a persona in "all" mode, **When** the agent runs, **Then** every skill in the registry is available
3. **Given** a persona "work" set to "blacklist" mode with `git-commit` listed, **When** the agent runs with `--persona work`, **Then** `git-commit` is not loaded; all other skills are
4. **Given** a persona "focus" set to "whitelist" mode with only `web-search` listed, **When** the agent runs with `--persona focus`, **Then** only `web-search` is available; all other skills are excluded
5. **Given** the CLI, **When** the user runs `assistant persona skill-mode work blacklist`, **Then** the persona's mode is set to "blacklist" and a warning is shown if an existing skill list will now be interpreted as a blacklist
6. **Given** the CLI, **When** the user runs `assistant persona skill-add work git-commit` (persona in blacklist or whitelist mode), **Then** `git-commit` is added to that persona's list

---

### User Story 4 - AI-Assisted Skill Generation (Priority: P4)

A user requests the agent to generate a skill for them by describing what they want. The agent, informed by knowledge of the agentskills.io specification, produces a compliant SKILL.md draft which the user can review and save.

**Why this priority**: Lowers the barrier to skill authorship; requires all prior stories as a foundation since the output is saved as a manageable skill.

**Independent Test**: Can be tested end-to-end by sending a generation request, verifying the agent returns a valid SKILL.md structure, and confirming the user can save or discard it via the web UI or CLI.

**Acceptance Scenarios**:

1. **Given** the web UI skills creation page, **When** the user clicks "Generate with AI" and provides a plain-language description, **Then** the agent returns a SKILL.md draft pre-populated in the editor with valid frontmatter and body
2. **Given** the CLI, **When** the user runs `assistant skill generate "Teach the agent how to write git commit messages"`, **Then** the generated SKILL.md content is printed; the user can pipe or redirect it for review
3. **Given** a generated skill draft displayed to the user, **When** the user saves it, **Then** it is stored as a new user-scoped or persona-scoped skill
4. **Given** the generation agent, **When** it produces a skill, **Then** the output complies with agentskills.io spec structure (valid frontmatter fields: name, description, license, compatibility, allowed-tools)

---

### Edge Cases

- What happens when a user tries to create a skill with a name that already exists? The system rejects it with a clear duplicate-name error and suggests the user edit the existing skill instead.
- What happens when a skill body contains no frontmatter? The system requires at minimum `name` and `description` and presents a validation error before saving.
- How does the system handle deletion of a builtin skill? Builtin skills cannot be deleted or edited; the UI/CLI presents a descriptive error.
- What if a persona is deleted that had skill access rules configured? The persona's access mode and skill list are deleted along with the persona; the skills themselves remain unaffected in the registry.
- What happens when a persona's mode is changed (e.g., from "whitelist" to "blacklist")? The existing skill list is preserved but reinterpreted under the new mode. The user should be warned that the list now acts as a blacklist instead of a whitelist.
- What if a persona is in "whitelist" mode but its skill list is empty? No skills are loaded for that persona — an intentionally restricted persona with zero skills.
- What if the AI generation agent fails or times out? The user is shown an error and the editor remains empty so they can author manually.
- What if a filesystem write succeeds but the SQLite sync fails (or vice versa)? The operation is treated as failed; the user is shown an error and the partial write is rolled back or flagged for re-sync.
- What happens when a skill name contains invalid characters? The system enforces kebab-case naming (letters, digits, hyphens only; max 64 chars) and rejects non-conforming names with a helpful message.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: The web UI MUST provide a Skills section listing all discoverable skills (builtin, user, installed) with name, description, and source visible at a glance
- **FR-002**: The web UI MUST allow users to create a new skill by entering a name, description, and SKILL.md body content
- **FR-003**: The web UI MUST allow users to edit the body and description of existing user or installed skills; edits MUST be persisted to both the filesystem (`~/.assistant/skills/<name>/SKILL.md`) and SQLite in lockstep
- **FR-004**: The web UI MUST allow users to delete user or installed skills, with a confirmation step before permanent removal
- **FR-005**: The system MUST prevent creation or modification of builtin skills through the UI or CLI
- **FR-006**: The CLI MUST provide `assistant skill list` to display all skills; an optional `--persona <id>` flag shows the effective skill set for that persona after applying its access mode
- **FR-007**: The CLI MUST provide `assistant skill create` accepting name, description, and body or body-file
- **FR-008**: The CLI MUST provide `assistant skill show <name>` to display a skill's full content
- **FR-009**: The CLI MUST provide `assistant skill delete <name>` with a `--yes` flag to skip interactive confirmation
- **FR-009a**: The CLI MUST provide `assistant persona skill-mode <persona-id> <all|whitelist|blacklist>` to set a persona's access mode
- **FR-009b**: The CLI MUST provide `assistant persona skill-add <persona-id> <skill-name>` and `assistant persona skill-remove <persona-id> <skill-name>` to manage a persona's skill list (used in whitelist or blacklist mode)
- **FR-010**: All skills MUST be stored in a single shared registry; skills are not duplicated per persona
- **FR-011**: Each persona MUST have exactly one access mode: "all" (default), "whitelist", or "blacklist". Users MUST be able to change a persona's mode at any time.
- **FR-011a**: In "all" mode, every skill in the registry is loaded for that persona; no skill list is consulted.
- **FR-011b**: In "whitelist" mode, only skills explicitly listed for that persona are loaded; all others are excluded.
- **FR-011c**: In "blacklist" mode, all skills are loaded except those explicitly listed for that persona.
- **FR-011d**: Users MUST be able to add and remove skills from a persona's whitelist or blacklist from both the web UI and the CLI.
- **FR-012**: The web UI MUST provide an "Generate with AI" action on the skill creation page that accepts a plain-language description and returns a valid SKILL.md draft in the editor
- **FR-013**: The CLI MUST provide `assistant skill generate "<description>"` that prints a generated SKILL.md draft to stdout
- **FR-014**: The AI generation action MUST produce output conforming to agentskills.io spec structure (valid frontmatter with at minimum `name` and `description` fields)
- **FR-015**: Skill names MUST be validated as kebab-case (lowercase letters, digits, hyphens; max 64 characters) and the system MUST reject non-conforming names with a descriptive error
- **FR-016**: The system MUST prevent duplicate skill names within the same scope (global or per-persona)

### Key Entities

- **Skill**: A named knowledge package identified by a kebab-case name. Has a description, markdown body (instructions), allowed-tools list, license, compatibility, metadata, and source (builtin/user/installed/project). Skills are stored once and shared across all personas.
- **Persona**: An isolated agent context identified by an id (e.g. "default", "work"). Does not own skills; instead holds zero or more skill access rules.
- **Persona Access Mode**: Each persona is configured in exactly one of three modes:
  - **all** (default): every skill in the registry is available — no rule list required
  - **whitelist**: only the skills explicitly listed for this persona are available; all others are excluded
  - **blacklist**: all skills are available except those explicitly listed for this persona
- **Persona Skill List**: The set of skill names associated with a persona's whitelist or blacklist. Irrelevant (and ignored) when the persona is in "all" mode.

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: A user with no prior knowledge can create a new global skill in under 2 minutes using the web UI
- **SC-002**: All CRUD operations (create, list, edit, delete) for skills are reachable within 2 navigation steps from the web UI home page
- **SC-003**: Skill filtering by persona access mode is correct in all three modes: a blacklisted skill is absent, a non-whitelisted skill is absent, and "all" mode loads every skill — each verifiable by observing the agent's available skill set
- **SC-004**: The AI generation action produces a syntactically valid SKILL.md draft (parseable by the skill parser) for 95% of plain-language input descriptions
- **SC-005**: CLI subcommands are discoverable via `assistant skill --help` and `assistant persona --help`; all happy-path scenarios execute without error
- **SC-006**: Attempting to edit or delete a builtin skill via any interface results in a clear error message, never silent failure

## Assumptions

- Users are authenticated (or trust-based in single-user deployments); no per-skill access control beyond persona scoping is required for v1
- Skills are stored in a single flat registry; no per-persona duplication of skill records is needed
- Each persona record stores its access mode ("all", "whitelist", or "blacklist") and a separate list of skill names (the whitelist or blacklist entries); this is consulted at skill-load time to filter the registry for the active persona
- The agentskills.io specification is embedded as a builtin skill and loaded at startup; the AI generation agent uses this builtin as its knowledge source — no network access or user-provided file is required
- Skill body editing in the web UI uses a plain textarea (not a rich editor) for v1; a dedicated markdown editor is out of scope
- All writes to user/installed skills persist to both `~/.assistant/skills/<name>/SKILL.md` on disk and the SQLite registry atomically; the two stores are kept in lockstep
- Importing or bulk-uploading skill directories is out of scope for v1
- The `project`-source skill type (read from `<project>/.assistant/skills/`) remains filesystem-only and is not manageable via UI/CLI in v1
- If a persona is deleted, its scoped skills are promoted to global scope with a warning, preserving skill content
