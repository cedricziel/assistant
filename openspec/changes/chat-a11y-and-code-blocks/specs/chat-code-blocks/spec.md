## ADDED Requirements

### Requirement: CodeBlock element renders a labelled, copyable code block

`assistant_ui` SHALL export `CodeBlock({required String code, String? language, ValueChanged<String>? onCopy, bool highlight = true})`.

- It SHALL render a header row above the code containing the language label (the fence language as written, or `text` when absent) and, when a copy callback is available, a copy button.
- It SHALL render the code in a horizontally scrollable, selectable monospace body.
- The header SHALL NOT overlap the code.
- The header SHALL be excluded from text selection.
- `CodeBlock` SHALL be registered in `kAssistantUiElements` with a widgetbook use-case and golden tests in light and dark themes.

#### Scenario: Language label is shown

- **WHEN** a `CodeBlock` renders with `language: 'rust'`
- **THEN** the header SHALL show `rust`
- **WHEN** it renders with `language: null`
- **THEN** the header SHALL show `text`

#### Scenario: Header does not overlap code

- **GIVEN** a `CodeBlock` whose first line is long enough to reach the right edge
- **WHEN** it renders
- **THEN** the bounding box of the copy button SHALL NOT intersect the bounding box of the code text

#### Scenario: Selecting the block does not copy the label

- **GIVEN** a `CodeBlock` inside a `SelectionArea`
- **WHEN** the user selects all and copies
- **THEN** the copied text SHALL equal the code AND SHALL NOT contain the language label or the copy button's label

#### Scenario: Long lines scroll instead of overflowing

- **GIVEN** a 300-character single-line code block at 360 px width
- **WHEN** it renders
- **THEN** no overflow error SHALL be reported AND the body SHALL be horizontally scrollable

### Requirement: Copy is a callback with an accessible, always-visible control

`CodeBlock` SHALL resolve its copy callback from `onCopy`, falling back to the nearest `ChatMarkdownScope.onCopyCode`. If neither is set, no copy button SHALL render.

The copy button:

- SHALL be visible whenever it renders, with no hover-only reveal;
- SHALL have the tooltip and semantics label "Copy code";
- SHALL be keyboard focusable and activatable;
- SHALL have a hit area of at least 24×24 logical pixels.

Activating it SHALL invoke the callback with the exact code string, then show a "Copied" state (check icon, label "Copied") for 2 seconds. The package SHALL NOT access the clipboard itself.

#### Scenario: Copy invokes the callback with the code

- **GIVEN** a `CodeBlock(code: 'fn main() {}', onCopy: spy)`
- **WHEN** the copy button is activated
- **THEN** `spy` SHALL be called once with `'fn main() {}'` AND the button's semantics label SHALL become "Copied"
- **WHEN** 2 seconds elapse
- **THEN** the label SHALL return to "Copy code"

#### Scenario: Scope supplies the callback

- **GIVEN** a `ChatMarkdownScope` with `onCopyCode: spy` and a `CodeBlock` without `onCopy`
- **WHEN** the copy button is activated
- **THEN** `spy` SHALL receive the code

#### Scenario: No callback, no button

- **WHEN** a `CodeBlock` renders with no `onCopy` and no scope
- **THEN** no widget with the "Copy code" semantics label SHALL exist

#### Scenario: Copy is reachable by keyboard

- **GIVEN** a `CodeBlock` with a copy callback
- **WHEN** focus is moved onto the copy button with Tab and Enter is pressed
- **THEN** the callback SHALL fire

#### Scenario: Copied feedback respects reduced motion

- **GIVEN** `MediaQuery.disableAnimations` is true
- **WHEN** the copy button is activated
- **THEN** the "Copied" state SHALL appear in the next frame without an opacity or size animation

### Requirement: Syntax highlighting with contrast-checked themes

`CodeBlock` SHALL highlight its code through the package function `highlightCode(code, language)`, backed by the `highlight` 0.7.0 package.

- Highlighting SHALL apply only when `highlight` is true, `language` names a registered grammar, and the code is at most 20,000 characters. Otherwise the code SHALL render as plain monospace.
- There SHALL be no language auto-detection.
- Results SHALL be memoised in a bounded LRU of 64 entries keyed by `(language, code)` with full string equality. When the most recently inserted entry has the same language and its code is a prefix of the new code (a growing streaming fence), the new entry SHALL replace it in place instead of taking a new slot. Plain (unhighlighted) renders SHALL NOT be memoised.
- Token colours SHALL come from `CodeTheme.light` or `CodeTheme.dark`, selected by the ambient theme brightness. Both are defined in the package theme directory and derived from highlight.js `a11y-light` / `a11y-dark`.
- Each `CodeTheme` SHALL own its background colour, and `CodeBlock` SHALL paint the code body with it.
- Every token colour SHALL reach a contrast ratio of at least 4.5:1 against its own theme's background.

#### Scenario: Known language is highlighted

- **WHEN** a `CodeBlock` renders `let x = 1;` with `language: 'rust'`
- **THEN** the body's `TextSpan` tree SHALL contain at least one span whose colour comes from `CodeTheme` keyword styles

#### Scenario: Unknown or absent language renders plain

- **WHEN** a `CodeBlock` renders with `language: 'not-a-language'` or `language: null`
- **THEN** the body SHALL be a single unstyled-colour span AND no exception SHALL be thrown

#### Scenario: Oversized block renders plain

- **WHEN** a `CodeBlock` renders 20,001 characters with `language: 'dart'`
- **THEN** `highlight` SHALL NOT be invoked AND the code SHALL render plain

#### Scenario: Theme follows brightness

- **WHEN** the same `CodeBlock` renders under a dark theme and under a light theme
- **THEN** it SHALL use `CodeTheme.dark` and `CodeTheme.light` respectively

#### Scenario: Theme contrast is verified

- **WHEN** the contrast unit test computes WCAG contrast for every token colour in `CodeTheme.light` and `CodeTheme.dark` against that theme's own `background`
- **THEN** every ratio SHALL be at least 4.5

#### Scenario: Growing fence does not evict finished blocks

- **GIVEN** a finished `dart` fence that has been highlighted once
- **WHEN** a second `rust` fence grows through 100 chunks, each highlighted in turn
- **THEN** the memo SHALL hold at most two entries for these fences AND highlighting the finished fence again SHALL NOT call the highlighter

#### Scenario: Re-render reuses highlighting

- **GIVEN** a block already highlighted once
- **WHEN** the same `(language, code)` renders again
- **THEN** the underlying highlighter SHALL NOT be called a second time

### Requirement: Markdown code fences render through CodeBlock

`MessageBody` and `ThinkingBody` SHALL register a `code_block` builder in the `BuilderRegistry` they pass to `SmoothMarkdown` and `StreamMarkdown`, and that builder SHALL render `CodeBlock`. The builder SHALL override `flutter_smooth_markdown`'s `EnhancedCodeBlockBuilder` while leaving the other enhanced builders (blockquote, link, header) in place. Fences claimed by parser plugins (`mermaid`, `svg`) SHALL keep rendering through their own builders. The app SHALL install `ChatMarkdownScope.onCopyCode`, which writes the code to the system clipboard and announces "Code copied".

#### Scenario: Assistant fence renders as CodeBlock

- **GIVEN** an assistant message whose content is a fenced `python` block
- **WHEN** `MessageBody` renders it, streaming or final
- **THEN** exactly one `CodeBlock` SHALL render with `language: 'python'`

#### Scenario: Mermaid is unaffected

- **GIVEN** content with a `mermaid` fence and a `ChatMarkdownScope` registering the Mermaid plugin and builder
- **WHEN** `MessageBody` renders it
- **THEN** the Mermaid builder SHALL render it AND no `CodeBlock` SHALL render

#### Scenario: App copy writes the clipboard

- **GIVEN** the chat screen with an assistant reply containing a code fence
- **WHEN** the copy button is activated
- **THEN** the mocked clipboard channel SHALL receive `Clipboard.setData` with the code AND an accessibility announcement "Code copied" SHALL be sent

#### Scenario: Streaming highlights only changed blocks

- **GIVEN** a streaming reply with one finished fence followed by a growing fence
- **WHEN** a new chunk arrives
- **THEN** highlighting for the finished fence SHALL be served from the memo
