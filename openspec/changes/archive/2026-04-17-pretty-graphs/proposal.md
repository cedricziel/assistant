## Why

The assistant produces structured visual content — flowcharts, sequence diagrams, pie charts, architecture sketches — but the Flutter app renders all of it as plain monospace code blocks. Users see raw Mermaid syntax instead of the diagram it describes. This makes the assistant's output harder to understand and less useful than competing AI chat interfaces.

## What Changes

- Replace `flutter_markdown_plus` with `flutter_smooth_markdown` for markdown rendering in chat messages
- Enable the Mermaid parser plugin so ` ```mermaid ` code blocks render as native Flutter diagrams (flowcharts, sequence, pie, gantt, kanban, timeline, radar, xy charts) via CustomPainter — no WebView, no JS, no external service
- Enable SVG rendering via the bundled `flutter_svg` dependency for ` ```svg ` code blocks
- Switch streaming messages from rebuilding `SmoothMarkdown` on each token to using `StreamMarkdown` with a `Stream<String>`, gaining built-in 50ms throttle and optimised re-rendering
- Enable enhanced components (copy buttons on code blocks, fullscreen diagrams) for richer interaction
- Syntax highlighting for all code blocks comes free with the switch

## Capabilities

### New Capabilities

- `mermaid-diagram-rendering`: Native rendering of Mermaid diagrams in chat bubbles with pan/zoom, theme-aware styling, and optional fullscreen view
- `svg-inline-rendering`: Inline SVG rendering in chat messages
- `stream-markdown`: Token-stream-aware markdown rendering with throttled rebuilds
- `syntax-highlighting`: Language-aware code block highlighting

### Modified Capabilities

- `chat-message-display`: Message bubbles gain rich content rendering; the markdown widget changes from `MarkdownBody` to `SmoothMarkdown`/`StreamMarkdown`
- `chat-streaming`: Streaming messages switch from accumulated-string rebuilds to `Stream<String>`-based rendering

## Impact

- `app/pubspec.yaml` — swap `flutter_markdown_plus` for `flutter_smooth_markdown`, add `fl_chart` (optional, for custom chart JSON blocks later)
- `app/lib/features/chat/chat_screen.dart` — replace `MarkdownBody` with `SmoothMarkdown`/`StreamMarkdown`, configure plugins and theme
- `app/lib/features/chat/chat_provider.dart` — expose a `Stream<String>` for streaming message tokens alongside the current accumulated string
- Theme mapping: `MarkdownStyleSheet.fromTheme()` API differs between packages; need to remap our `copyWith` overrides
- No backend/Rust changes required — the LLM already produces Mermaid naturally
- No breaking changes to stored messages or API

## Risks

- `flutter_smooth_markdown` is v0.7.2 (pre-1.0, single author) — API may shift. Mitigation: pin version, the widget surface we use is small.
- Native Mermaid parser covers flowcharts, sequences, pie, gantt, kanban, timeline, radar, xy charts. Does NOT cover class diagrams, state diagrams, ER, mindmap, git graph. LLMs most commonly produce flowcharts and sequences, so coverage is adequate.
- Rendering fidelity won't match mermaid.js pixel-for-pixel. Acceptable for an AI assistant where diagrams are explanatory.

## Future (out of scope)

- Server-side rendering (Kroki/resvg/plotters) for thin clients (Slack, Matrix, Signal, CLI) — Phase 2
- Custom ` ```chart ` JSON spec with `fl_chart` rendering — Phase 1.5
- Thinking block rendering via ThinkingPlugin — evaluate after initial rollout
- Tool call rendering via ToolCallPlugin — we already have `ToolCallChip`, evaluate overlap
