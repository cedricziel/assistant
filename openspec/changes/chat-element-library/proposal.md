## Why

The chat surface lives in two god files: `chat_screen.dart` (2225 LOC) and `chat_provider.dart` (2306 LOC). This violates the project convention "avoid screen widgets longer than ~300 lines."

The widgets that render an agentic turn are already mostly pure — `streaming_timeline_entry.dart` (669 LOC), `timeline_section.dart` (302 LOC), `tool_call_chip.dart` (177 LOC) and `turn_status_label.dart` (51 LOC) contain **zero** Riverpod references, and `StreamingTimelineEntry` already takes `(message, density, entryState)` as plain constructor params. What they lack is naming, a package boundary, and a gallery. Today the only way to see a thinking indicator in its `stale × compact` state is to get lucky with a live stream.

[assistant-ui Elements](https://www.assistant-ui.com/elements) defines the conventional vocabulary for how an assistant conversation looks — shell, thinking, streaming, display. Adopting that vocabulary as a Flutter package with a live gallery turns a folder into a design system and makes every element golden-testable without a server, a provider stub, or a live stream.

## What Changes

- New package `app/packages/assistant_ui/`, depending on `flutter` only — **not** `flutter_riverpod`, **not** `assistant_api`. The compiler enforces element purity; `TurnProgressCard`'s 3 `ref.watch` calls are the existing leak this closes.
- Split `StreamingTimelineEntry` into `TimelineEntryShell` (density, expand/pin, auto-collapse, stale) plus stateless bodies (`MessageBody`, `ThinkingBody`, `ToolCallBody`, `SubagentBody`, `CommandBody`). `ChatTimelineSection` duplicates that same state machine today and collapses into the shell.
- Move the render-facing models (`ChatMessage`, `ChatAttachment`, `ToolCallRecord`, `MessageStatus`, `ToolCallStatus`, `TimelineEntryType`) into the package. They are already pure data classes.
- `TimelineDensity` becomes an inherited `ThreadDensityScope` instead of a param threaded through every level.
- New elements: `ReasoningPanel` — a collapsible timeline of reasoning steps with elapsed time, replacing today's flat thinking blob; `ThreadViewport` — scroll anchoring with a "jump to latest" recovery pill for when the user has scrolled away mid-stream.
- The composer moves too: `_InputRow` is already the cleanest element in the codebase — 13 constructor parameters, every interaction a callback, zero `ref` reads — and becomes `Composer`, with `SlashCommandMenu`, `AttachmentTray` and `ComposerVoiceButton` alongside it.
- New `app/widgetbook/` app: every element × every state (`active`/`complete`/`stale` × `compact`/`normal`/`expanded`), runnable via `flutter run -d chrome`.
- `ChatScreen` keeps every `ref.watch` and becomes a thin adapter over the package.
- `ELEMENTS.md` in the package: a standing index of all 59 assistant-ui elements with an explicit Adopt / Defer / Decline verdict and reason for each. Drafted in this change as `elements.md`, moved into the package on PR 1, and maintained from then on — adding an element without adding its row is a review failure.

## Capabilities

### New Capabilities

- `chat-element-library`: the packaged, state-management-free element vocabulary for rendering an assistant conversation — shell, thinking, streaming, and display — plus the gallery that exercises it.

## Impact

- **Code touched**: new `app/packages/assistant_ui/` and `app/widgetbook/`; `app/lib/features/chat/` shrinks substantially; chat test imports move.
- **Visual change**: intentional and near-zero for existing elements, but `ReasoningPanel` and the scroll-anchor pill are new UI. **Playwright screenshot baselines will move** and must be updated in this change.
- **Non-goals** (full reasoning per element in `elements.md`):
  - Mentions and the context/token ring. Neither has a data source: `grep -rn "mention"` over `app/lib` and `crates/web-ui` returns nothing, and token counts exist only as analytics aggregates, never per-conversation in the stream.
  - Moving the model/persona picker into the composer — that is a UX change, not an extraction.
  - I/O inside the package. `record`, `file_picker` and `desktop_drop` stay in the app; the package renders the affordances and emits callbacks.
  - Tool-derived renderers (terminal block, code diff, sources). `ToolOutput.data` is discarded at `OrchestratorEvent::ToolResult` — separate change.
  - Approval cards and any client→server return channel. `crates/web-ui/src/lib.rs:530` hardwires `AutoDenyConfirmation` — separate defect, unblocked by this work.
  - Converting `ChatMessage` to a sealed hierarchy (see design.md).
  - Generative UI / block vocabulary.
- **User-facing documentation needed**: No. Add `app/packages/assistant_ui/README.md` for contributors.
