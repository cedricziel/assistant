# Rich Output Rendering

## Problem

The model can already produce SVG, Mermaid diagrams, and other structured visual
formats — but the rendering pipeline mostly treats them as monospace code blocks.
Mermaid works by accident (a plugin was added), not by design. SVG is displayed as
raw XML. There is no systematic approach to rich output, and critically, the model
has no awareness of what the connected client can actually render.

## Proposal

Two changes, shipped together:

### 1. Code-controlled output capabilities in the system prompt

Add a hardcoded section to `compose_system_prompt()` in
`crates/runtime/src/orchestrator/prompt.rs` that declares what the connected
client can render. This section is **compiled into the binary** — the model cannot
edit or delete it (unlike SOUL.md, AGENTS.md, and the other memory files which the
model owns and freely rewrites).

The capabilities block should be **interface-aware**: a web UI client can render
SVG and Mermaid; a Signal interface can only render plain text. The orchestrator
already knows which interface is active.

Example output in the system prompt:

````
## Output capabilities

Your responses are rendered in a rich client that supports:
- Markdown (headings, bold, italic, lists, links, blockquotes, tables)
- Fenced code blocks with syntax highlighting
- ```mermaid — renders as a visual diagram (flowchart, sequence, state, ER, etc.)
- ```svg — renders as an inline SVG graphic

Use visual formats when they genuinely aid understanding. Prefer Mermaid for
structured diagrams (flows, sequences, ERs). Use SVG for custom layouts, charts,
illustrations, or anything Mermaid can't express.
````

For text-only interfaces (CLI, Signal), the block would omit SVG/Mermaid or note
that only plain text is supported.

### 2. SVG fenced code block rendering in the Flutter app

Extend the markdown renderer to recognize ` ```svg ` fenced blocks and render
them as actual SVG graphics using `flutter_svg`, following the same pattern already
established by the Mermaid plugin.

- Buffer the full SVG content (partial SVG is invalid XML — same as Mermaid)
- Render via `SvgPicture.string()` from the `flutter_svg` package
- Show a placeholder/skeleton while the block is still streaming
- Sanitize: strip `<script>`, `on*` event handlers, `<foreignObject>` (flutter_svg
  on native doesn't execute JS, but the web target needs explicit stripping)
- Respect dark/light theme for default fill colors where possible

## Non-goals

- Interactive SVG (click handlers, JS execution, animations) — static rendering only
- Chart DSL or custom widget specs — the model writes raw SVG, not a chart schema
- Model fine-tuning or SVG quality improvement — that's a model concern, not ours
- Skill for visual output guidance — useful later, but out of scope for this change

## Risks

| Risk                            | Mitigation                                                          |
| ------------------------------- | ------------------------------------------------------------------- |
| Model produces broken SVG       | Fallback: show raw code block (same as today)                       |
| XSS via SVG on web target       | Sanitize `<script>`, `on*`, `<foreignObject>` before render         |
| Token cost of SVG is high       | Capability prompt says "use when it genuinely helps" — not "always" |
| Model edits out capability text | Can't — it's hardcoded in Rust, not in a memory file                |

## Success criteria

- Model reliably uses ` ```svg ` blocks when visual output would help, on
  interfaces that support it
- SVG blocks render as graphics in the Flutter web/desktop app
- Model does **not** produce SVG when connected via text-only interfaces
- Mermaid continues to work unchanged
