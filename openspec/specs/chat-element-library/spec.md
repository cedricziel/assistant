# chat-element-library Specification

## Purpose

The pure `assistant_ui` element package: its dependency allowlist, render models, inherited scopes, elements, gallery and the thin chat-screen adapter.

## Requirements

### Requirement: Element package is pure (no state management, no I/O, no platform branching)

The `assistant_ui` package at `app/packages/assistant_ui/` SHALL declare only `flutter`, `flutter_smooth_markdown`, `material_ui` and `highlight` under `dependencies`. `highlight` is pinned to the version already resolved transitively through `flutter_smooth_markdown` (0.7.0); it is pure Dart and only tokenises source text for `CodeBlock`. Package source SHALL NOT import `flutter_riverpod`, `assistant_api`, `cupertino_ui`, `package:flutter/cupertino.dart`, `dart:io`, or any I/O or platform-channel package (`record`, `file_picker`, `desktop_drop`, `url_launcher`, `audioplayers`, `http`, `cached_network_image`, `flutter_svg`, `share_plus`, `path_provider`). Package source SHALL NOT reference `Clipboard` or `SemanticsService`. Exported widgets SHALL receive data through constructor parameters or a package-defined inherited scope, and SHALL emit user intent through callbacks.

Package source SHALL NOT branch on `defaultTargetPlatform` or `Theme.of(context).platform` itself. The `.adaptive` constructors of `material_ui` widgets (for example `CircularProgressIndicator.adaptive`, used today at `streaming_timeline_entry.dart:221,304,420` and `turn_progress_card.dart:101`) are permitted. They branch inside the allowlisted `material_ui` dependency, and keeping them preserves today's Cupertino spinner on Apple platforms.

`flutter_smooth_markdown` 0.8.1 depends transitively on `cached_network_image` and `flutter_svg`, and its default `ImageBuilder` fetches `http(s)` images. The package SHALL NOT reach that path on its own: every `SmoothMarkdown` and `StreamMarkdown` it builds SHALL pass an `imageBuilder`, taken from `ChatMarkdownScope` when one is supplied and otherwise a package placeholder that does no I/O.

To add a dependency, a change SHALL carry a MODIFIED delta of this requirement naming the new allowlist and showing that the dependency only renders. Otherwise its adapter SHALL stay in `app/lib`.

The deprecated `package:flutter/material.dart` import that `flutter_smooth_markdown` needs (`MarkdownStyleSheet.fromTheme`) SHALL be confined to the single file `lib/src/theme/legacy_theme.dart`. Only files under `lib/src/highlight/` SHALL import `package:highlight/`.

#### Scenario: Manifest matches the closed allowlist

- **WHEN** `app/packages/assistant_ui/pubspec.yaml` is inspected
- **THEN** its `dependencies` SHALL contain exactly `flutter`, `flutter_smooth_markdown`, `material_ui` and `highlight`

#### Scenario: Source imports no forbidden library

- **WHEN** every Dart file under `app/packages/assistant_ui/lib/` is scanned for `import` and `export` directives and for `Clipboard.` and `SemanticsService.` references
- **THEN** none SHALL reference a forbidden library or API listed in this requirement
- **AND** `package:flutter/material.dart` SHALL appear only in `lib/src/theme/legacy_theme.dart`
- **AND** `package:highlight/` SHALL appear only under `lib/src/highlight/`

#### Scenario: Platform branching stays inside material_ui

- **WHEN** package source is scanned
- **THEN** it SHALL contain no `defaultTargetPlatform`, `.platform ==` or `TargetPlatform.` reference
- **AND** `.adaptive` constructors of `material_ui` widgets SHALL be allowed

#### Scenario: Markdown image without a scope fetches nothing

- **GIVEN** a `MessageBody` with content `![logo](https://example.com/logo.png)` and no `ChatMarkdownScope` ancestor
- **WHEN** it renders
- **THEN** a placeholder showing the alt text `logo` SHALL render
- **AND** no `CachedNetworkImage`, `Image.network` or `SvgPicture.network` widget SHALL be in the tree

#### Scenario: Elements render without a ProviderScope

- **GIVEN** a widget test that pumps any element in `kAssistantUiElements` with literal arguments
- **WHEN** no `ProviderScope` ancestor exists
- **THEN** the element SHALL build without error

#### Scenario: Turn progress card takes its status by parameter

- **GIVEN** `TurnProgressCard` exported from `assistant_ui`
- **WHEN** it is pumped with `status: null`
- **THEN** it SHALL render nothing
- **WHEN** it is pumped with a status older than `kTurnStallThreshold` relative to an injected `now`, and `showSkip: true`
- **THEN** activating the skip affordance SHALL invoke `onSkip` and SHALL NOT call any API

### Requirement: Package colours and spacing are tokens equal to today's values

The package SHALL define its own colour and spacing tokens in `lib/src/theme/` and SHALL use no raw `Color(0x…)` literal or `Colors.X` name outside that directory, except `Colors.transparent`. Each token SHALL equal the value the moved code renders today, so moving code changes no pixel: `AssistantUiColors.warning` (`0xFFB45309`, today's `AssistantColors.warning`), `AssistantUiColors.toolOk` (`0xFF4CAF50`, today's `Colors.green`), `AssistantUiColors.toolDenied` (`0xFFFFC107`, today's `Colors.amber`) and `AssistantUiColors.fadeMask` (`0xFFFFFFFF`, the fade shader stops at `streaming_timeline_entry.dart:542-543`). `AssistantUiSpacing` SHALL carry the values of today's `AssistantSpacing`. The app's `AssistantColors.warning` and `AssistantSpacing` constants SHALL be defined in terms of the package tokens, so the two cannot drift.

#### Scenario: Tokens equal today's values

- **WHEN** the token test compares `AssistantUiColors.toolOk`, `toolDenied`, `warning` and `fadeMask` with `Colors.green`, `Colors.amber`, `AssistantColors.warning` and `Color(0xFFFFFFFF)`
- **THEN** each pair SHALL have the same ARGB value

#### Scenario: Raw-colour gate covers the package

- **WHEN** `app/scripts/check_no_raw_colors.sh` runs
- **THEN** it SHALL scan `packages/assistant_ui/lib/` and exempt only `packages/assistant_ui/lib/src/theme/`

### Requirement: Platform-adaptive and I/O-backed parts are injected by the app

When an element needs a platform-adaptive primitive or an I/O-backed widget, it SHALL expose a slot or builder for it and SHALL ship a neutral Material default so the gallery and goldens work standalone. The app SHALL supply the façade (`app/lib/shared/platform/`) or I/O implementation.

#### Scenario: Composer text field is injected

- **GIVEN** a `Composer` built with an `inputBuilder`
- **WHEN** it renders
- **THEN** the widget returned by `inputBuilder` SHALL be used as the text input, receiving the composer's controller, focus node, placeholder and submit callback

#### Scenario: Default input renders without the façade

- **GIVEN** a `Composer` built without an `inputBuilder`
- **WHEN** it renders in a test with no app imports
- **THEN** a Material text input SHALL render

#### Scenario: Bubble media is supplied by slot

- **GIVEN** a `MessageBubble` with `attachments` and `audio` slot widgets
- **WHEN** it renders
- **THEN** both slot widgets SHALL appear inside the bubble AND the bubble SHALL NOT itself fetch, play, share or download anything

### Requirement: Render models live in the package unchanged

`ChatMessage`, `ChatAttachment`, `ToolCallRecord`, `MessageStatus`, `ToolCallStatus`, `TimelineEntryType`, `PendingAttachment`, `PendingMessage`, `CommandEntry`, `CommandArg`, `TurnEventKind`, `TurnStatusSnapshot`, `TimelineDensity` and `EntryState` SHALL be defined in `assistant_ui` and imported by the app. Their fields, enum values and mutability SHALL be unchanged by the move. This includes any value added before the move, such as `MessageStatus.stopped` and `ToolCallStatus.cancelled` from chat-stop-cancels-turn. `ChatState` and `ChatToolResult` SHALL remain in the app.

#### Scenario: Provider uses package models

- **WHEN** `app/lib/features/chat/chat_provider.dart` and `chat_models.dart` are inspected after the move
- **THEN** neither SHALL declare any of the moved types AND both SHALL import them from `package:assistant_ui/models.dart`

#### Scenario: Streaming still mutates in place

- **GIVEN** a `ChatMessage` from the package with `isStreaming: true`
- **WHEN** the notifier assigns `content`, `isStreaming`, `isStale` and `status` in place
- **THEN** the assignments SHALL compile and take effect, as before the move

### Requirement: Timeline entry shell owns presentation state; bodies are stateless

`TimelineEntryShell` SHALL be the single owner of per-entry presentation state: expansion, user pinning, auto-collapse timing, reduced-motion handling, stale opacity, and the reaction to `EntryState`. It SHALL take an optional `footer` widget and SHALL render it below the collapsible region, outside it, so the footer stays visible when the entry is collapsed. Entry bodies (`ThinkingBody`, `ToolCallBody`, `SubagentBody`, `CommandBody`) SHALL be `StatelessWidget`s that render only the fields of their own `TimelineEntryType`. Bodies SHALL have no footer of their own. `TimelineEntryView` SHALL forward its `footer` argument to the shell.

`EntryState` SHALL have four values: `active`, `complete`, `stale` and `awaitingInput`. An entry in `awaitingInput` SHALL be expanded, SHALL NOT auto-collapse, SHALL NOT collapse on tap while it remains in that state, and SHALL show an attention accent. Leaving `awaitingInput` SHALL follow the scenarios below. Today's transitions out of `active` (`streaming_timeline_entry.dart:86-104`) SHALL be unchanged.

#### Scenario: Complete entry auto-collapses

- **GIVEN** an unpinned shell at `TimelineDensity.normal` in `EntryState.active`
- **WHEN** its state becomes `EntryState.complete`
- **THEN** the shell SHALL collapse after its auto-collapse delay

#### Scenario: User pinning survives state transitions

- **GIVEN** a shell the user has expanded by tapping
- **WHEN** its state becomes `complete` or `stale`
- **THEN** the shell SHALL remain expanded

#### Scenario: Reduced motion collapses without a timer

- **GIVEN** `MediaQuery.disableAnimations` is true
- **WHEN** an unpinned entry goes from `active` to `complete`
- **THEN** the shell SHALL collapse in the same frame without scheduling a timer

#### Scenario: Awaiting-input entry stays open

- **GIVEN** a collapsed shell at `TimelineDensity.compact`
- **WHEN** its state becomes `EntryState.awaitingInput`
- **THEN** the shell SHALL expand and cancel any pending auto-collapse timer
- **AND** tapping its header SHALL NOT collapse it while the state remains `awaitingInput`

#### Scenario: Awaiting input to complete schedules auto-collapse

- **GIVEN** an unpinned shell in `awaitingInput`
- **WHEN** its state becomes `complete`
- **THEN** the shell SHALL schedule auto-collapse exactly as for `active` to `complete`, including the reduced-motion rule
- **GIVEN** a pinned shell in `awaitingInput`
- **WHEN** its state becomes `complete`
- **THEN** the shell SHALL remain expanded

#### Scenario: Awaiting input to active stays expanded

- **GIVEN** a shell in `awaitingInput` (an approved tool that now runs)
- **WHEN** its state becomes `active`
- **THEN** the shell SHALL remain expanded, at every density, AND SHALL NOT schedule a collapse

#### Scenario: Awaiting input to stale collapses unless pinned

- **GIVEN** an unpinned shell in `awaitingInput`
- **WHEN** its state becomes `stale`
- **THEN** the shell SHALL collapse immediately and render at stale opacity
- **GIVEN** a pinned shell in `awaitingInput`
- **WHEN** its state becomes `stale`
- **THEN** the shell SHALL remain expanded at stale opacity

#### Scenario: A pin made before awaiting input survives the round trip

- **GIVEN** a shell the user pinned open while `active`
- **WHEN** its state goes to `awaitingInput` and then to `complete`
- **THEN** the shell SHALL remain expanded throughout AND SHALL NOT schedule a collapse

#### Scenario: Footer is visible while collapsed

- **GIVEN** a collapsed `TimelineEntryShell` with a `footer` containing a button
- **WHEN** it renders
- **THEN** the button SHALL be visible and tappable
- **AND** the body SHALL NOT be in the tree

#### Scenario: Bodies hold no presentation state

- **WHEN** any entry body is inspected
- **THEN** it SHALL be a `StatelessWidget` AND SHALL NOT reference timers, expansion or `MediaQuery.disableAnimations`

### Requirement: Ambient configuration is supplied by inherited scopes

`ThreadDensityScope` (a `TimelineDensity`), `ChatMarkdownScope` (markdown parser plugins, builder registry, `imageBuilder`, style overrides, an `onTapLink` callback and an `onCopyCode` callback) and `ToolResultRendererScope` (a single `ToolResultBuilder` resolver function) SHALL be the only inherited scopes in the package. Each SHALL have a default when no ancestor exists: `TimelineDensity.normal`; no plugins, the no-I/O image placeholder, links ignored and no code-copy callback (so `CodeBlock` hides its copy button); and no resolver, respectively.

#### Scenario: Density resolves from the scope

- **GIVEN** a `ThreadDensityScope` providing `compact`
- **WHEN** a density-dependent descendant builds
- **THEN** it SHALL render its compact presentation
- **WHEN** the scope is rebuilt providing `expanded`
- **THEN** that descendant SHALL rebuild with its expanded presentation

#### Scenario: Markdown plugins come from the app

- **GIVEN** a `ChatMarkdownScope` whose builder registry maps the `mermaid` block to a test builder
- **WHEN** a `MessageBody` renders content containing a mermaid block
- **THEN** the test builder SHALL render that block

#### Scenario: Image rendering comes from the app

- **GIVEN** a `ChatMarkdownScope` with an `imageBuilder` returning a test widget
- **WHEN** a `MessageBody` renders a markdown image
- **THEN** the test widget SHALL render in its place

#### Scenario: Link taps are delegated

- **GIVEN** a `ChatMarkdownScope` with an `onTapLink` callback
- **WHEN** a link in a `MessageBody`, or in an active streaming `ThinkingBody`, is tapped
- **THEN** the callback SHALL receive the URL AND the package SHALL NOT launch it

#### Scenario: Code copy is delegated

- **GIVEN** a `ChatMarkdownScope` with an `onCopyCode` callback
- **WHEN** the copy button of a `CodeBlock` inside a `MessageBody`, a `ThinkingBody` or a tool result renderer is activated
- **THEN** the callback SHALL receive the code AND the package SHALL NOT write the clipboard

### Requirement: Tool call body renders through a resolver scope with a generic fallback

`ToolCallBody` SHALL call the `resolve` function of the nearest `ToolResultRendererScope` with the record. If it returns a non-null widget, that widget SHALL render the tool details. The package SHALL NOT define how resolution works (tool-name override, data kinds, parsing); the scope's provider owns that. Otherwise `GenericToolResultView` SHALL render the arguments as indented JSON and the result as monospace text, as today. Status icons SHALL follow the `tool-call-display` capability (spinner, check, cross, prohibition).

Tool calls inside `SubagentBody`'s inner timeline SHALL keep today's compact row (icon, tool name, status icon; `streaming_timeline_entry.dart:470-499`) at every density, SHALL NOT expand, and SHALL NOT consult the resolver. Rich tool renderers do not appear inside subagent timelines. Subagent tool calls are not given an opt-in by this change or by tool-result-renderers.

#### Scenario: Resolver result wins

- **GIVEN** a scope whose resolver returns `Text('custom')` for `web_search` records
- **WHEN** a `ToolCallBody` for a `web_search` record renders
- **THEN** `custom` SHALL be visible AND the generic JSON view SHALL NOT be rendered

#### Scenario: Null result falls back

- **GIVEN** a scope whose resolver returns `null` for a `bash` record
- **WHEN** the body renders
- **THEN** the generic view SHALL render the arguments and result

#### Scenario: No scope renders the generic view

- **WHEN** a `ToolCallBody` renders without any `ToolResultRendererScope` ancestor
- **THEN** the generic view SHALL render

#### Scenario: Subagent tool calls stay compact

- **GIVEN** a scope whose resolver returns a custom widget for every record
- **WHEN** a `SubagentBody` with a `file_read` call in `subagentToolCalls` renders expanded
- **THEN** that call SHALL render as one row of icon, `file_read` and status icon
- **AND** the resolver SHALL NOT be called for it

### Requirement: Thinking body renders one collapsible reasoning section

`ThinkingBody` SHALL render streamed reasoning from `thinkingTokenStream` through `StreamMarkdown` while active, with `onTapLink` from `ChatMarkdownScope` and no parser plugins or custom builders, as today (`streaming_timeline_entry.dart:182-198`). Otherwise it SHALL render the accumulated `thinkingContent` as plain selectable text, not markdown (`:199-213`). It SHALL accept an optional `elapsed` duration and SHALL show it only when it is non-null. It SHALL NOT infer step boundaries from token arrival. A per-step timeline stays deferred behind P5 in `ELEMENTS.md`.

#### Scenario: Active reasoning streams

- **GIVEN** an active thinking entry with a token stream
- **WHEN** chunks arrive
- **THEN** the body SHALL render them in one markdown body labelled "Thinking..."
- **AND** a `mermaid` builder registered in `ChatMarkdownScope` SHALL NOT be applied

#### Scenario: Completed reasoning renders as plain text

- **GIVEN** a complete thinking entry whose content contains `**bold**`
- **WHEN** it renders expanded
- **THEN** the literal text `**bold**` SHALL be visible in a selectable text widget

#### Scenario: Empty reasoning renders no expandable content

- **GIVEN** a thinking entry with null or empty content and no stream
- **WHEN** it renders
- **THEN** the shell SHALL NOT be expandable

#### Scenario: Elapsed shows only when supplied

- **WHEN** a complete thinking entry renders with `elapsed: Duration(seconds: 4)`
- **THEN** its label SHALL read "Thought for 4s"
- **WHEN** it renders with `elapsed: null`
- **THEN** its label SHALL read "Thought", as today

#### Scenario: No invented steps

- **GIVEN** thinking content that arrived as several chunks
- **WHEN** the body renders
- **THEN** it SHALL render one continuous body with no per-chunk divisions

### Requirement: Message bubble and message actions expose slots

`MessageBubble` SHALL render a message's alignment, bubble decoration, grouping, failed indicator, typing indicator and content. It SHALL accept these slots: `attachments`, `audio`, `extensions` (a list rendered inside the bubble below the content), and `actions` (rendered below the bubble). Slot positions SHALL follow the role, as today (`message_bubble.dart:87-183`):

- **User messages:** `attachments` render above the content. When `audio` is supplied (a voice message), it replaces the plain-text content row; the transcript stays the caller's concern. User content renders as plain selectable text, not markdown.
- **Assistant messages:** the content renders as markdown through `MessageBody`, and `attachments` and then `audio` render below it.

`MessageActions` SHALL lay out a caller-supplied list of action widgets, an optional `leading` widget, and a `trailing` list of widgets, in that order. It SHALL NOT decide which actions apply to a message, and SHALL render nothing when all three are empty. `MessageActionButton` SHALL be the standard action primitive, with an icon, a label shown as text and used as the semantics label, a `tone` (`muted`, the default, or `danger`, which uses `colorScheme.error` as the retry action does today), a `selected` state, and an `onPressed` callback.

#### Scenario: Caller decides the actions

- **GIVEN** `MessageActions` with two `MessageActionButton`s, a `leading` widget and `trailing: [T1, T2]`
- **WHEN** it renders
- **THEN** the order SHALL be leading, the two buttons in the order given, T1, T2

#### Scenario: Empty actions render nothing

- **GIVEN** `MessageActions` with no actions, no `leading` and an empty `trailing`
- **WHEN** it renders
- **THEN** it SHALL occupy no space

#### Scenario: Selected state is exposed

- **GIVEN** a `MessageActionButton` with `selected: true`
- **WHEN** it renders
- **THEN** its semantics SHALL report it as selected AND it SHALL use the selected colour

#### Scenario: Danger tone uses the error colour

- **GIVEN** a `MessageActionButton` with `tone: danger`
- **WHEN** it renders
- **THEN** its icon and label SHALL use `colorScheme.error`

#### Scenario: Extensions render inside the bubble

- **GIVEN** a `MessageBubble` for an assistant message with one `extensions` widget
- **WHEN** it renders
- **THEN** the widget SHALL appear inside the bubble decoration, below the markdown content

#### Scenario: User attachments render above the text

- **GIVEN** a user `MessageBubble` with text content and an `attachments` widget
- **WHEN** it renders
- **THEN** the attachments widget SHALL appear above the text

#### Scenario: User voice message replaces the text row

- **GIVEN** a user `MessageBubble` with an `audio` widget
- **WHEN** it renders
- **THEN** the audio widget SHALL render AND the content SHALL NOT render as a separate text row

#### Scenario: Assistant attachments render below the content

- **GIVEN** an assistant `MessageBubble` with markdown content and an `attachments` widget
- **WHEN** it renders
- **THEN** the attachments widget SHALL appear below the content

#### Scenario: Existing keys are preserved

- **WHEN** the app builds today's actions through `MessageActions`
- **THEN** each SHALL keep its existing widget `Key`: `stop_action`, `read_aloud_action`, `copy_action`, `share_action`, `save_action` and `retry_button`

#### Scenario: Stopped message keeps its label and Copy

- **GIVEN** an assistant message with status `MessageStatus.stopped`, whether it was just stopped or reloaded from history (chat-stop-cancels-turn persists the finish reason)
- **WHEN** the app builds its actions through `buildMessageActions`
- **THEN** a `stopped_label` reading "Stopped" SHALL replace the Stop action AND the Copy action SHALL remain

### Requirement: Thread layout exposes banner and above-composer slots

`ChatThreadLayout` SHALL arrange, top to bottom: a `banners` list, the `thread`, an `aboveComposer` list and the `composer`. It SHALL render each list in the order given and SHALL render nothing for an empty list. `ChatScreen` SHALL pass `banners: [error banner, history-loading indicator]` and `aboveComposer: [ReconnectBanner, TurnProgressCard]`, each item only when it renders today, so the vertical order of today's screen (`chat_screen.dart:416-459,578-583`) is unchanged. `ReconnectBanner` SHALL keep its current position, the first item of `aboveComposer`, directly above `TurnProgressCard` and the composer; this change and its siblings SHALL NOT move it. Siblings SHALL append their above-composer items after `TurnProgressCard` unless their own spec states another position.

#### Scenario: Above-composer items keep order

- **GIVEN** `aboveComposer: [A, B]`
- **WHEN** the layout renders
- **THEN** A SHALL appear above B AND both SHALL appear directly above the composer

#### Scenario: Reconnect banner stays above the composer

- **GIVEN** a reconnect in progress and no active turn
- **WHEN** the chat screen renders
- **THEN** `ReconnectBanner` SHALL render below the thread and directly above the composer, and SHALL NOT render at the top of the screen

#### Scenario: Chat screen keeps today's vertical order

- **GIVEN** a chat screen with an error, history loading, a reconnect in progress and an active turn
- **WHEN** it renders
- **THEN** from top to bottom the order SHALL be: error banner, history `LinearProgressIndicator`, thread, `ReconnectBanner`, `TurnProgressCard`, composer

### Requirement: Thread viewport anchors scrolling as specified by chat-auto-scroll

`ThreadViewport` SHALL implement the behaviour of the `chat-auto-scroll` capability: an 80 dp bottom threshold, auto-scroll on new content only when at the bottom, the `scroll_to_bottom_button` when scrolled away, and a scroll to the bottom when a conversation is opened or switched. It SHALL take an explicit `revision` value instead of watching application state, and a `ThreadViewportController` exposing `scrollToBottom()` and `scrollToBottomIfNear()`. When `revision` changes while the viewport is at the bottom, it SHALL scroll to the bottom. `ChatScreen` SHALL derive `revision` from a counter it increments on every `chatProvider` emission, which matches today's `ref.listen(chatProvider)` hook (`chat_screen.dart:305-309`), and SHALL call `scrollToBottom()` where it does today after a conversation loads (`chat_screen.dart:110,226,231`).

#### Scenario: At bottom, new content stays visible

- **GIVEN** the viewport is within 80 dp of the bottom
- **WHEN** it rebuilds with a higher `revision` and more items
- **THEN** it SHALL animate to the new bottom

#### Scenario: Thinking streams while at bottom

- **GIVEN** the viewport is at the bottom and the last item is a thinking entry
- **WHEN** `thinkingContent` grows and the provider emits, with the message count and the last message's `content` unchanged
- **THEN** `revision` SHALL change AND the viewport SHALL animate to the new bottom

#### Scenario: Subagent update while at bottom

- **GIVEN** the viewport is at the bottom and the last item is a subagent entry
- **WHEN** a tool call is appended to its `subagentToolCalls` in place, or a tool status flips in place
- **THEN** the viewport SHALL animate to the new bottom

#### Scenario: Queued message added while at bottom

- **GIVEN** the viewport is at the bottom
- **WHEN** a message is added to the pending queue, which grows `itemCount` without changing `messages`
- **THEN** the viewport SHALL animate to the new bottom

#### Scenario: Conversation switch scrolls to bottom

- **GIVEN** the user is scrolled away in conversation A
- **WHEN** the user opens conversation B, which has messages
- **THEN** the viewport SHALL animate to the bottom of B after layout AND `scroll_to_bottom_button` SHALL hide

#### Scenario: Scrolled away, the viewport does not jump

- **GIVEN** the user has scrolled more than 80 dp above the bottom
- **WHEN** `revision` changes
- **THEN** the scroll offset SHALL NOT change AND `scroll_to_bottom_button` SHALL be visible

#### Scenario: Button returns the user

- **WHEN** `scroll_to_bottom_button` is activated
- **THEN** the viewport SHALL animate to the bottom AND the button SHALL hide

### Requirement: Empty state accepts starter content

`ThreadEmptyState` SHALL render a title and a subtitle, defaulting to today's "Start a conversation" / "Type a message below to begin.", and an optional `starters` widget below them.

#### Scenario: Default copy

- **WHEN** `ThreadEmptyState` renders with no arguments
- **THEN** "Start a conversation" and "Type a message below to begin." SHALL be visible

#### Scenario: Starters slot

- **GIVEN** a `starters` widget
- **WHEN** the empty state renders
- **THEN** the widget SHALL appear below the subtitle

### Requirement: Composer emits intent by callback and performs no I/O

`Composer` SHALL render the input row: the injected input, send and stop affordances, attach affordance, an optional `voiceButton` slot, the queue-depth badge, `AttachmentTray` and `SlashCommandMenu`. It SHALL take an `isStopping` input; while it is true the stop affordance SHALL be disabled with the semantics label "Stopping…". It SHALL emit every intent as a callback. It SHALL NOT record audio, pick files, read the clipboard or the file system, or call any API. `ComposerVoiceButton` SHALL be the presentation half of the voice affordance, driven by `isRecording` and `elapsed`.

#### Scenario: Send emits a callback

- **GIVEN** a `Composer` with input text
- **WHEN** `send_button` is activated
- **THEN** `onSend` SHALL fire

#### Scenario: Stop replaces send while sending

- **GIVEN** `isSending: true`
- **WHEN** the composer renders
- **THEN** `stop_button` SHALL be visible instead of `send_button` AND activating it SHALL fire `onStop`

#### Scenario: Stop is disabled while stopping

- **GIVEN** `isSending: true` and `isStopping: true`
- **WHEN** the composer renders
- **THEN** `stop_button` SHALL be disabled with the semantics label "Stopping…" AND activating it SHALL NOT fire `onStop`

#### Scenario: Attachment removal is delegated

- **GIVEN** an `AttachmentTray` with two pending attachments
- **WHEN** `remove_attachment_0` is activated
- **THEN** `onRemove` SHALL fire with index 0 AND the tray SHALL NOT change its own list

#### Scenario: Slash-command menu filters a supplied list

- **GIVEN** a `SlashCommandMenu` with a list of commands and a filter string
- **WHEN** it renders
- **THEN** only matching commands SHALL show AND selecting one SHALL fire `onSelect` without dispatching it

#### Scenario: Voice button is presentation only

- **WHEN** `ComposerVoiceButton` with `isRecording: false` is activated
- **THEN** `onStart` SHALL fire
- **WHEN** it renders with `isRecording: true` and an elapsed duration
- **THEN** the elapsed time and a stop affordance SHALL render

### Requirement: Conversation list view is pure

`ConversationListView` SHALL render a list of package `ConversationListItem`s (id, title, updatedAt), an optional `header`, and an optional `trailingBuilder` per item. Relative dates SHALL be computed against an injected `now` (defaulting to `DateTime.now()`). It SHALL emit `onSelect`. Deletion SHALL go through a single `Future<bool> Function(String id) onDelete` that the caller implements as confirm-then-delete; the row SHALL be dismissed only when it resolves to `true`, so a declined confirmation or a failed delete leaves the row in place, as `conversation_list.dart:123-140` does today. The view SHALL NOT highlight any row as selected and SHALL expose no selection parameter: the sidebar looks as it does today. conversation-list-management adds a `selectedId` parameter in its own change and is the one that turns the highlight on.

#### Scenario: No row is highlighted

- **GIVEN** a list of items, one of which is the conversation currently open in the chat pane
- **WHEN** the view renders
- **THEN** every row SHALL render with the same, unhighlighted style (images unchanged from the characterization goldens)
- **AND** the view SHALL declare no `selectedId` or equivalent parameter

#### Scenario: Selection is delegated

- **WHEN** an item is tapped
- **THEN** `onSelect` SHALL fire with its id AND the view SHALL NOT navigate

#### Scenario: Declined delete keeps the item

- **GIVEN** `onDelete` resolves to `false` because the user declined
- **WHEN** the user swipes an item to delete
- **THEN** the item SHALL remain

#### Scenario: Failed delete keeps the item

- **GIVEN** `onDelete` resolves to `false` because the delete call failed
- **WHEN** the user swipes an item to delete
- **THEN** the item SHALL remain AND no dismissed `Dismissible` SHALL stay in the tree

#### Scenario: Successful delete dismisses the item

- **GIVEN** `onDelete` resolves to `true`
- **WHEN** the user swipes an item to delete
- **THEN** the row SHALL be dismissed

#### Scenario: Empty list

- **GIVEN** no items
- **WHEN** the view renders
- **THEN** the `header` SHALL render AND no item rows SHALL render

### Requirement: Gallery and goldens cover every element

The package SHALL expose `kAssistantUiElements`, an explicit registry naming every exported element. `app/widgetbook/` SHALL provide at least one use-case per registry entry. For timeline entries it SHALL expose every `EntryState` × `TimelineDensity` combination. Every registry entry SHALL have at least one golden test tagged `golden`. Golden tests SHALL inject a fixed clock wherever the element shows time, and SHALL advance animations with `pump(Duration)`, never `pumpAndSettle`. The gallery SHALL NOT be part of the shipped web build.

#### Scenario: Registry matches exports

- **WHEN** the export directives of `assistant_ui.dart` are compared with `kAssistantUiElements`
- **THEN** every exported widget SHALL appear in the registry

#### Scenario: Every registered element has a use-case and a golden

- **WHEN** the registry is compared with the gallery's use-cases and the package's golden tests
- **THEN** every entry SHALL have at least one of each

#### Scenario: Timeline matrix is selectable

- **WHEN** the gallery entry for `TimelineEntryShell` is opened
- **THEN** each of the 12 combinations of the four `EntryState` values and the three `TimelineDensity` values SHALL be selectable

#### Scenario: Goldens are deterministic

- **WHEN** a golden of an element that shows relative time, elapsed time or a repeating animation runs twice
- **THEN** both runs SHALL produce identical images

#### Scenario: Gallery is not shipped

- **WHEN** `make build-web` produces `app/build/web/`
- **THEN** no widgetbook code SHALL be included

### Requirement: Chat screen is a thin adapter

`ChatScreen` SHALL keep provider reads, routing, navigation chrome, layout breakpoints and I/O (`file_picker`, `desktop_drop`, `record`, audio, link launching, image loading). It SHALL delegate conversation-surface rendering to `assistant_ui` elements. `app/lib/features/chat/` SHALL NOT declare widgets whose names equal an exported element; app-side adapters SHALL carry distinct names (for example `ChatMessageBubbleAdapter`, `ConversationListPane`).

#### Scenario: Behaviour is preserved

- **GIVEN** the chat widget tests that exist before this change
- **WHEN** they run against the extracted elements with imports updated
- **THEN** every previously passing assertion SHALL still pass

#### Scenario: Characterization goldens are reproduced

- **GIVEN** the characterization goldens captured from the app widgets before extraction
- **WHEN** each extraction PR renders the same inputs through the extracted element
- **THEN** the images SHALL match byte for byte

#### Scenario: No visual change in the end-to-end suite

- **WHEN** the Playwright visual suite runs in any PR of this change
- **THEN** the chat baselines current on `main` at that time SHALL match without being updated

### Requirement: Element index records a verdict for every catalogue element

`app/packages/assistant_ui/ELEMENTS.md` SHALL record a verdict for each of the 59 assistant-ui catalogue elements: 37 individual rows plus one category row covering the 22 Generative elements. Each verdict is one of Adopt, Adopt (scoped), Adopt (have), Adopt via <change>, Defer, or Decline. Every Defer SHALL name its prerequisite. Every Decline SHALL give a reason. Every "Adopt via <change>" SHALL name the change and the seam it uses.

#### Scenario: Every element has one verdict

- **WHEN** `ELEMENTS.md` is inspected
- **THEN** it SHALL contain 37 element rows and one 22-element category row, each with exactly one verdict, AND the tally SHALL total 59

#### Scenario: Registry entries are adopted

- **GIVEN** an entry in `kAssistantUiElements` that maps to a catalogue element
- **WHEN** the index is compared with the registry
- **THEN** that element SHALL carry an Adopt, Adopt (scoped) or Adopt (have) verdict

#### Scenario: Adopted rows are registered

- **GIVEN** the change is complete
- **WHEN** every Adopt (have) and Adopt (scoped) row is compared with `kAssistantUiElements`
- **THEN** each element it names SHALL be in the registry
