## Context

The assistant has a memory system with six flat files (SOUL.md, IDENTITY.md, USER.md, TOOLS.md, MEMORY.md, AGENTS.md) and date-keyed daily notes. These are read/written via `memory-get` and `memory-append` builtin tools, with `memory-search` providing hybrid FTS+vector retrieval. The system works well for ephemeral observations but lacks structured, cross-referenced knowledge that persists and evolves across conversations.

Karpathy's "LLM Wiki" pattern proposes that an LLM incrementally builds and maintains a structured markdown wiki — synthesizing knowledge once rather than rediscovering it every query. This design adds wiki capabilities as a pure skill — no new tools, just instructions that teach the agent how to organize a wiki using its existing file I/O capabilities.

Skills are pure knowledge packages: a `SKILL.md` with YAML frontmatter and markdown instructions. They teach the agent how to behave but don't register tools or require Rust code changes.

## Goals / Non-Goals

**Goals:**

- Ship a builtin skill (`llm-wiki`) that teaches the agent the ingest/query/lint workflows
- Define conventions for wiki page format, directory layout, index, and activity log
- Store wiki pages as plain markdown in the agent's workspace directory (`wiki/` subdirectory)
- Keep the wiki fully optional — agents without the skill loaded behave identically to today
- Zero Rust code changes — the skill uses existing tools (`bash`, `memory-append`, file I/O)

**Non-Goals:**

- New builtin tools — the agent already has file read/write capabilities
- Embedding/vector indexing of wiki pages (future work — `memory-search` stays separate)
- UI for browsing or editing the wiki (agent-internal only)
- Multi-agent wiki sharing or federation
- Automatic ingestion triggers (the agent decides when to ingest based on skill instructions)

## Decisions

### D1: Pure skill, no new tools

**Decision:** The entire wiki system is implemented as a SKILL.md that teaches the agent conventions and workflows. The agent uses its existing tools (`bash` for file operations, `memory-search` for finding related content) to maintain the wiki.

**Rationale:** The agent already has all the capabilities needed to read, write, and list files. Adding dedicated wiki tools would duplicate existing functionality and add Rust code to maintain. A skill-only approach is simpler, faster to ship, and follows the project's philosophy that skills are knowledge packages. If tool-level guarantees (e.g., atomic index updates) prove necessary later, tools can be added incrementally.

**Alternatives considered:**

- _Three dedicated tools (`wiki-read`, `wiki-write`, `wiki-list`)_ — Would enforce conventions at the tool layer (e.g., auto-updating index on write). But this rigidity isn't needed at v1 — the LLM can follow conventions from instructions, and the lint workflow catches drift. Rejected as premature.

### D2: Wiki storage as flat markdown files in `wiki/` subdirectory

**Decision:** Wiki pages live at `<agent_workspace>/wiki/<page-name>.md`. The `wiki/` directory sits alongside existing memory files.

**Rationale:** Matches the project's existing approach (memory files are plain markdown on disk). No database schema changes, no migrations. Pages are human-readable and debuggable.

**Alternatives considered:**

- _SQLite table for wiki pages_ — Rejected; wiki should be inspectable by users.
- _Nested subdirectories by topic_ — Adds complexity without clear benefit at early scale.

### D3: Page format — YAML frontmatter + markdown body + cross-references

**Decision:** The skill instructs the agent to follow this page structure:

```markdown
---
title: Page Title
created: 2026-04-19T12:00:00Z
updated: 2026-04-19T12:00:00Z
tags: [topic-a, topic-b]
---

Content here...

## See Also

- [[related-page]]
- [[another-page]]
```

**Rationale:** Frontmatter gives structured metadata for future indexing. `[[wikilink]]` syntax is a well-known convention. Timestamps enable staleness detection during lint. This is a convention enforced by the skill's instructions, not by code.

### D4: Two index files maintained by convention

**Decision:** `wiki/index.md` is a topic catalog with one-line summaries per page. `wiki/log.md` is an append-only activity journal. The skill instructs the agent to update both whenever it modifies wiki pages.

**Rationale:** The index enables page discovery without listing the directory. The log provides an audit trail. Both are maintained by the agent following skill instructions — no tooling enforcement needed.

### D5: Skill includes reference examples and scripts

**Decision:** The skill directory includes a `references/` subdirectory with example wiki pages and index format, so the agent has concrete templates to follow.

**Rationale:** LLMs follow conventions better when given concrete examples rather than abstract rules. Reference files are discovered and served by the skill system automatically.

## Risks / Trade-offs

- **[Convention drift]** → The agent may deviate from wiki conventions over time since nothing enforces them programmatically. Mitigated by the lint workflow and clear examples in the skill's reference files. If drift becomes a problem, dedicated tools can be added later.
- **[Wiki page sprawl]** → The lint workflow instructs the agent to identify orphan and stale pages. The agent can delete files via bash when cleanup is needed.
- **[Index desync]** → If wiki files are edited outside the agent, index.md may drift. The lint workflow can rebuild the index by scanning the directory.
- **[System prompt bloat]** → The skill body is subject to the 20,000 char cap. The skill should be concise; reference files provide detail without inflating the main body.
- **[No search integration]** → Wiki pages are not indexed by `memory-search`. The agent discovers pages via the index and reads them with file tools. Vector indexing can be added later.
