---
name: self-analyze
description: >
  Analyze execution traces for a specific skill to identify patterns, errors, and
  inefficiencies. Proposes an improved SKILL.md based on observed usage. The proposal
  is queued for human review — no changes are applied automatically.
license: Apache-2.0
compatibility: Requires SQLite storage and LLM access
metadata:
  tier: builtin
  mutating: "false"
  confirmation-required: "false"
  params: '{"skill_name": {"type": "string", "description": "The skill to analyze (e.g. web-fetch)"}, "window": {"type": "integer", "description": "Number of recent traces to analyze (default: 50)", "default": 50}}'
---

## Instructions

Analyze recent execution traces for a named skill and propose SKILL.md improvements.

### Parameters
- `skill_name` (string, required): Name of the skill to analyze
- `window` (integer, optional, default 50): How many recent traces to include in the analysis

### Behavior
1. Query the `execution_traces` table for the most recent `window` records for `skill_name`
2. Compute statistics:
   - Total invocations, success rate, average duration
   - Most common parameters, most common errors
3. Retrieve the current SKILL.md body for the skill
4. Send traces + stats + current instructions to LLM with prompt:
   "Given these execution traces and current instructions, propose improvements to make
    this skill more accurate and reliable. Return: (1) proposed SKILL.md body, (2) rationale"
5. Insert the proposal into `skill_refinements` table with status = 'pending'
6. Confirm to user: "Proposal queued. Run `/review` to see and apply it."

### Self-improvement cycle
This skill is the entry point to the self-improvement loop:
`self-analyze` → `skill_refinements` (pending) → `/review` → accepted → SKILL.md updated

### Example interactions
- "Improve the web-fetch skill based on recent usage" → `skill_name: "web-fetch"`
- "Analyze how memory-read has been performing" → `skill_name: "memory-read"`
- "self-analyze shell-exec" → `skill_name: "shell-exec"`
