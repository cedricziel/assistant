# Rich Output Rendering — Tasks

## Task 1: Add `output_capabilities()` to system prompt composition

**Crate:** `assistant-runtime`
**Files:** `crates/runtime/src/orchestrator/prompt.rs`

- Add `fn output_capabilities(interface: &Interface) -> &'static str` that returns
  a hardcoded capability block per `Interface` variant
- Change `compose_system_prompt()` signature to accept `&Interface`
- Append the output of `output_capabilities()` after the skills XML section
- Write unit tests: each interface variant returns non-empty text, `Web` mentions
  `svg` and `mermaid`, `Signal` does not mention `svg` or `mermaid`

**Depends on:** nothing

---

## Task 2: Thread `Interface` into `compose_system_prompt()` call sites

**Crate:** `assistant-runtime`
**Files:** `crates/runtime/src/orchestrator/mod.rs`, `crates/runtime/src/orchestrator/subagent.rs`

- Update `run_turn_core()` (~line 913) to pass `&interface` to `compose_system_prompt()`
- Update `run_turn_with_tools_impl()` (~line 539) to pass `&interface` to
  `compose_system_prompt()`
- Update `subagent.rs` (~line 149) to pass `&interface` to `compose_system_prompt()`
- Verify compilation with `cargo check -p assistant-runtime`

**Depends on:** Task 1

---

## Task 3: Add `flutter_svg` dependency

**Files:** `app/pubspec.yaml`

- Add `flutter_svg: ^2.0.17` to dependencies
- Run `flutter pub get` to verify resolution

**Depends on:** nothing

---

## Task 4: Implement `SvgBuilder` and SVG sanitization

**Files:** `app/lib/features/chat/svg_builder.dart` (new)

- Create `SvgPlugin` class (block parser plugin for ` ```svg ` fenced blocks),
  following the `MermaidPlugin` pattern from `flutter_smooth_markdown`
- Create `SvgBuilder` class that takes the raw SVG string, sanitizes it, and
  returns an `SvgPicture.string()` widget
- Implement `_sanitizeSvg()`: strip `<script>` elements, `on*` attributes,
  `<foreignObject>` elements, and `javascript:` URIs
- Add error handling: if SVG parsing fails, return `null` so the markdown renderer
  falls back to a code block

**Depends on:** Task 3

---

## Task 5: Register SVG plugin in chat screen

**Files:** `app/lib/features/chat/chat_screen.dart`

- Import `svg_builder.dart`
- Register `SvgPlugin()` in both `ParserPluginRegistry` instances (streaming and
  static markdown, ~lines 758-761 and 781-784)
- Register `SvgBuilder()` in both `BuilderRegistry` instances alongside `MermaidBuilder`

**Depends on:** Task 4

---

## Task 6: Flutter widget tests for SVG rendering

**Files:** `app/test/widget/svg_rendering_test.dart` (new)

- Test: `SmoothMarkdown` with a valid ` ```svg ` block renders an `SvgPicture`
- Test: malformed SVG falls back to code block (no `SvgPicture` in widget tree)
- Test: SVG containing `<script>` tag — sanitizer strips it before rendering
- Test: SVG containing `onclick` attribute — sanitizer strips it

**Depends on:** Task 5

---

## Task 7: Rust tests for output capabilities

**Files:** `crates/runtime/src/orchestrator/prompt.rs` (add `#[cfg(test)]` module)

- Test: `output_capabilities(Interface::Web)` contains "svg" and "mermaid"
- Test: `output_capabilities(Interface::Signal)` does not contain "svg"
- Test: `output_capabilities(Interface::Slack)` mentions "mrkdwn"
- Test: every `Interface` variant produces a non-empty string (exhaustive match
  prevents forgetting new variants)

**Depends on:** Task 1

---

## Task 8: Run lints and format

- `make lint` and `make format` for Rust
- `make lint-flutter` for Flutter
- Fix any issues

**Depends on:** Tasks 1-7
