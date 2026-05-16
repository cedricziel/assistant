## Context

Tool calls flow through these layers:

1. **Runtime** (`crates/runtime/src/otel_spans.rs::start_tool_span`) — creates an OTel span named `execute_tool <tool_name>` with attributes:
   - `tool_name`, `tool_params` (JSON string), `iteration`, `turn`, `interface`, `conversation_id`, `gen_ai.conversation.id`, optional `active_skill`.
2. **Orchestrator** (`crates/runtime/src/orchestrator/dispatch.rs`) — closes the span with:
   - `duration_ms`, `tool_status` in {`ok`, `error`, `denied`}, plus `tool_observation` on success or `tool_error` on failure / denial.
3. **Exporter** (`crates/opentelemetry-exporter-sqlite`) — persists spans to SQLite.
4. **Web-ui API** — returns `TraceDetailResponse.spans` with each `OtelSpanRecord { name, durationMs, attributes }`.
5. **Flutter trace detail screen** (`app/lib/features/traces/trace_detail_screen.dart`) — sorts spans by `startTime`, renders each through `_SpanCard` with name, duration bar, and an expandable alphabetical attribute dump.

The data is all there. The web UI just renders it generically. This change is **rendering-only on the Flutter side**.

## Goals / Non-Goals

**Goals**

- A tool span is immediately recognisable in the trace list — distinct card style, status icon and color.
- The two most-asked questions while debugging — _"what did the tool receive?"_ and _"what did it return?"_ — are answerable without expanding a long attribute dump.
- Status colours match the chat `tool-call-display` chip palette so users build one mental model.

**Non-goals**

- Span-tree visualisation across non-tool spans (separate change).
- Live streaming trace updates (out of scope).

## Decisions

### D1: Detection — `span.name.startsWith('execute_tool ')`

**Choice:** Treat a span as a tool span when its name starts with the literal `execute_tool ` prefix. Fall back to `attributes['tool_name'] != null` if the prefix is missing (defensive — in case future runtime code drops the prefix).

**Why:** The prefix is set in exactly one place (`otel_spans.rs:181`) and is stable. Attribute-based detection alone would mis-categorise unrelated spans that happen to mention a tool name.

### D2: Status palette mirrors chat tool-call chips

**Choice:** Use the same colours as the chat `tool-call-display` spec:

- `ok` → checkmark icon, `colorScheme.tertiary` (green-ish in seed-based scheme).
- `error` → cross icon, `colorScheme.error`.
- `denied` → prohibition icon, amber (`Color(0xFFB45309)` or the closest theme token).

**Why:** Users debugging a turn often look at the chat _and_ the trace. Identical chip semantics across both views makes the link obvious without explicit cross-references.

### D3: Two-pane body — `Params` / `Output`

**Choice:** Each `_ToolCallSpanCard`, when expanded, renders a two-pane scrollable body:

```
┌──────────────────────────────┐ ┌──────────────────────────────┐
│ Params                       │ │ Output                       │
│ {                            │ │ "Wrote 3 files to /tmp/..."  │
│   "path": "/tmp/foo.txt"     │ │                              │
│ }                            │ │                              │
└──────────────────────────────┘ └──────────────────────────────┘
```

`tool_params` is pretty-printed via `JsonEncoder.withIndent('  ')`. If parsing fails the raw string is shown verbatim. On `error` or `denied`, the Output pane shows `tool_error` styled with `colorScheme.error` text.

**Why:** Aligns with the existing log/JSON viewer aesthetic and keeps everything on one row at desktop widths. On narrow widths (< 600 dp), the two panes stack vertically — handled with `LayoutBuilder`.

### D4: Other attributes stay accessible via a "show all" toggle

**Choice:** The remaining attributes (`iteration`, `turn`, `interface`, `active_skill`, etc.) live behind a small `Show all attributes` toggle in the footer, so the card stays focused on the diagnostically critical fields by default but doesn't hide data.

**Why:** Power users still need iteration / turn info; we just stop making it the front-of-house information.

## Risks / Trade-offs

- **JSON parsing cost:** `tool_params` and `tool_observation` are strings that may be large. Parse lazily on first expand, not on every list render.
- **Theme drift:** If `colorScheme.tertiary` is reused for unrelated purposes, our success-green could clash. We accept this; revisit when the broader theme audit (#266) lands.
- **Missing attributes from older traces:** Traces generated before this PR may not have `tool_status` (rare — code path predates this change but the schema is stable). The detector falls back to "unknown" status (neutral grey) so older traces still render.

## Migration Plan

No data migration. Existing trace records already contain the attributes we display. New rendering only.
