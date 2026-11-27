## Context

This change builds on chat-element-library. All new widgets land in `app/packages/assistant_ui` with a widgetbook use-case and a golden. The seams it uses (PR numbers from chat-element-library's "Sibling unblock map", which is the source of truth) are:

| Seam (chat-element-library)                                                               | Used for                                      |
| ----------------------------------------------------------------------------------------- | --------------------------------------------- |
| `ChatMarkdownScope(plugins, builders, imageBuilder, onTapLink, …)` (PR 7a)                | `code_block` builder and `onCopyCode`         |
| `MessageActions(actions, leading, trailing: List<Widget>)`, `buildMessageActions` (PR 8a) | timing indicator, first in `trailing`         |
| `MessageBody`, `MessageBubble`, `TypingIndicator` (PR 8b)                                 | sender semantics, static typing indicator     |
| `ThreadDensityScope` (PR 5a), `TimelineEntryShell` (PR 5b)                                | density fed from pane width; focusable header |
| `ThreadViewport` (PR 10)                                                                  | reduced-motion scroll                         |
| `Composer`, `SlashCommandMenu` (PR 11a), `AttachmentTray` (PR 11b)                        | labels, focus order, `isStopping`             |
| `ThreadErrorBanner`, `TurnProgressCard` (PR 9)                                            | live-region error, labelled skip              |
| `ChatMessage` in the package (PR 4)                                                       | `timing` field                                |

Sibling `tool-result-renderers` C7 depends on this change's `CodeBlock` element (its tasks.md C7, design.md:81). That makes the element PR an early unblocker.

### Evidence: what is wrong today

**Code blocks.** The bubble enables `flutter_smooth_markdown`'s enhanced components (`app/lib/features/chat/message_bubble.dart:147,158`). For fenced code they give `EnhancedCodeBlockBuilder` (`flutter_smooth_markdown-0.8.1/lib/widgets/smooth_markdown.dart:687-706`), which highlights with `highlight` 0.7.0 and shows a copy button and a language tag. Its source (`lib/src/renderer/builders/enhanced_code_block_builder.dart`) shows:

- the copy button is an `InkWell` with no tooltip or semantics label (`:247-285`);
- on desktop and web it sits at opacity 0 until hover (`:243`), but can still be hit and focused while invisible;
- it branches on `defaultTargetPlatform` (`:176-177`);
- it hardcodes `Colors.green` and `Colors.black` (`:187,254,265`);
- the controls overlay the first line of code (`:206-209`);
- its GitHub and VS2015 themes were never checked against our contrast rule;
- it re-highlights the whole block on every build (`:130`), which during streaming means every chunk;
- the widget is private (`_EnhancedCodeBlockWidget`), so none of this can be fixed in place.

**Accessibility.**

- A message bubble has no semantics wrapper, so a screen reader hears its text with no sender.
- Streamed text is not announced at all.
- Icon-only buttons without a label: `send_button` and `stop_button` (`chat_input_row.dart:282-292`), `remove_attachment_N` (`:204-211`), the drawer menu (`chat_screen.dart:346-355`), the error-banner close (`chat_screen.dart:444-450`) and the image-viewer close (`message_attachments.dart:310-316`).
- The timeline entry header is a bare `GestureDetector` (`streaming_timeline_entry.dart:585`): not reachable or activatable by keyboard.
- Scroll-to-bottom animates even under reduced motion (`chat_screen.dart:77,149`).
- No test exercises Flutter's accessibility guidelines (`grep -rn meetsGuideline app/test` is empty).

**Breakpoints.** `ChatScreen` shows the inline conversation list above 700 px of _window_ width (`chat_screen.dart:302`). `NavShell` switches to the rail or sidebar at 768 px (`shared/nav_shell.dart:14,59`), the value `adaptive-shell` REQ-5 fixes. From 701 to 767 px the user gets a bottom nav bar and an inline list at once. At 768 px with the expanded sidebar (240 px plus a 1 px divider, `shared/nav/material_wide_layout.dart:17-19`), the 240 px inline list leaves 287 px for messages. The `ux-principles` skill documents a third set of values (640/900, `SKILL.md:58-68`) that no code uses.

**Timing.** The runtime knows each LLM call's duration (`orchestrator/mod.rs:1278`) and output token count (`LlmResponseMeta.output_tokens`, `crates/core/src/llm/types.rs:98`), but sends them only to OTel (`otel_spans.rs:302-345`).

### What flutter_smooth_markdown 0.8.1 offers

The source was read from `~/.pub-cache/hosted/pub.dev/flutter_smooth_markdown-0.8.1`:

- The default `CodeBlockBuilder` (`lib/src/renderer/builders/code_block_builder.dart:32-42`) renders plain monospace with no highlighting and no copy.
- `useEnhancedComponents: true` swaps in `EnhancedCodeBlockBuilder` (`widgets/smooth_markdown.dart:706`), which picks `githubTheme` or `vs2015Theme` from brightness (`enhanced_code_block_builder.dart:88-99`) and shows the copy button only on hover except on Android and iOS (`:176-177,243`).
- `codeBuilder: (code, language)` (`smooth_markdown.dart:382`) and `builderRegistry` (`:659`, merged on top of the enhanced registry at `:725-729`) are the two extension points. Registering `code_block` in `builderRegistry` overrides the enhanced builder and leaves the enhanced blockquote, link and header builders alone.
- Mermaid and SVG fences are parsed by block plugins before the code-block parser (`parser/plugins/mermaid_plugin.dart:86-88`), so a `code_block` override does not affect them.

### How a web turn reaches the client today

`send_message` calls `Orchestrator::submit_turn_with_request_id` (`orchestrator/worker.rs:701`), which publishes a `TurnRequest` on the bus and waits in `submit_turn_internal`. A worker picks it up and, because web turns carry a token sink, calls `run_turn_streaming` (`worker.rs:255-266`), which is `run_turn_core` (`mod.rs:1146-1148`) with a sink. `run_turn_core` streams through its own forward task (`mod.rs:1252-1272`) and saves the final answer (`mod.rs:1325-1340`). The worker then serialises the orchestrator `TurnResult` into `assistant_core::bus_messages::TurnResult` (`worker.rs:295-302`; struct at `crates/core/src/bus_messages.rs:107-128`), and `submit_turn_internal` rebuilds a new orchestrator `TurnResult` from the bus payload (`worker.rs:957-963`). Anything the web layer should see must survive both hops.

`run_turn_with_tools_impl` (`mod.rs:725`, forward task at `:860-869`) is the Slack/Mattermost extension-tool path. Its final answers return `message_id: None` (`turn_control.rs:145-151`, `mod.rs:1097-1103`), so there is no row to key timing by. It is out of scope.

## Goals / Non-Goals

**Goals:**

- Accessible, theme-correct code blocks that siblings reuse.
- A screen-reader experience in which every message has a sender and replies are announced once.
- A single breakpoint token.
- Server-measured response timing that survives replay and reload.

**Non-Goals:** see proposal.md.

## Decision 1: Own `CodeBlock` element instead of configuring `EnhancedCodeBlockBuilder`

`CodeBlock({required String code, String? language, ValueChanged<String>? onCopy, bool highlight = true})` is a `StatefulWidget` in `assistant_ui`. Its only state is the 2 s "copied" flag. Layout:

- **Header row** with the language label, rendered lowercase as written in the fence (`dart`, or `text` when absent), and a `MessageActionButton`-style copy button with tooltip "Copy code". The header is wrapped in `SelectionContainer.disabled` so that selecting across the block never copies the label.
- **Body**: `SingleChildScrollView(horizontal)` → `Text.rich`, so it takes part in the surrounding `SelectionArea` that `SmoothMarkdown(selectable: true)` installs.
- **Background**: `CodeTheme.background`, owned by the code theme (Decision 2), not taken from the ambient `ColorScheme`.

The copy button is always visible. Hover-only reveal needs either platform branching (forbidden in the package) or a control that is invisible but still hit-testable, which fails WCAG 2.4.7 and 2.5.8.

Alternatives:

- (a) Keep `useEnhancedComponents` and only fix the styles. Rejected: the widget is private.
- (b) Use the `codeBuilder` callback. Rejected: it replaces code rendering for every `SmoothMarkdown` call, and the builder-registry route composes better with `ChatMarkdownScope`.
- (c) Fork the package. Rejected: maintenance cost.

## Decision 2: Highlighter is `highlight` 0.7.0 inside the package; themes ported from highlight.js `a11y-*`

Candidates:

| Package                              | Version / freshness                                    | Grammar                                               | Init                | New bundle weight                                                                        |
| ------------------------------------ | ------------------------------------------------------ | ----------------------------------------------------- | ------------------- | ---------------------------------------------------------------------------------------- |
| `highlight` 0.7.0                    | latest, published ~5 years ago (pub.dev versions page) | highlight.js 10-era, 190 languages (`lib/languages/`) | sync                | **none**: already compiled in through `flutter_smooth_markdown` (`app/pubspec.lock:647`) |
| `re_highlight` 0.0.3                 | ~2 years ago                                           | highlight.js 11.9.0 port                              | sync                | a second full grammar set                                                                |
| `syntax_highlight` 0.5.0 (serverpod) | ~15 months ago                                         | TextMate, 15 languages                                | async, loads assets | grammar assets plus an async init path                                                   |

`highlight` is pure Dart (its only dependency is `collection`) and only renders, so it meets chat-element-library's amendment rule. Adding it as a direct dependency of `assistant_ui` lets `CodeBlock` highlight in goldens and in sibling renderers with no app plumbing. Its staleness is contained: `highlightCode(String code, String? language) → List<CodeToken>` is the only call site, so a later swap to `re_highlight` is a one-file change.

**Themes.** `CodeTheme.light` and `CodeTheme.dark` live in `lib/src/theme/code_theme.dart`, the theme dir where raw colours are allowed. Token colours are ported from highlight.js `a11y-light` / `a11y-dark` (MIT, also shipped as `flutter_highlight` 0.7.0 `themes/a11y-light.dart` and `a11y-dark.dart`), which were designed for WCAG AA. Each theme **owns its background colour** (`CodeTheme.background`), and `CodeBlock` paints it. The contrast test therefore checks every token colour against the exact colour it is drawn on (at least 4.5:1, `app-theme-tokens` "Migrated screens meet WCAG AA contrast"), and a change to the app's `ColorScheme` cannot silently break it. The theme is chosen from `Theme.of(context).brightness` through `legacyThemeOf`. That is the theme value, not a platform check.

Alternative: take the background from `surfaceContainerLowest`. Rejected: the golden harness's seeded `ColorScheme` and the app's real one can diverge, and a test against one does not protect the other.

**Memoisation.** Results are kept in a 64-entry LRU keyed by `(language, code)` with full string equality (no hash-only key, so a collision cannot return another block's tokens). A growing streaming fence must not flood the cache, so insertion follows one rule: when the most recently inserted entry has the same language and its code is a prefix of the new code, the new entry **replaces it in place** instead of adding a new one. During streaming the trailing fence therefore occupies one slot, finished blocks stay hits, and no stale partial strings accumulate. `StreamMarkdown` already throttles rebuilds (`stream_markdown.dart:339-340`). Blocks over 20,000 characters, unknown languages and an absent language render as plain monospace and are not memoised. There is no auto-detection, because `highlight.parse(autoDetection: true)` runs every grammar.

## Decision 3: Copy is a callback; `ChatMarkdownScope.onCopyCode` supplies it ambiently

The package must not do I/O, and chat-element-library's Composer requirement already forbids clipboard access. `CodeBlock.onCopy` wins when given. Otherwise the block uses `ChatMarkdownScope.of(context).onCopyCode`. When both are null the copy button is not rendered, so the gallery shows a block without copy unless the use-case supplies one.

The app installs `onCopyCode: (code) async { await Clipboard.setData(...); announcer.announce('Code copied'); }`. Tool-result renderers that use `CodeBlock` inside the timeline sit under the same scope, so they get copy for free.

This adds a field to a sibling-owned requirement, so the change carries a MODIFIED delta of "Ambient configuration is supplied by inherited scopes", copied verbatim from the sibling's current text with only the `ChatMarkdownScope` part changed. The scope count stays at three.

## Decision 4: Announcements are driven by explicit lifecycle events, never per token

`ChatState` does not carry the facts the announcements need. Stop today calls `cancelStream()`, which only drops the placeholder and clears `isSending` (`chat_provider.dart:274-287`); the stream loop breaks on `_session.cancelled` before any `agent_error` arrives, and `ErrorEvent` has no reason (`api/models/stream_event.dart:219-223`). A stall is a timer, not a state change. A reconnect replay (`_replayRun` → `_finishTurn`, `chat_provider.dart:477-525`) produces the same state transition as a live `done`. A diff of two `ChatState`s cannot tell these apart.

So `ChatNotifier` emits explicit lifecycle events on a broadcast stream (`ChatNotifier.lifecycle`), each carrying `at` (from the notifier's injected clock):

| Event                                                   | Emitted when                                                                                                                            | Announcement                                                                                                                    |
| ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `TurnStarted`                                           | a streaming placeholder is added for a live send                                                                                        | polite "{Persona} is responding"                                                                                                |
| `TurnCompleted{content, source: live\|replay}`          | `_finishTurn` applies `done` for a turn the notifier was tracking as in flight, whether the frames came live or from a reconnect replay | polite "{Persona} replied: {excerpt}" (excerpt: markdown stripped to plain text, cut to 200 chars at a word boundary, plus "…") |
| `TurnStopped`                                           | the turn reconciles to `MessageStatus.stopped` (chat-stop-cancels-turn PR 4 and PR 5a)                                                  | polite "Response stopped"                                                                                                       |
| `TurnFailed{message}`                                   | `_failTurn`                                                                                                                             | assertive "Response failed: {message}"                                                                                          |
| `TurnStalled`                                           | the status age crosses `kTurnStallThreshold`, from a timer the notifier arms on each status update                                      | polite "Still working", once per turn, suppressed while awaiting input                                                          |
| `TurnAwaitingInput{announcement}` / `TurnInputResolved` | emitted by tool-approval-prompts PR 16 when an interaction is pending / resolved                                                        | the emitter's text (polite); sets and clears the "awaiting input" flag                                                          |

Loading a conversation's history emits **no** lifecycle events, so it is silent by construction. A reconnect that resumes a turn the user started (or a run that was still active when the conversation was opened) emits `TurnCompleted{source: replay}` when it finishes, and that **is** announced: the user is waiting for it. This is a final decision: a reconnect replay is announced exactly like a live completion, because from the listener's side the reply they were waiting for has arrived.

`TurnStarted` also resets the per-turn flags (the "Still working" count and the awaiting-input flag). A turn abandoned while an approval was pending (message-branching auto-cancels a pending approval when the user edits, regenerates or switches branch) therefore cannot leave the flag set and suppress the next turn's stall announcement, and no new lifecycle event is needed for that path. The user started that action, so it is not announced.

`ChatAnnouncer` (`app/lib/features/chat/chat_announcer.dart`) is a pure policy over this event stream plus a clock, producing `Announcement{text, assertive}`. A thin `ChatAnnouncementSink` calls `SemanticsService.sendAnnouncement(View.of(context), text, TextDirection.ltr, assertiveness:)` (`announce` is deprecated in Flutter 3.50.1, `packages/flutter/lib/src/semantics/semantics_service.dart:51-56,81-92`). It lives in the app because it reacts to `chatProvider`, and the package stays free of platform channels. Other features (tool-approval-prompts, code copy) announce through the same sink so throttling is global.

**Throttle model (one model, fake-clock testable).**

- A polite announcement is **held** in a single slot and spoken when the slot's deadline passes. The deadline is set to `at + 1.5 s` when the slot goes from empty to full. A newer polite announcement arriving while the slot is full **replaces** its text and keeps the deadline. So no polite announcement waits more than 1.5 s, and bursts collapse to the latest.
- An assertive announcement is spoken **at once** and **drops** any held polite one.

Examples on a fake clock: start at t=0 and completion at t=0.8 → only the completion is spoken, at t=1.5. Start at t=0, completion at t=4 → start spoken at t=1.5, completion at t=5.5. Start at t=0, failure at t=0.5 → the failure is spoken at t=0.5 and nothing at t=1.5.

The streaming bubble is **not** a live region. On Android, TalkBack re-reads a live region on every content change, which during streaming means every token.

Alternatives:

- (a) Mark the bubble `liveRegion: true`. Rejected: per-token chatter.
- (b) Derive announcements from `(previous ChatState, next ChatState)`. Rejected: see the first paragraph.
- (c) Announce the full reply. Rejected: long replies would monopolise the reader, and the transcript is one keystroke away.
- (d) Announce the Persona name only ("Ada replied"). Rejected: the user then has to navigate to the message to learn whether it needs attention. The excerpt is the first 200 characters, so a short answer is heard whole.

**Decided (final):** completion is announced as "{Persona} replied: {excerpt}", with the excerpt as described in the table (plain text, at most 200 characters, cut at a word boundary, "…" when cut).

## Decision 5: Message semantics shape

`MessageBubble` wraps its content in `Semantics(container: true, label: '{sender}{status}')`:

- `sender` is a new required `senderLabel` parameter. The app passes "You" for the user's own messages and the active Persona's name for assistant messages (`chat_screen.dart:313`).
- `status` is one of ", sending", ", failed", ", responding", ", stopped" (`MessageStatus.stopped`, from chat-stop-cancels-turn), or empty.

The `stopped` status reaches the bubble from `ChatMessage.status`. A live stop sets it through chat-stop-cancels-turn PR 4, and a reloaded conversation sets it from the persisted `finish_reason` (chat-stop-cancels-turn PR 2a and 5c), so the ", stopped" label survives a reload with no work in this change beyond the label itself.

The content stays as child nodes rather than merged, so links, code-block copy buttons and actions remain individually focusable. `TypingIndicator` carries `Semantics(label: 'typing')`, and under reduced motion it renders its dots static. `MessageActions` is a separate container after the bubble in traversal order.

`TimelineEntryShell`'s header switches from `GestureDetector` to a focusable control (`FocusableActionDetector` with `ActivateIntent` plus `InkWell`) that keeps `Semantics(button: expandable, expanded: …)`. Its label is `"{label}, {status}"`, where status is "running", "done", "failed", "awaiting input" or "stale".

## Decision 6: One breakpoint token, compared against the region being split

`app/lib/shared/layout/breakpoints.dart` declares `abstract final class Breakpoints { static const double wide = 768; }`. The value is the one `adaptive-shell` REQ-5 fixes.

- `NavShell` compares the window width, as today.
- `ChatScreen` places a `LayoutBuilder` **above its `Scaffold`**, because `isWide` drives the body, `AppBar.leading` and `drawer` (`chat_screen.dart:302,345-361`). It compares the width of its own pane to decide between the inline list and the drawer. The rule: a region splits into list and detail only when the region itself is at least `Breakpoints.wide`.
- `TimelineDensity` keeps its thresholds (400/700) as named package constants, `kTimelineCompactMaxWidth` and `kTimelineExpandedMinWidth`. They describe content density, not layout. They are fed by the thread pane's width through `ThreadDensityScope`'s input in `ChatScreen`, instead of `MediaQuery.size.width` (`chat_screen.dart:510-512`).

Effect (pane = window − 241 px with the expanded sidebar, window − 73 px with the collapsed one, window below 768 px): the list moves from inline to the drawer for windows of **701–1008 px with the sidebar expanded** and **701–840 px with it collapsed**. The 701–767 px overlap with the bottom nav disappears.

**Sidebar animation.** The sidebar animates its width over 200 ms (`material_wide_layout.dart:19,41-46`), so the pane width passes through intermediate values. The decision is a pure function of the current pane width and the animation is monotonic, so it flips at most once per toggle. An open drawer is closed when the layout flips to inline (the `Scaffold` no longer has a drawer). A widget test toggles the sidebar at 900 px and asserts exactly one flip and no exceptions.

**Decided (final):** `ChatScreen` compares its own pane width. The user accepted the behaviour change for windows of 701–1008 px (sidebar expanded) and 701–840 px (collapsed), where the list now lives in the drawer. A region is split into list and detail only when the region itself is wide enough, which is the same rule `adaptive-shell` applies to the window.

Alternatives:

- (a) Compare the window width in `ChatScreen` against 768. Rejected: removes the 701–767 overlap but keeps a 287 px message column at 768 px.
- (b) Adopt Material 3 window classes (600/840/1200). Rejected: re-decides `adaptive-shell` REQ-5 and moves every nav test.
- (c) Feed the pane width as an explicit input from `NavShell`. Rejected: `LayoutBuilder` gets the same number with no coupling.

The `ux-principles` skill's 640/900 table is corrected to match.

## Decision 7: Timing is measured in `run_turn_core`, carried across the bus, persisted, and delivered on `done`

The client cannot count tokens (chunks are not tokens) and loses everything on reload, so measurement happens on the server.

**Where.** `run_turn_core` (`mod.rs:1148`) is the path for every web, CLI and Signal turn. It takes `t0 = self.clock.now_instant()` on entry. Per LLM call it takes `call_start` and `call_end` from `self.clock` (the existing `llm_start = std::time::Instant::now()` at `mod.rs:1251` keeps feeding OTel metrics unchanged).

- **With a token sink** (`mod.rs:1252-1272`), the forward task records, **immediately after `chunk_rx.recv()` returns and before any `.await`**, the instant of the first chunk of any kind for this call, and the first `StreamChunk::Text` of the turn. They go into a `tokio::sync::Mutex<CallMarks>` shared with the loop and read after `forward_handle.await`.
- **Without a token sink** (`run_turn`, `llm.chat`), `time_to_first_token_ms` is `None` and each call contributes its whole duration to decode time.

**Accumulation.** After each call, `output_tokens += meta.output_tokens`; if any call reports `None`, the turn's `output_tokens` becomes `None` (no estimation). `decode += call_end − first_chunk` for a call that streamed at least one chunk. **A call that produced no `StreamChunk`** (non-streaming, or a streaming call that only returned tool calls, since tool-call arguments emit no chunk) contributes its **whole duration** `call_end − call_start`. One rule for all chunk-less calls; it slightly understates tok/s on agentic turns (prefill is counted) rather than overstating it.

**Persistence and return.** At the final answer (`mod.rs:1325-1340`), `total_ms = now − t0` is taken just before the save, and `TurnTiming { time_to_first_token_ms: Option<u64>, total_ms: u64, output_tokens: Option<u64>, decode_ms: Option<u64> }` is written through `persist_final_answer` (introduced by message-feedback PR 3), which this change extends with `FinalAnswerMeta.timing` and a `MessageTimingStore::put(conversation_id, message_id, &timing)`. A storage failure is logged with `warn!` and does not fail the turn. The timing is returned on the orchestrator's `TurnResult.timing` (`mod.rs:80-99`). No timing is produced when the answer is empty (no `message_id`), on cancellation, or on error.

**Across the bus.** `assistant_core::bus_messages::TurnResult` gains `timing: Option<TurnTiming>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. The worker copies it at `worker.rs:295-302`, and `submit_turn_internal` copies it back at `worker.rs:957-963`. Without both copies `done.timing` would always be absent on the web. A test sends a turn through `submit_turn_with_request_id` and the bus and asserts the timing arrives.

**Constructor sites** that gain `timing` (all `None` except the two above and the final-answer return): orchestrator `TurnResult` at `mod.rs:1097`, `mod.rs:1369`, `turn_control.rs:53`, `turn_control.rs:145`, `worker.rs:957`, `orchestration.rs:153,184`, `interface_trait.rs:211`, `web-ui/src/a2a/handlers.rs:942`, `mcp-server/tests/io_loop.rs:153,160`, `mcp-server/tests/dispatch.rs:167`; bus `TurnResult` at `worker.rs:295`, `worker.rs:489`, `title_generator.rs:680,739`.

**Extension-tool path.** `run_turn_with_tools_impl` (Slack/Mattermost) gets no timing: its final answers carry no `message_id`, and no client displays timing there.

**Deterministic tests.** `ScriptedLlmProvider` has no clock or per-call timing (`llm-provider/src/scripted.rs:93-96`; its single chunk queue is consumed by each call). This change adds `with_timed_streaming_calls(clock: FakeClock, calls: Vec<TimedCall>)`, where a `TimedCall` is a list of `(advance_before: Duration, StreamChunk)` steps plus an `advance_before_return`. After each `send`, the scripted provider waits until the channel is drained (`tx.capacity() == tx.max_capacity()`, yielding in between) before it advances the clock again. Together with the forwarder recording the instant immediately after `recv()`, and the tests running on the current-thread runtime (`#[tokio::test]` default), the measured instants are exact. Production channel capacity stays at 64.

**On the wire.**

- **`done` frame:** today an untyped `serde_json::json!` (`messages.rs:416-434`) with no OpenAPI schema (the 200 response of `POST /api/conversations/{id}/messages` is `{"text/event-stream": {}}`; components have `SseTokenEvent` and siblings, no done event). This change introduces a typed `SseDoneEvent { role, content, message_id?, attachment_ids?, timing?: MessageTimingSummary }` (`utoipa::ToSchema`), registers it in `openapi.rs` components next to `SseTokenEvent` (`openapi.rs:52,546`), and builds the terminal `done` frame from it in the shared runner `stream_turn_to_sse` (the extraction PR owned by message-branching, PR 5 in the stack plan). Voice turns use the same runner after chat-stop-cancels-turn PR 3, so there is one build site. The server measures, stores and sends timing for every eligible turn regardless of the client preference in Decision 9: replay frames are shared by all viewers, and storing it keeps old replies' timing available if the user turns the preference on later. The terminal frame is already persisted to `conversation_event_store` (`messages.rs:452-456`), so replay carries `timing` with no new event type.
- **History:** `MessageSummary.timing: Option<MessageTimingSummary>` for the assistant row that has a timing row, loaded with one `get_for_conversation` call per request.
- **tokens/s:** clients compute `output_tokens / (decode_ms / 1000)`, shown only when both are present and `decode_ms ≥ 250`. Durations are integer milliseconds, not timestamps, so the RFC 3339 rule does not apply.

**Storage.** A side table, `message_timings(message_id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE, time_to_first_token_ms INTEGER, total_ms INTEGER NOT NULL, output_tokens INTEGER, decode_ms INTEGER)` with an index on `conversation_id`. It has **no foreign key to `messages`**: context compaction's `replace_history` deletes every message of the conversation and re-inserts the kept ones (`storage/src/conversations.rs:369-386`), and a cascade from `messages` would wipe the timings of kept messages. With the conversation-level cascade, kept messages keep their timing, rows of messages compaction dropped are orphaned but never read (history only returns live message ids), and everything goes when the conversation is deleted. Reads are `get_for_conversation(conversation_id) -> HashMap<Uuid, TurnTiming>`, a single indexed query instead of an unbounded `IN` list. It follows ADR-0009 with `InMemoryMessageTimingStore` and `SqliteMessageTimingStore`. A side table keeps `assistant_core::Message` untouched while message-branching, tool-result-renderers and message-feedback edit it.

Alternatives:

- (a) Measure on the client: time from `run_started` to the first `token` and to `done`. Rejected: no real token counts, skewed by network buffering, lost on reload.
- (b) A per-call `usage` SSE event. Rejected: a projection variant and a client event for numbers needed once.
- (c) Columns on `messages`. Rejected: collides with the siblings' schema changes in the contended `messages` lane.
- (d) A foreign key to `messages(id)`. Rejected: compaction wipes it (above).
- (e) Instrument `run_turn_with_tools_impl` too. Rejected: no message id, no display.

## Decision 8: Timing display

`MessageTimingIndicator(timing)` is a package element. `buildMessageActions` places it **first** in `MessageActions.trailing` (a `List<Widget>`; feedback follows it, chat-element-library PR 8a) for finished assistant messages that have `timing`, **only when the "Show response timing" preference (Decision 9) is on**. The builder takes the preference as a plain `bool showResponseTiming` argument, so the package element stays free of app state.

- It is a **focusable button** in the `MessageActionButton` style (24×24 minimum hit area) showing a 14 px `Icons.speed_outlined` glyph, wrapped in a `Tooltip` with a `GlobalKey<TooltipState>`.
- Material's tooltip (Flutter 3.50.1) shows on mouse hover and, with `triggerMode: TooltipTriggerMode.tap`, on tap. It does **not** show on keyboard focus: `RawTooltip` installs only the recogniser for its trigger mode (`widgets/raw_tooltip.dart:683-698`), and its only focus use is Escape-to-dismiss (`:596-615`). So the button calls `TooltipState.ensureTooltipVisible()` when it gains focus and when it is activated (Enter, Space or tap), which also covers touch. Long-press is not a trigger: it adds a second gesture for the same result.
- The tooltip text is `formatMessageTiming(timing)`, for example "First token 0.8 s · Total 12.4 s · 42 tok/s".
- The semantics label is `describeMessageTiming(timing)`, for example "Response timing: first token 0.8 seconds, total 12.4 seconds, 42 tokens per second". Screen-reader users hear it without a hover.

Formatting rules: values under 10 s show one decimal, 10 s to under 60 s show whole seconds, longer values show "1 m 5 s". Missing parts are dropped.

## Decision 9: Response timing is shown only when the user turns on "Show response timing"

The preference is **off by default**. Timing numbers are useful to a minority (people comparing models or debugging slow turns) and are visual noise on every reply for everyone else, so the default chat looks the same as today and the indicator is opt-in.

- **Storage.** A per-device preference in `SharedPreferences` under `assistant.showResponseTiming` (boolean, absent means `false`), exposed as `showResponseTimingProvider`, an `AsyncNotifier<bool>` modelled on `sidebarCollapsedProvider` (`shared/nav/sidebar_collapsed_provider.dart`): reads and writes are wrapped in `try`/`catch`, a failed read means `false`, and a write updates state first. It lives in `app/lib/features/settings/response_timing_preference.dart`.
- **Control.** `SettingsScreen` gains a "Show response timing" switch row with the subtitle "Show first-token time, total time and tokens per second under finished replies." The switch has a semantics label equal to its title and toggles on Enter, Space and tap.
- **Gating.** `ChatScreen` watches the provider and passes `showResponseTiming` to `buildMessageActions`. With the preference off, `timing` is still parsed and stored on `ChatMessage`, and the indicator is simply not built; turning the preference on shows the indicator on every finished reply that has timing, including ones already on screen and loaded history.
- **Scope.** Server behaviour does not depend on the preference (Decision 7).

Alternatives:

- (a) Always show the icon (the earlier default). Rejected by the user decision: noise for most users.
- (b) A server-side per-user preference. Rejected: there is no user UI-preference store today, it would add an endpoint, an `openapi.json` change and a regenerated client for one boolean, and the sidebar collapse state already follows the per-device pattern.
- (c) Have the server omit `timing` when the preference is off. Rejected: replay frames are shared, the stored value would be lost for later, and it couples measurement to presentation.

## Risks / Trade-offs

- **`highlight` 0.7.0 is unmaintained.** Mitigated by the single call site and the swap path above, and it adds no weight. Its grammars lag highlight.js 11, so some newer syntax (for example Rust let-else) colours imperfectly.
- **Highlighting cost on huge blocks.** Mitigated by the 20k-character cap and the LRU.
- **Visual baseline moves.** The code block gains a header bar, and the timing icon appears. Never merge Playwright PNGs by hand: after a rebase, regenerate with the e2e-testing skill and record the diff in the PR body.
- **Announcements depend on platform support.** On web, Flutter's semantics tree must be enabled (it is, once an assistive technology is detected or the user activates the "Enable accessibility" placeholder). The sink is a no-op in tests unless the accessibility channel is mocked, and the tests mock it.
- **Breakpoint behaviour change** for windows of 701–1008 px (sidebar expanded) or 701–840 px (collapsed): the user sees a drawer where an inline list used to be. Accepted (Decision 6).
- **Timing is undiscoverable by default.** The indicator is hidden until the user finds the setting. Accepted; C12's docs name the setting in the "Response timing" section.
- **Timing excludes server queue wait.** `t0` is the start of the orchestrator turn, not request arrival. The per-conversation worker queue is normally empty, so the difference is negligible.
- **ttft includes forwarder lag.** In production the instant is taken when the forwarder receives the chunk, not when the provider produces it. The 64-slot channel is normally drained, so the lag is microseconds.
- **Providers without usage** (for example OpenAI-compatible streaming without `stream_options.include_usage`) show no tok/s. Honest, but uneven across providers.
- **Contended files.** The runner (`stream_turn_to_sse`), the `messages` save sites (`persist_final_answer`), `get_conversation`/`MessageSummary` and the client stream lane are edited by several siblings. This change edits them only after the owning extraction PRs land (see tasks.md "Sibling coordination").

## Migration Plan

- New table only, with no backfill. Old messages have no timing and show no indicator. The preference needs no migration: an absent key reads as off.
- The migration is `053_message_timings.sql`, the number the cross-change stack plan pre-allocates to this change (`openspec/chat-ui-parity-stack.md`). It is registered in the `run_migrations` list in `crates/storage/src/lib.rs` (`:273`, list ending at `:472`).
- Rollback: the table is additive. Reverting the code leaves it unused.
