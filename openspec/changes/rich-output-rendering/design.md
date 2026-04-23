# Rich Output Rendering — Design

## Architecture

Two independent subsystems, connected by convention (the ` ```svg ` fenced
block language tag):

```
┌──────────────────────────────────────────────────────────────────┐
│  RUST (system prompt)                                            │
│                                                                  │
│  compose_system_prompt(interface)                                │
│      │                                                           │
│      ├── memory_loader.load_system_prompt()   (model-owned)      │
│      ├── available_skills_xml()               (code-generated)   │
│      └── output_capabilities(interface) ← NEW (code-controlled)  │
│                                                                  │
│  Interface::Web  → svg, mermaid, markdown, images                │
│  Interface::Cli  → markdown, mermaid (if terminal supports)      │
│  Interface::Slack → mrkdwn, emoji, Block Kit                     │
│  Interface::Signal → plain text only                             │
│  Interface::Matrix → HTML subset, images                         │
│  ...                                                             │
└──────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────┐
│  FLUTTER (rendering)                                             │
│                                                                  │
│  StreamMarkdown / SmoothMarkdown                                 │
│      │                                                           │
│      ├── ParserPluginRegistry                                    │
│      │   ├── MermaidPlugin()        (exists)                     │
│      │   └── SvgPlugin()          ← NEW                         │
│      │                                                           │
│      └── BuilderRegistry                                         │
│          ├── 'mermaid' → MermaidBuilder()  (exists)              │
│          └── 'svg'     → SvgBuilder()    ← NEW                  │
│                                                                  │
│  SvgBuilder renders via flutter_svg SvgPicture.string()          │
└──────────────────────────────────────────────────────────────────┘
```

## Part 1: Output Capabilities in System Prompt

### Where

`crates/runtime/src/orchestrator/prompt.rs` — the `compose_system_prompt()` method.

### Change

`compose_system_prompt()` currently takes no arguments. It needs the `Interface`
to select the right capabilities block. The signature becomes:

```rust
pub(crate) async fn compose_system_prompt(&self, interface: &Interface) -> String
```

A new private method generates the capabilities text:

```rust
fn output_capabilities(interface: &Interface) -> String
```

### Call sites

Three places call `compose_system_prompt()`:

1. `run_turn_core()` at `mod.rs:913` — has `interface` in scope
2. `run_turn_with_tools_impl()` at `mod.rs:539` — has `interface` in scope
3. `subagent.rs:149` — subagent prompt composition fallback

All three already have `Interface` available (subagents inherit the parent's
execution context), so threading it through is trivial.

### Capability text per interface

The capability block is a hardcoded `&str` per interface variant, appended after
the skills XML. It's **not** a memory file — it lives in compiled Rust code.

**Web / Desktop:**

````markdown
## Output capabilities

Your responses render in a rich client. These fenced code block languages produce
visual output instead of code blocks:

- ```mermaid — rendered as a diagram (flowchart, sequence, state, ER, Gantt, etc.)

  ```

- ```svg — rendered as an inline SVG graphic

  ```

Use Mermaid for structured diagrams. Use SVG for custom visuals, charts,
illustrations, or layouts that Mermaid cannot express. Use visual output when it
genuinely aids understanding — not every response needs a diagram.
````

**CLI:**

````markdown
## Output capabilities

Your responses render in a terminal. Markdown formatting (bold, italic, lists,
code blocks) is supported. Fenced ```mermaid blocks are rendered as diagrams
when the terminal supports it. Prefer text-based explanations.
````

**Signal:**

```markdown
## Output capabilities

Your responses render as plain text in a mobile messaging app. No markdown,
no images, no diagrams. Keep responses concise and text-only.
```

**Slack:**

```markdown
## Output capabilities

Your responses render in Slack. Use mrkdwn formatting (bold, italic, code blocks,
lists, links). Do not use `svg or `mermaid — they will appear as raw code.
Use emoji where appropriate. Reply via the provided reply tools.
```

**Matrix:**

```markdown
## Output capabilities

Your responses render in a Matrix client. Basic HTML formatting is supported
(bold, italic, code blocks, lists, links). Images can be sent as attachments.
Do not use `svg or `mermaid — they will appear as raw code.
```

The remaining interfaces (Mattermost, Nextcloud, Scheduler, Mcp) follow similar
patterns — declare what works, explicitly note what doesn't.

### Why hardcoded strings, not a config file

- **The model cannot edit compiled Rust.** Memory files (SOUL.md, etc.) are the
  model's scratchpad — anything placed there can be overwritten in the next session.
- **Capabilities are a function of the code.** If the Flutter app can render SVG,
  that's a build-time fact. A config file would create drift.
- **It's small.** Each interface variant is ~4 lines of text. A config system would
  be over-engineering.

## Part 2: SVG Rendering in Flutter

### Dependency

Add `flutter_svg` to `app/pubspec.yaml`:

```yaml
dependencies:
  flutter_svg: ^2.0.17
```

### Plugin + Builder

Following the established Mermaid pattern, create two classes:

**`SvgPlugin`** — a `ParserPluginRegistry` block plugin that recognizes ` ```svg `
fenced code blocks (analogous to `MermaidPlugin`). This comes from
`flutter_smooth_markdown`'s plugin system.

**`SvgBuilder`** — a `BuilderRegistry` entry that receives the raw SVG string and
returns a widget:

```dart
class SvgBuilder extends MarkdownElementBuilder {
  @override
  Widget? buildWidget(String content, Map<String, String> attributes) {
    final sanitized = _sanitizeSvg(content);
    return SvgPicture.string(
      sanitized,
      fit: BoxFit.contain,
      width: double.infinity,
    );
  }
}
```

Whether `SvgPlugin` + `SvgBuilder` are custom classes we write, or whether
`flutter_smooth_markdown` already exposes a generic "custom code block" hook that
just needs registration, depends on the library's API. The Mermaid integration
suggests the library supports this pattern — we follow it exactly.

### Registration

In `chat_screen.dart`, two locations (streaming and static markdown) register the
Mermaid plugin. SVG registration is added alongside:

```dart
plugins: ParserPluginRegistry()
  ..registerBlock(MermaidPlugin())
  ..registerBlock(SvgPlugin()),       // NEW
builderRegistry: BuilderRegistry()
  ..register('mermaid', const MermaidBuilder())
  ..register('svg', const SvgBuilder()),  // NEW
```

### SVG sanitization

Strip before rendering:

- `<script>` elements
- `on*` event handler attributes (`onclick`, `onload`, etc.)
- `<foreignObject>` elements (can embed arbitrary HTML)
- `javascript:` URIs in `href` / `xlink:href`

On native (macOS/iOS), `flutter_svg` doesn't execute JS regardless, so this is
defense-in-depth. On the **web target**, sanitization is essential because
`flutter_svg` may fall back to HTML rendering.

### Streaming behavior

SVG (like Mermaid) cannot render incrementally — a half-written `<svg>` tag is
invalid XML. The approach:

1. While inside a ` ```svg ` fenced block, buffer content
2. Show a subtle placeholder (e.g., a small loading indicator or a muted
   "Generating graphic..." label — same treatment as Mermaid)
3. On block close (closing ` ``` `), hand the full SVG to `SvgBuilder`
4. If the SVG is malformed, fall back to showing it as a code block

This is likely already handled by `flutter_smooth_markdown`'s fenced block
buffering — Mermaid has the same constraint and works today.

### Error fallback

If `SvgPicture.string()` fails to parse, catch the error and fall back to
rendering the raw SVG as a syntax-highlighted code block. The user sees the
source (which is what happens today anyway) rather than a broken widget.

## File changes summary

| File                                        | Change                                                                                  |
| ------------------------------------------- | --------------------------------------------------------------------------------------- |
| `crates/runtime/src/orchestrator/prompt.rs` | Add `output_capabilities(interface)`, thread `Interface` into `compose_system_prompt()` |
| `crates/runtime/src/orchestrator/mod.rs`    | Pass `interface` to `compose_system_prompt()` at both call sites                        |
| `app/pubspec.yaml`                          | Add `flutter_svg` dependency                                                            |
| `app/lib/features/chat/svg_builder.dart`    | New: `SvgPlugin` + `SvgBuilder` + sanitization                                          |
| `app/lib/features/chat/chat_screen.dart`    | Register `SvgPlugin` and `SvgBuilder` alongside Mermaid                                 |

## Testing

### Rust

- Unit test for `output_capabilities()`: each `Interface` variant produces
  non-empty text, Web variant mentions `svg` and `mermaid`, Signal variant
  does not mention `svg`
- Existing `compose_system_prompt` tests (if any) updated to pass `Interface`

### Flutter

- Widget test: render a `SmoothMarkdown` with a ` ```svg ` block containing
  a simple `<svg>` element, verify an `SvgPicture` widget appears in the tree
- Widget test: malformed SVG falls back to code block
- Widget test: SVG with `<script>` tag — verify script is stripped (check
  rendered SVG string does not contain `<script>`)
