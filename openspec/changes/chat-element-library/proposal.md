## Why

Nine sibling changes (approvals, tool renderers, branching, feedback, suggestions, A2UI surfaces, conversation management, a11y/code blocks, stop-cancels-turn) all add chat UI. Today that UI lives in `app/lib/features/chat/` (`chat_screen.dart` 623 LOC, `streaming_timeline_entry.dart` 671 LOC, `message_bubble.dart` 276 LOC), where render code, Riverpod reads and plugin I/O mix freely. Without a shared element vocabulary and named slots, each sibling grows those files on its own and they collide.

The render widgets are already close to pure. `StreamingTimelineEntry` takes `(message, density, entryState)` by constructor (`streaming_timeline_entry.dart:47-58`), `ChatInputRow` takes everything by parameter (`chat_input_row.dart:13-43`), and the models already live in their own Riverpod-free file (`chat_models.dart`). What is missing is a package boundary, the extension points siblings need, and a gallery. [assistant-ui Elements](https://www.assistant-ui.com/elements) supplies the vocabulary.

## What Changes

- New package `app/packages/assistant_ui/`. Its allowlist is closed: `flutter`, `flutter_smooth_markdown`, `material_ui`. It has no Riverpod, no `assistant_api`, no I/O, and no platform branching of its own (`material_ui`'s `.adaptive` constructors are allowed). `flutter_smooth_markdown`'s built-in network image loading is closed off: the app supplies the image builder. A boundary test enforces this. Any later change that needs another dependency amends the allowlist requirement in its own spec delta.
- Render models (`ChatMessage`, `ToolCallRecord`, `ChatAttachment`, `PendingAttachment`, `PendingMessage`, `CommandEntry`, status/type enums, `TurnStatusSnapshot`, `TimelineDensity`, `EntryState`) move into the package unchanged.
- `TimelineEntryShell` owns expand/pin/auto-collapse/stale. Stateless bodies render each entry kind: `ThinkingBody`, `ToolCallBody`, `SubagentBody`, `CommandBody`.
- **Extension points for siblings:**
  - a `ToolResultRendererScope` carrying one resolver function that `ToolCallBody` consults, with the generic view as fallback (tool-result-renderers supplies its registry through it);
  - a `footer` slot on `TimelineEntryShell` (forwarded by `TimelineEntryView`) that stays visible when the entry is collapsed, plus `EntryState.awaitingInput`, for approval prompts and ask-the-user questions;
  - a `MessageActions` row taking caller-supplied actions, a `leading` slot (branch picker) and a `trailing` list (timing, then feedback);
  - `MessageBubble.extensions` and the shell `body` slot for generative UI (generative-ui-a2ui owns `SurfaceFrame`);
  - `ChatThreadLayout` slots (`banners`, `aboveComposer`) for interaction cards and follow-up chips;
  - a `starters` slot on `ThreadEmptyState`;
  - `ChatMarkdownScope` for markdown plugins and code-block builders;
  - a pure `ConversationListView` whose delete callback returns whether the row may be dismissed. It highlights no selected row; conversation-list-management adds `selectedId` and turns the highlight on.
- Composer family: `Composer`, `SlashCommandMenu`, `AttachmentTray`, `QueuedMessageBubble`. The app injects platform-adaptive and I/O widgets (text field, voice recorder) through slots.
- `app/widgetbook/` gallery (widgetbook 3.25.0), the `kAssistantUiElements` registry, and golden tests. Characterization goldens of today's app widgets are captured before any extraction and must be reproduced byte for byte.
- ADR-0021 records the package boundary, the pub workspace and the allowlist amendment rule, which bind the sibling changes. (0020 belongs to tool-result-renderers.)
- Dead `tool_call_chip.dart` and `timeline_section.dart` are deleted, with their tests. Neither is referenced from `lib/`.
- `ELEMENTS.md` keeps a verdict for each of the 59 assistant-ui elements (37 rows plus one 22-element Generative category row), now marking the ones siblings adopt.

## Capabilities

### New Capabilities

- `chat-element-library`: the pure element package, its extension points, gallery and element index.

### Modified Capabilities

- `tool-call-display`: removes the requirement that tool calls render as chips inside the assistant bubble (that widget is dead code), and rewords the persistence, status-icon and tool-only-row requirements from chips to timeline entries.

## Impact

- **Code**: new `app/packages/assistant_ui/` and `app/widgetbook/`; new `docs/adr/adr-0021-assistant-ui-element-package.md`. `app/lib/features/chat/` shrinks. The app's `pubspec.yaml` becomes a pub workspace root. CI runs package tests.
- **Visual change**: none intended. This is a behaviour-preserving extraction, and existing widget `Key`s are kept. Characterization goldens guard the timeline, bubble and composer; the Playwright chat baselines current on `main` must not move in any PR of this change.
- **Settled product decisions**: no selection highlight in the conversation list here (conversation-list-management turns it on); subagent tool calls stay compact rows and never use rich renderers; `ReconnectBanner` stays above the composer. See design.md Decision 8.
- **Non-goals**: sealing `ChatMessage`; reasoning step timelines (P5); mentions; the context ring (P4); a composer model picker; any server change; the siblings' own features. This change provides their slots, not their contents.
- **User-facing documentation**: not needed. A contributor `README.md` ships in the package.
