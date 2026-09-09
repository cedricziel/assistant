## ADDED Requirements

### Requirement: Builtin llm-wiki skill exists with valid frontmatter

The system SHALL include a builtin skill at `skills/llm-wiki/SKILL.md` with valid YAML frontmatter containing `name: llm-wiki`, a `description`, and `allowed-tools` listing the existing tools the agent uses for wiki operations (e.g., `bash`, `memory-search`). The skill body SHALL contain markdown instructions for the agent.

#### Scenario: Skill is discovered and parseable

- **WHEN** the skill discovery process scans the builtin skills directory
- **THEN** it finds `skills/llm-wiki/SKILL.md` and parses it as a valid `SkillDef` with name `llm-wiki`

#### Scenario: Skill is synced to disk

- **WHEN** `sync_builtins_to_disk()` runs
- **THEN** the `llm-wiki` skill is written to the user's skills directory if not already present or outdated

### Requirement: Skill defines wiki directory layout and page format conventions

The skill body SHALL define the wiki directory structure: pages stored as `<agent_workspace>/wiki/<page-name>.md` with kebab-case names, an `index.md` catalog file, and a `log.md` activity journal. The skill SHALL specify the page format: YAML frontmatter (`title`, `created`, `updated`, `tags`) followed by a markdown body and an optional `## See Also` section using `[[page-name]]` wikilink syntax for cross-references.

#### Scenario: Agent creates a well-formed wiki page

- **WHEN** the agent follows the skill's instructions to create a wiki page
- **THEN** the resulting file has YAML frontmatter with title, created timestamp, updated timestamp, and tags, followed by markdown content and a See Also section with wikilinks

#### Scenario: Agent maintains the index

- **WHEN** the agent creates or updates a wiki page
- **THEN** it also updates `wiki/index.md` with a one-line entry for the page (name + summary), following the skill's instructions

#### Scenario: Agent maintains the activity log

- **WHEN** the agent creates or updates a wiki page
- **THEN** it appends a timestamped entry to `wiki/log.md` recording the action type (created/updated) and page name

### Requirement: Skill defines the ingest workflow

The skill body SHALL instruct the agent on the **ingest** workflow: when the agent acquires new knowledge (from conversation, documents, or tool output), it SHOULD determine whether the information is wiki-worthy (durable, structured knowledge vs. ephemeral observation), and if so, check the wiki index for existing related pages, then create a new page or update an existing one with synthesized knowledge, including cross-references in the `## See Also` section and reciprocal links on related pages.

#### Scenario: Agent ingests new knowledge

- **WHEN** the agent learns a significant new concept or durable fact during conversation
- **THEN** the skill instructs the agent to check the wiki index, decide whether to create or update a page, write the page with appropriate cross-references, update the index, and log the activity

#### Scenario: Agent adds reciprocal cross-references

- **WHEN** the agent creates or updates a wiki page that relates to existing pages
- **THEN** the skill instructs the agent to add `[[related-page]]` links in the See Also section and update related pages' See Also sections to link back

### Requirement: Skill defines the query workflow

The skill body SHALL instruct the agent on the **query** workflow: when answering a user question, the agent SHOULD first check the wiki index for relevant pages, read them, and use the structured wiki knowledge to provide a more informed answer. Optionally, valuable Q&A synthesis can be filed back into the wiki.

#### Scenario: Agent queries wiki before answering

- **WHEN** the agent receives a question that may be covered by wiki knowledge
- **THEN** the skill instructs the agent to check the wiki index, read relevant pages, and incorporate that knowledge into its response

### Requirement: Skill defines the lint workflow

The skill body SHALL instruct the agent on the **lint** workflow: a periodic self-check where the agent reviews the wiki for quality issues including stale pages (not updated recently), orphan pages (no incoming cross-references), missing cross-references between related pages, and contradictions across pages. The agent SHOULD fix issues it finds.

#### Scenario: Agent performs lint check

- **WHEN** the agent is asked to lint the wiki or performs a periodic check
- **THEN** the skill instructs the agent to read the index and log, identify stale/orphan pages, check for contradictions, and fix issues by updating pages

### Requirement: Skill distinguishes wiki from daily notes and memory files

The skill body SHALL explicitly state that the wiki is complementary to the existing memory system. Daily notes (`memory-append`) are for transient observations. Core memory files (SOUL.md, IDENTITY.md, etc.) are for identity and configuration. The wiki is for synthesized, structured, long-lived, cross-referenced knowledge.

#### Scenario: Agent distinguishes wiki from daily notes

- **WHEN** the agent encounters a transient observation (e.g., "user is debugging auth today")
- **THEN** the skill instructs the agent to use daily notes rather than the wiki
- **WHEN** the agent encounters a durable concept (e.g., "the project uses OAuth2 with PKCE flow")
- **THEN** the skill instructs the agent to write or update a wiki page

### Requirement: Skill includes reference examples

The skill directory SHALL include a `references/` subdirectory with example files demonstrating the expected wiki page format, index format, and log format, giving the agent concrete templates to follow.

#### Scenario: Reference files are available

- **WHEN** the skill is loaded
- **THEN** the `references/` directory contains at least an example wiki page, an example index.md, and an example log.md
