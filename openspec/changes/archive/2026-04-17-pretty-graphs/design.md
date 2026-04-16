## Architecture

### Package Swap

Replace `flutter_markdown_plus: ^1.0.0` with `flutter_smooth_markdown: ^0.7.2`.

This brings in transitively: `flutter_svg`, `flutter_highlight`, `flutter_math_fork`, `cached_network_image`. Removes the direct `flutter_markdown_plus` dependency.

### Widget Mapping

```
BEFORE                              AFTER
──────                              ─────
MarkdownBody(                       SmoothMarkdown(
  data: message.content,              data: message.content,
  styleSheet: ...,                    styleSheet: ...,
  selectable: true,                   selectable: true,
)                                     useEnhancedComponents: true,
                                      plugins: ParserPluginRegistry([
                                        MermaidPlugin(),
                                      ]),
                                    )
```

For streaming messages, use `StreamMarkdown` instead:

```
StreamMarkdown(
  stream: tokenStream,          // Stream<String> of chunks
  styleSheet: ...,
  plugins: ParserPluginRegistry([
    MermaidPlugin(),
  ]),
)
```

### Streaming Architecture

Currently, `chat_provider.dart` accumulates tokens into `ChatMessage.content` via string concatenation, and the UI rebuilds `MarkdownBody` on every state change.

The new approach:

```
┌──────────────────────────────────────────────────────────┐
│  chat_provider.dart                                      │
│                                                          │
│  _streamMessage() {                                      │
│    final controller = StreamController<String>();         │
│    // expose controller.stream on ChatMessage             │
│                                                          │
│    for await (event in sseStream) {                       │
│      if (event is ContentDelta) {                         │
│        controller.add(event.text);  // raw chunks         │
│        accumulated += event.text;   // keep for history   │
│      }                                                   │
│    }                                                     │
│    controller.close();                                   │
│    // finalize: message.content = accumulated             │
│  }                                                       │
└──────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────┐
│  chat_screen.dart                                        │
│                                                          │
│  if (message.isStreaming && message.tokenStream != null)  │
│    StreamMarkdown(stream: message.tokenStream!, ...)      │
│  else                                                    │
│    SmoothMarkdown(data: message.content, ...)             │
└──────────────────────────────────────────────────────────┘
```

`StreamMarkdown` internally accumulates chunks via `StringBuffer` with 50ms throttled rebuilds. After streaming completes, the widget is replaced by `SmoothMarkdown` with the final accumulated string (which enables caching and selection).

### Theme Mapping

`flutter_smooth_markdown`'s `MarkdownStyleSheet` is a different class with different property names:

```
flutter_markdown_plus          flutter_smooth_markdown
─────────────────────          ───────────────────────
MarkdownStyleSheet             MarkdownStyleSheet
  .fromTheme(theme)              .fromTheme(theme)
  .copyWith(                     .copyWith(
    p: TextStyle(...)              paragraphStyle: TextStyle(...)
    code: TextStyle(...)           inlineCodeStyle: TextStyle(...)
  )                              )
```

Use `MarkdownStyleSheet.fromTheme(Theme.of(context))` as the base (auto light/dark), then `.copyWith()` to match our current color overrides.

### Mermaid Rendering Pipeline (from flutter_smooth_markdown)

````
```mermaid code block
    │
    ▼
MermaidPlugin.parse()          ← parser plugin, priority 10
    │                            intercepts before CodeBlockParser
    ▼
MermaidDiagramNode { code, theme }
    │
    ▼
MermaidBuilder                 ← registered in BuilderRegistry
    │
    ├── Dart mermaid parser (flowchart_parser, sequence_parser, etc.)
    ├── Model objects (nodes, edges, slices, bars)
    ├── Layout algorithm (position computation)
    ├── CustomPainter (Canvas drawing)
    └── InteractiveViewer (pan + zoom)
````

Theme cascade: node-level `theme=dark` > builder default > stylesheet background luminance > light.

### SVG Rendering

SVG in code blocks is handled by `flutter_svg` (bundled dependency). The code block builder detects `language == 'svg'` and renders via `SvgPicture.string()`. No additional wiring needed beyond the package being present.

### Plugin Configuration

Only enable `MermaidPlugin` initially. The package also offers `ThinkingPlugin`, `ToolCallPlugin`, and `ArtifactPlugin` — evaluate these post-launch since we already have custom `ToolCallChip` rendering.

## Decisions

| Decision             | Choice                                                     | Rationale                                                                      |
| -------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Mermaid renderer     | Native Flutter (CustomPainter) via flutter_smooth_markdown | No WebView overhead, no JS bundle, works on all platforms, theme-aware         |
| Markdown package     | flutter_smooth_markdown                                    | Mermaid + SVG + streaming + syntax highlighting + LaTeX in one package         |
| Streaming widget     | StreamMarkdown with Stream<String>                         | Built-in 50ms throttle, purpose-built for token streams                        |
| Non-streaming widget | SmoothMarkdown with accumulated string                     | Enables parse caching and text selection                                       |
| Enhanced components  | Enabled                                                    | Copy buttons on code blocks and fullscreen diagrams improve UX                 |
| Chart rendering      | Deferred (Phase 1.5)                                       | Mermaid's xychart-beta covers basic bar/line; fl_chart for richer charts later |
| ThinkingPlugin       | Deferred                                                   | Evaluate overlap with existing UI patterns                                     |
| ToolCallPlugin       | Deferred                                                   | We already render tool calls via ToolCallChip                                  |

## File Changes

| File                                       | Change                                                                                       |
| ------------------------------------------ | -------------------------------------------------------------------------------------------- |
| `app/pubspec.yaml`                         | Remove `flutter_markdown_plus`, add `flutter_smooth_markdown: ^0.7.2`                        |
| `app/lib/features/chat/chat_screen.dart`   | Replace `MarkdownBody` with `SmoothMarkdown`/`StreamMarkdown`, configure plugins + theme     |
| `app/lib/features/chat/chat_provider.dart` | Add `StreamController<String>` for token streaming, expose `Stream<String>` on `ChatMessage` |
| `app/lib/features/chat/chat_provider.dart` | Add `tokenStream` field to `ChatMessage`                                                     |
