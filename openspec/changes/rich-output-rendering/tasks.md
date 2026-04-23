# Rich Output Rendering — Tasks

- [x] Add `output_capabilities()` to system prompt composition — add `fn output_capabilities(interface: &Interface) -> &'static str` in `crates/runtime/src/orchestrator/prompt.rs` that returns a hardcoded capability block per Interface variant; change `compose_system_prompt()` to accept `&Interface` and append capabilities after the skills XML
- [x] Thread `Interface` into `compose_system_prompt()` call sites — update `run_turn_core()` (mod.rs:913), `run_turn_with_tools_impl()` (mod.rs:539), and `subagent.rs:149` to pass `&interface`; verify with `cargo check -p assistant-runtime`
- [x] Add `flutter_svg` dependency — add `flutter_svg: ^2.0.17` to `app/pubspec.yaml` and run `flutter pub get`
- [x] Implement `SvgBuilder` and SVG sanitization — create `app/lib/features/chat/svg_builder.dart` with `SvgPlugin` (block parser for ```svg fenced blocks, following MermaidPlugin pattern) and `SvgBuilder`(sanitize SVG, render via`SvgPicture.string()`); sanitize: strip `<script>`, `on\*`attributes,`<foreignObject>`, `javascript:` URIs; fallback to code block on parse error
- [x] Register SVG plugin in chat screen — in `app/lib/features/chat/chat_screen.dart`, register `SvgPlugin()` in both `ParserPluginRegistry` instances and `SvgBuilder()` in both `BuilderRegistry` instances alongside Mermaid
- [x] Flutter widget tests for SVG rendering — create `app/test/widget/svg_rendering_test.dart`: test valid SVG renders as `SvgPicture`, malformed SVG falls back to code block, `<script>` and `onclick` are stripped by sanitizer
- [x] Rust tests for output capabilities — add `#[cfg(test)]` module in `prompt.rs`: `Web` contains "svg"+"mermaid", `Signal` omits "svg", `Slack` mentions "mrkdwn", every variant produces non-empty text
- [x] Run lints and format — `make lint`, `make format`, `make lint-flutter`; fix any issues
