---
name: agentskills-spec
description: Authoritative specification for the Agent Skills (agentskills.io) SKILL.md format. Use this skill when generating or validating SKILL.md files.
license: MIT
---

## Agent Skills Specification

A **skill** is a directory containing a `SKILL.md` file that teaches an AI agent how to perform a task or use a domain of knowledge. This specification defines the required format.

### Directory Structure

```
<skill-name>/
├── SKILL.md          # Required — frontmatter + body
├── scripts/          # Optional — executable scripts
├── references/       # Optional — reference documents
└── assets/           # Optional — binary assets
```

### SKILL.md Format

```
---
name: <kebab-case-name>
description: <human-readable description>
license: <SPDX license identifier>
compatibility: <optional version or platform constraint>
allowed-tools: <space-separated kebab-case tool IDs (e.g. file-read web-fetch bash)>
metadata:
  key: value
---

<markdown body — instructions for the agent>
```

### Frontmatter Fields

| Field           | Required | Constraints                                                                              |
| --------------- | -------- | ---------------------------------------------------------------------------------------- |
| `name`          | Yes      | kebab-case (lowercase letters, digits, hyphens); max 64 chars; must match directory name |
| `description`   | Yes      | Non-empty; max 1024 chars                                                                |
| `license`       | No       | SPDX identifier (e.g. MIT, Apache-2.0)                                                   |
| `compatibility` | No       | Max 500 chars; version constraint or platform note                                       |
| `allowed-tools` | No       | Space-separated kebab-case tool IDs (e.g. `file-read web-fetch bash`)                    |
| `metadata`      | No       | Arbitrary key/value map for custom attributes                                            |

### Body

- Written in Markdown
- Contains instructions the agent should follow when this skill is active
- Use `##` headings to organize sections
- Include examples, constraints, and step-by-step guidance

### Validation Rules

1. `name` must be kebab-case: only lowercase ASCII letters, digits, and non-leading/trailing/consecutive hyphens
2. `name` must match the directory name exactly
3. `description` must be non-empty and ≤ 1024 characters
4. `compatibility` if present must be ≤ 500 characters
5. The SKILL.md must have valid YAML frontmatter delimited by `---`

### Example

```
---
name: git-commit
description: Teaches the agent to write semantic git commit messages following the Conventional Commits specification.
license: MIT
allowed-tools: bash file-read
---

## Conventional Commits

Always format commit messages as: `<type>(<scope>): <description>`

Types: feat, fix, chore, docs, refactor, test, perf

### Rules
- Subject line max 72 characters
- Use imperative mood ("add feature" not "added feature")
- Reference issues with (#123) at end of subject
```

### When Generating a SKILL.md

1. Choose a `name` that is kebab-case and describes the skill's purpose
2. Write a `description` that is one clear sentence (≤ 1024 chars)
3. Set `license: MIT` unless the user specifies otherwise
4. List any tools the skill requires in `allowed-tools`
5. Write a body with `##` sections: purpose, rules/constraints, examples
6. Keep instructions actionable and specific — avoid vague language
7. Output the complete SKILL.md content including the `---` frontmatter delimiters
