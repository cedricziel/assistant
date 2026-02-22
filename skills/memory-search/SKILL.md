---
name: memory-search
description: >
  Search across all indexed memory files (SOUL.md, IDENTITY.md, USER.md, MEMORY.md,
  and daily notes) using full-text and semantic vector search. Returns the most
  relevant chunks with their source file paths and relevance scores.
metadata:
  tier: builtin
  params: '{"type":"object","properties":{"query":{"type":"string","description":"Natural language search query"},"limit":{"type":"integer","description":"Max results to return (default 5, max 20)"}},"required":["query"]}'
---
