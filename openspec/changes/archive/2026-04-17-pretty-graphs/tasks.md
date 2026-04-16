## Tasks

### 1. Swap markdown dependency

- [x] Remove `flutter_markdown_plus: ^1.0.0` from `app/pubspec.yaml`
- [x] Add `flutter_smooth_markdown: ^0.7.2` to `app/pubspec.yaml`
- [x] Run `flutter pub get` and verify resolution
- [x] Remove `flutter_markdown_plus` import from `chat_screen.dart`

### 2. Replace MarkdownBody with SmoothMarkdown for non-streaming messages

- [x] Import `flutter_smooth_markdown` in `chat_screen.dart`
- [x] Replace `MarkdownBody(...)` with `SmoothMarkdown(...)` in `_MessageBubble`
- [x] Map theme: `MarkdownStyleSheet.fromTheme(Theme.of(context)).copyWith(paragraphStyle: ..., inlineCodeStyle: ...)`
- [x] Set `selectable: true`
- [x] Set `useEnhancedComponents: true`
- [x] Configure `plugins: ParserPluginRegistry` with `MermaidPlugin` + `MermaidBuilder`
- [x] Verify existing chat messages render correctly (flutter analyze clean, 305 tests pass)

### 3. Expose token stream from chat provider

- [x] Add `Stream<String>? tokenStream` field to `ChatMessage`
- [x] In `_streamMessage()`, create a `StreamController<String>.broadcast()`
- [x] On each `TokenEvent`, `controller.add(event.token)` alongside existing accumulation
- [x] Close the controller when streaming ends (DoneEvent, ErrorEvent, stream-ended, catch)
- [x] Set `tokenStream` on the `ChatMessage` during streaming, clear on finalization
- [x] Also wired up `_streamVoiceMessage()` with same pattern
- [x] Write test: verify streaming message has tokenStream, finalized message does not

### 4. Use StreamMarkdown for streaming messages

- [x] In `_MessageBubble`, check `message.isStreaming && message.tokenStream != null`
- [x] Render `StreamMarkdown(stream: message.tokenStream!, ...)` for active streams
- [x] Apply same styleSheet and plugins as SmoothMarkdown
- [x] Fall back to `SmoothMarkdown(data: message.content)` when streaming completes
- [ ] Verify streaming renders token-by-token without flicker (manual test)

### 5. Test mermaid rendering

- [ ] Send a message that triggers a mermaid flowchart response, verify diagram renders
- [ ] Test sequence diagram rendering
- [ ] Test pie chart rendering
- [ ] Verify pan/zoom works on diagrams (InteractiveViewer)
- [ ] Verify dark/light theme auto-detection works
- [ ] Verify mermaid renders correctly after streaming completes (SmoothMarkdown path)

### 6. Test SVG rendering

- [ ] Verify ```svg code blocks render as inline SVG via flutter_svg
- [ ] Test with a simple SVG shape
- [ ] Verify fallback to code block on malformed SVG

### 7. Verify no regressions

- [x] Run `flutter analyze` — zero issues
- [x] Run `flutter test` — all 306 tests pass (305 existing + 1 new)
- [ ] Manual test: regular markdown (headers, lists, bold, italic, links, images)
- [ ] Manual test: inline code and fenced code blocks with syntax highlighting
- [ ] Manual test: long messages, empty messages, messages with only code blocks
- [ ] Manual test: message retry preserves content correctly
- [ ] Manual test: web and macOS targets both work
