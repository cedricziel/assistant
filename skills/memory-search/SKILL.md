---
name: memory-search
description: >
  Search through all persistent memory entries for a keyword or phrase. Use when the user
  asks what you remember in general, wants to find stored information without knowing
  the exact key, or asks to list everything you know about a topic.
license: Apache-2.0
compatibility: Requires SQLite storage
metadata:
  tier: builtin
  mutating: "false"
  confirmation-required: "false"
  params: '{"query": {"type": "string", "description": "Search term to match against keys and values"}}'
---

## Instructions

Search the persistent memory store for entries matching a keyword or phrase.

### Parameters
- `query` (string, required): The search term to look for in both keys and values (case-insensitive substring match)

### Behavior
- Searches both keys AND values for the query string
- Returns all matching entries sorted by last-updated date
- If no matches: state that nothing was found

### Example interactions
- "What do you remember about my projects?" → `query: "project"`
- "Do you know anything about Python?" → `query: "python"`
- "What have I told you about myself?" → `query: "user"`
- "Show me everything you remember" → `query: ""`  (empty = list all)

### Output format
List matching entries as a short bulleted list:
- `user_name`: Alice (saved 3 days ago)
- `project_name`: Assistant (saved 1 week ago)

If no matches: "I don't have any memories matching '{query}'."
