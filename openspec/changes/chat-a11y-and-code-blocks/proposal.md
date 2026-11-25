## Why

Four gaps separate our chat from assistant-ui and tool-ui (evidence in design.md "Context"):

1. **Code blocks.** `flutter_smooth_markdown`'s private `EnhancedCodeBlockBuilder` has an unlabelled hover-only copy button, platform branching, hardcoded colours, unchecked contrast and per-chunk re-highlighting. Sibling `tool-result-renderers` also needs a reusable `CodeBlock`.
2. **Accessibility.** Bubbles have no sender semantics, replies are never announced, several icon-only buttons are unlabelled, timeline headers are not keyboard-operable, scrolling ignores reduced motion, and no test checks accessibility guidelines.
3. **Breakpoints disagree.** `ChatScreen` splits at 700 px of window width, `NavShell` at 768 px, and the `ux-principles` skill documents 640/900.
4. **No response timing.** The runtime knows call durations and token counts but sends them only to OTel.

## What Changes

- **`CodeBlock` element** in `assistant_ui`: header with language label and an always-visible, labelled copy button; scrollable, selectable body highlighted by `highlight` 0.7.0 with WCAG-AA themes that own their background; memoised, size-capped. The allowlist gains `highlight`, and `ChatMarkdownScope` gains `onCopyCode` (**amends** chat-element-library). Markdown fences render through it.
- **Semantics:** sender and status on every bubble (including ", stopped"), focusable timeline headers, labels and adequate targets on every icon-only control.
- **Announcements:** `ChatNotifier` emits explicit turn lifecycle events; an app-side `ChatAnnouncer` speaks start, completion (with excerpt), stop, failure and stall, throttled, never per token. History loads are silent; resumed turns are announced.
- **Composer focus** follows visual order; **reduced motion** stops scroll, typing and copy animations.
- **Breakpoints:** one `Breakpoints.wide = 768`; `ChatScreen` compares its own pane width.
- **Per-message timing:** `run_turn_core` measures time to first token, total, output tokens and decode time; the timing crosses the bus, is stored in a `message_timings` table, and reaches the client on a typed `done` frame and `MessageSummary.timing`. A focusable `MessageTimingIndicator` shows it.
- An accessibility guideline suite covers the chat screen in both themes.

## Capabilities

### New Capabilities

- `chat-code-blocks`: `CodeBlock`, highlighting, themes, copy, markdown integration.
- `chat-accessibility`: semantics, announcements, labels, focus order, reduced motion, guideline suite.
- `layout-breakpoints`: shared tokens and the pane-width rule.
- `message-timing`: measurement, persistence, wire shape, display.

### Modified Capabilities

- `chat-element-library` (unarchived sibling): `highlight` allowlist, `ChatMarkdownScope.onCopyCode`. Archives after it.
- `chat-auto-scroll`: jump instead of animate under reduced motion.
- `conversation-event-log`: the persisted `done` payload carries `timing`.

## Impact

- **Server:** `crates/storage` (migration, `MessageTimingStore`), `crates/core` (`TurnTiming`, bus `TurnResult.timing`), `crates/runtime` (`run_turn_core`, worker, `persist_final_answer`), `crates/llm-provider` (timed scripted streams), `crates/web-ui` (`SseDoneEvent`, `MessageSummary.timing`), `openapi.json`, regenerated `assistant_api`.
- **Client:** `assistant_ui`, `app/lib/shared/layout/`, `nav_shell.dart`, `features/chat/`, goldens, baselines.
- **Dependencies:** `highlight` 0.7.0, already locked. No new crates.
- **Visual:** code-block header bar, timing icons, and the conversation list moves to the drawer for windows of 701–1008 px (sidebar expanded) or 701–840 px (collapsed).
- **Non-goals:** language auto-detection; line numbers or run buttons; live token display; thinking-duration timing; timing for cancelled, failed or Slack/Mattermost turns; Mermaid screen-reader support beyond a label; conversation-list keyboard deletion; non-chat WCAG audit.
- **Docs verdict:** user-facing documentation is needed. `docs/web-ui.md` gains "Response timing" and "Accessibility" sections (C12), and the `ux-principles` skill's breakpoint table is corrected (C1). No glossary terms. No ADR.
