## ADDED Requirements

### Requirement: Message bubbles expose sender and status semantics

`MessageBubble` SHALL take a required `senderLabel` and SHALL wrap its content in a semantics container whose label begins with the sender and appends the status: ", sending" while pending, ", failed" when failed, ", responding" while streaming, ", stopped" for `MessageStatus.stopped` (a reply the user stopped), and nothing otherwise. The app SHALL pass "You" for the current user's messages and the active Persona's name for assistant messages.

The bubble's interactive descendants (links, code-block copy buttons) SHALL remain separately focusable and SHALL NOT be merged into the container's label. While the typing indicator shows, it SHALL carry the semantics label "typing". `MessageActions` SHALL follow the bubble in traversal order.

#### Scenario: Assistant reply announces its sender

- **GIVEN** a finished assistant `MessageBubble` with `senderLabel: 'Ada'`
- **WHEN** its semantics are inspected
- **THEN** a container node SHALL have a label starting with "Ada" AND the message text SHALL be reachable as a descendant node

#### Scenario: Stopped label survives reload

- **GIVEN** a conversation reloaded from history whose last assistant row carries the persisted stopped `finish_reason`, so its `ChatMessage.status` is `stopped`
- **WHEN** the conversation renders with `senderLabel: 'Ada'`
- **THEN** the bubble's container label SHALL be "Ada, stopped"

#### Scenario: Status is part of the label

- **WHEN** a user bubble with status `failed` renders with `senderLabel: 'You'`
- **THEN** the container label SHALL be "You, failed"
- **WHEN** an assistant bubble is streaming
- **THEN** its container label SHALL end with ", responding"
- **WHEN** an assistant bubble with status `stopped` renders with `senderLabel: 'Ada'`
- **THEN** the container label SHALL be "Ada, stopped"

#### Scenario: Links stay individually focusable

- **GIVEN** an assistant bubble containing a markdown link
- **WHEN** semantics are inspected
- **THEN** the link SHALL be its own node with the link flag, separate from the container

### Requirement: Timeline entry headers are keyboard operable

The `TimelineEntryShell` header SHALL be focusable. It SHALL toggle expansion on Enter and Space when the entry is expandable, and SHALL expose `button` and `expanded` semantics. Its label SHALL be "{label}, {status}", where status is "running" (active), "done" (complete), "failed" (error status), "awaiting input" (`EntryState.awaitingInput`) or "stale". Focus SHALL be visibly indicated.

#### Scenario: Keyboard toggles an entry

- **GIVEN** an expandable, collapsed tool-call entry
- **WHEN** focus moves onto its header with Tab and Enter is pressed
- **THEN** the entry SHALL expand AND its semantics SHALL report `expanded: true`

#### Scenario: Status is spoken

- **WHEN** an active tool-call entry labelled "web_search" renders
- **THEN** its header semantics label SHALL be "web_search, running"

### Requirement: Icon-only controls have labels and adequate targets

Every icon-only control in the chat surface SHALL have a tooltip and a semantics label: send, stop, attach, voice, remove-attachment, scroll-to-bottom, drawer menu, error dismiss, image-viewer close, the timeline header, the code-block copy and the timing indicator. The labels are:

| Control                   | Label                                                    |
| ------------------------- | -------------------------------------------------------- |
| `send_button`             | "Send message"                                           |
| `stop_button`             | "Stop response"; "Stopping…" while disabled and stopping |
| `attach_image_button`     | "Attach image"                                           |
| voice button              | "Send voice message"; "Stop recording" while recording   |
| `remove_attachment_N`     | "Remove {filename}"                                      |
| `scroll_to_bottom_button` | "Scroll to bottom"                                       |
| drawer menu               | "Open conversations"                                     |
| error-banner close        | "Dismiss error"                                          |
| image-viewer close        | "Close image"                                            |

Composer controls SHALL have a hit area of at least 48×48 logical pixels, except the attachment remove control. The remove control, the message-action buttons, the code-block controls and the timing indicator SHALL have a hit area of at least 24×24 (WCAG 2.2 SC 2.5.8). The remove control's hit area SHALL lie entirely inside its attachment tile, so that every point of it receives taps.

#### Scenario: Labelled tap target guideline passes

- **GIVEN** the chat screen pumped with a fixture conversation containing a user message with an attachment, an assistant reply with a code fence, a tool call, and an error banner
- **WHEN** `expect(tester, meetsGuideline(labeledTapTargetGuideline))` runs
- **THEN** it SHALL pass

#### Scenario: Composer targets meet 48 dp

- **GIVEN** a `Composer` with voice enabled and no pending attachments
- **WHEN** `meetsGuideline(androidTapTargetGuideline)` runs against it
- **THEN** it SHALL pass

#### Scenario: Remove control is hit at its edge

- **GIVEN** an `AttachmentTray` with one 64 px attachment tile
- **WHEN** a tap lands 1 px inside each edge of `remove_attachment_0`'s hit area
- **THEN** each tap SHALL invoke the remove callback AND the hit area SHALL be at least 24×24 AND SHALL lie within the tile's bounds

#### Scenario: Stop button is labelled

- **GIVEN** a `Composer` with `isSending: true`
- **WHEN** semantics are inspected
- **THEN** `stop_button` SHALL have the label "Stop response"

### Requirement: Replies are announced once, never per token

`ChatNotifier` SHALL emit explicit turn lifecycle events, each with a timestamp from its injected clock: `TurnStarted`, `TurnCompleted{content, source: live|replay}`, `TurnStopped`, `TurnFailed{message}`, `TurnStalled`, `TurnAwaitingInput{announcement}` and `TurnInputResolved`. `TurnCompleted` SHALL be emitted only for a turn the notifier was tracking as in flight, whether its frames arrived live or through a reconnect replay. Loading a conversation's history SHALL emit no lifecycle events.

A pure `ChatAnnouncer` policy SHALL map these events to announcements, sent through `SemanticsService.sendAnnouncement`:

- `TurnStarted`: polite "{Persona} is responding";
- `TurnCompleted`: polite "{Persona} replied: {excerpt}", where the excerpt is the reply converted from markdown to plain text and cut to at most 200 characters at a word boundary, followed by "…" when cut;
- `TurnStopped`: polite "Response stopped";
- `TurnFailed`: assertive "Response failed: {message}";
- `TurnStalled`: polite "Still working", at most once per turn, and not while the turn is awaiting input (between `TurnAwaitingInput` and `TurnInputResolved`). `TurnStarted` SHALL reset the per-turn stall count and the awaiting-input flag;
- `TurnAwaitingInput`: its polite `announcement` text.

Throttling SHALL follow one model. A polite announcement is held in a single slot. When the slot goes from empty to full, its deadline is set to the announcement's time plus 1.5 s, and it is spoken when the deadline passes. A polite announcement that arrives while the slot is full SHALL replace the held text and keep the deadline. An assertive announcement SHALL be spoken at once and SHALL drop any held polite announcement.

Token frames SHALL NOT produce announcements. The streaming message SHALL NOT be a live region. The error banner SHALL be a live region. Other app features (code copy, tool approvals) SHALL announce through the same sink and throttle.

#### Scenario: One announcement per completed reply

- **GIVEN** a fake clock, `TurnStarted` at t=0, 200 token frames, and `TurnCompleted` with content "**Hello** there" at t=4 s
- **WHEN** the policy runs to t=6 s
- **THEN** exactly two announcements SHALL be sent: "Ada is responding" at t=1.5 s and "Ada replied: Hello there" at t=5.5 s

#### Scenario: Long reply is excerpted

- **WHEN** `TurnCompleted` carries a 1,000-character reply
- **THEN** the announcement's excerpt SHALL be at most 200 characters, SHALL end at a word boundary, and SHALL be followed by "…"

#### Scenario: Rapid transitions coalesce

- **GIVEN** a fake clock, `TurnStarted` at t=0 and `TurnCompleted` at t=0.8 s
- **WHEN** the policy runs to t=3 s
- **THEN** only the completion announcement SHALL be sent, at t=1.5 s

#### Scenario: Failure is spoken at once

- **GIVEN** a fake clock and `TurnStarted` at t=0, still held
- **WHEN** `TurnFailed` with "rate limited" arrives at t=0.5 s
- **THEN** an assertive "Response failed: rate limited" SHALL be sent at t=0.5 s AND nothing SHALL be sent at t=1.5 s

#### Scenario: Stop is announced

- **GIVEN** a live turn
- **WHEN** the user presses Stop and the turn reconciles to a stopped message
- **THEN** "Response stopped" SHALL be sent

#### Scenario: Stall is not announced while awaiting input

- **GIVEN** `TurnAwaitingInput` has been emitted and not resolved
- **WHEN** `TurnStalled` arrives
- **THEN** no "Still working" announcement SHALL be sent

#### Scenario: Reconnect announces the resumed completion

- **GIVEN** a live turn whose stream drops and is resumed by replay
- **WHEN** the replayed `done` is applied
- **THEN** `TurnCompleted{source: replay}` SHALL be emitted AND the completion SHALL be announced once

#### Scenario: Abandoned approval does not suppress the next stall announcement

- **GIVEN** `TurnAwaitingInput` was emitted and the turn was then abandoned without `TurnInputResolved` (the user edited the message, so the server auto-cancelled the pending approval)
- **WHEN** a new turn emits `TurnStarted` and later `TurnStalled`
- **THEN** "Still working" SHALL be sent for the new turn

#### Scenario: Completion names the Persona and excerpts the reply

- **GIVEN** a Persona named "Ada" and a completed reply
- **WHEN** the completion is announced
- **THEN** the text SHALL be "Ada replied: " followed by the excerpt, AND SHALL NOT be only the Persona name or the full reply

#### Scenario: History load is silent

- **WHEN** a conversation with ten finished messages is opened
- **THEN** no lifecycle event and no announcement SHALL be produced

#### Scenario: Screen sends through the accessibility channel

- **GIVEN** a widget test that mocks `SystemChannels.accessibility`
- **WHEN** a scripted turn completes on the chat screen
- **THEN** the channel SHALL receive an `announce` message with the completion text

### Requirement: Composer focus follows visual order and is preserved

Focus traversal in the composer SHALL follow visual order: attach, voice (when shown), input, then send or stop. After send, focus SHALL remain in or return to the input. When the stop button replaces send while it holds focus, focus SHALL move to the stop button. When stop is activated, focus SHALL move to the input before the stop button becomes disabled ("Stopping…"), and SHALL stay there after the turn ends. Escape SHALL close an open slash-command menu and keep focus in the input. The composer SHALL NOT introduce focus nodes that are not visible controls.

#### Scenario: Tab order

- **GIVEN** a `Composer` with voice enabled and focus on the attach button
- **WHEN** Tab is pressed three times
- **THEN** focus SHALL land in turn on the voice button, the input, and `send_button`

#### Scenario: Focus returns after send

- **WHEN** the user types text in the input and presses Enter
- **THEN** after `onSend` fires the input SHALL have focus

#### Scenario: Stop keeps focus on a usable control

- **GIVEN** `send_button` has focus
- **WHEN** it is activated and the composer rebuilds with `isSending: true`
- **THEN** `stop_button` SHALL have focus
- **WHEN** `stop_button` is activated and the composer rebuilds with `isStopping: true`, so the stop button is disabled and labelled "Stopping…"
- **THEN** the input SHALL have focus
- **WHEN** the composer then rebuilds with `isSending: false`
- **THEN** the input SHALL still have focus

#### Scenario: Escape closes the slash menu

- **GIVEN** the slash-command menu is open
- **WHEN** Escape is pressed
- **THEN** the menu SHALL close AND the input SHALL keep focus AND its text SHALL be unchanged

### Requirement: Chat motion honours reduced-motion settings

When `MediaQuery.disableAnimations` is true:

- `ThreadViewport` SHALL jump to positions instead of animating, both for auto-scroll and for the scroll-to-bottom button;
- `TimelineEntryShell` SHALL change size without `AnimatedSize`;
- `CodeBlock` copied feedback SHALL appear without a fade;
- the typing indicator SHALL be static.

Indeterminate progress indicators MAY keep animating, because they convey status.

#### Scenario: Scroll jumps with reduced motion

- **GIVEN** `disableAnimations: true` and a viewport scrolled away from the bottom
- **WHEN** `scroll_to_bottom_button` is activated
- **THEN** the scroll offset SHALL equal `maxScrollExtent` after a single `pump()` with no `pumpAndSettle` needed

#### Scenario: Typing indicator is static with reduced motion

- **GIVEN** `disableAnimations: true`
- **WHEN** a `TypingIndicator` is pumped and the clock advances 1 s
- **THEN** no frame SHALL be scheduled by the indicator AND its rendering SHALL be identical before and after

#### Scenario: Scroll animates normally

- **GIVEN** `disableAnimations: false`
- **WHEN** `scroll_to_bottom_button` is activated
- **THEN** the offset after one 16 ms frame SHALL be strictly between the start offset and `maxScrollExtent`

### Requirement: Accessibility guidelines are tested in both themes

`app/test/widget/features/chat/chat_accessibility_test.dart` SHALL pump the chat screen with a fixture conversation (user message, assistant reply with code and a link and timing, tool call, thinking entry, error banner) in light and dark themes, with the "Show response timing" preference overridden to on so the timing indicator is present, and SHALL assert `labeledTapTargetGuideline`, `textContrastGuideline`, and that every node with a tap action has a non-empty label. The `assistant_ui` package SHALL include semantics tests for `MessageBubble`, `TimelineEntryShell`, `Composer`, `CodeBlock` and `MessageTimingIndicator`.

#### Scenario: Timing indicator is absent by default and passes when on

- **GIVEN** the fixture screen with the preference at its default
- **WHEN** it is pumped
- **THEN** no `MessageTimingIndicator` SHALL be present AND the guidelines SHALL pass
- **WHEN** the preference is overridden to on
- **THEN** the indicator SHALL be present, labelled with the timing sentence, AND the guidelines SHALL pass

#### Scenario: Suite runs in CI

- **WHEN** `make test-flutter` runs
- **THEN** the accessibility suite SHALL run for both themes AND fail on any guideline violation

#### Scenario: Contrast holds in dark mode

- **GIVEN** the fixture screen in the dark theme
- **WHEN** `meetsGuideline(textContrastGuideline)` runs
- **THEN** it SHALL pass
