## 1. Skill file

- [ ] 1.1 Create `skills/llm-wiki/SKILL.md` with YAML frontmatter (`name: llm-wiki`, `description`, `allowed-tools: bash memory-search`)
- [ ] 1.2 Write the wiki conventions section: directory layout (`wiki/`), page format (YAML frontmatter + body + See Also), index.md format, log.md format, kebab-case page naming
- [ ] 1.3 Write the ingest workflow section: when/how to create or update wiki pages from new knowledge, cross-reference maintenance with `[[wikilink]]` syntax, index and log updates
- [ ] 1.4 Write the query workflow section: checking the wiki index before answering questions, reading relevant pages, incorporating wiki knowledge
- [ ] 1.5 Write the lint workflow section: periodic quality checks (stale pages, orphans, contradictions, missing links), how to fix issues
- [ ] 1.6 Write the disambiguation section: when to use wiki vs daily notes vs MEMORY.md vs core memory files

## 2. Reference examples

- [ ] 2.1 Create `skills/llm-wiki/references/example-page.md` showing the expected wiki page format with frontmatter, content, and See Also section
- [ ] 2.2 Create `skills/llm-wiki/references/example-index.md` showing the expected index.md format with page entries and summaries
- [ ] 2.3 Create `skills/llm-wiki/references/example-log.md` showing the expected log.md format with timestamped activity entries

## 3. Verification

- [ ] 3.1 Run `make lint` and `make format` — fix any issues
- [ ] 3.2 Run `make test` — ensure all existing tests pass (no Rust changes, but verify skill discovery still works)
- [ ] 3.3 Verify the skill is discovered by the embedded skills system (`include_dir!("skills")` picks up the new directory)
