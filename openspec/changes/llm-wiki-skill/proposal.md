## Why

The assistant's memory system stores facts as append-only notes and flat markdown files (SOUL.md, MEMORY.md, daily notes). This works for short-lived context but breaks down for building structured, interconnected knowledge over time — there's no synthesis, no cross-referencing, and no way to organize knowledge by topic or entity. Inspired by Karpathy's "LLM Wiki" concept, a builtin skill can teach the agent to incrementally build and maintain a personal wiki alongside (not replacing) the existing memory files, turning raw observations into a curated, navigable knowledge base.

## What Changes

- **New builtin skill `llm-wiki`** — A SKILL.md that instructs the agent how to maintain a wiki directory structure inside its agent workspace (`~/.assistant/agents/<id>/wiki/`) using the tools it already has (`memory-append`, `bash`, file I/O tools).
- **Wiki conventions** — The skill defines a page format (YAML frontmatter + markdown body + cross-references), two index files (`wiki/index.md` catalog and `wiki/log.md` activity journal), and `[[wikilink]]` syntax for cross-referencing.
- **Three core workflows** — The skill teaches the agent:
  - **Ingest**: When the agent learns new information, it creates/updates wiki pages, maintains cross-references, and logs the activity.
  - **Query**: When answering questions, the agent checks the wiki for structured context before responding.
  - **Lint**: Periodic self-check to find stale pages, contradictions, orphans, and missing links.
- **No new tools** — The agent uses existing file I/O capabilities to read, write, and list wiki pages. The skill is purely instructional.

## Non-goals

- **Not replacing the memory system** — The wiki augments MEMORY.md and daily notes; it does not replace SOUL.md, IDENTITY.md, or the core memory files.
- **Not a RAG pipeline** — No embedding-based retrieval for wiki pages in this change. The existing `memory-search` vector search remains separate.
- **Not user-facing UI** — No Flutter screens for browsing/editing the wiki. The wiki is an agent-internal knowledge structure.
- **Not multi-agent shared wikis** — Each agent has its own wiki directory; no cross-agent wiki federation.
- **Not new builtin tools** — No new Rust tool handlers. The agent operates on wiki files using its existing tool set.

## Capabilities

### New Capabilities

- `wiki-skill`: The builtin SKILL.md that teaches the agent wiki conventions and maintenance workflows (ingest, query, lint) using existing tools.

### Modified Capabilities

_(none)_

## Impact

- **Crates affected**: `assistant-skills` only (new embedded skill via `include_dir!`)
- **Storage**: New `wiki/` directory inside agent workspace; no schema/migration changes (plain markdown files on disk)
- **System prompt**: When the skill is loaded, wiki instructions are injected into context
- **Existing tools**: No changes — the agent uses `bash`, `memory-append`, and file tools it already has
