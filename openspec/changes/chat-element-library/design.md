# Design: chat-element-library

## Context

assistant-ui Elements is a React and Tailwind registry. Nothing ports mechanically. What carries over is the vocabulary (the pieces an assistant conversation is made of) and the gallery model (every element can be shown alone, in every state).

This change is also the foundation for nine sibling changes. Every new chat widget they add lands in this package, so the package has to ship the slots they plug into before they need them. The PR stack in `tasks.md` is ordered with that in mind.

### Verified state of the code (2026-11)

| Fact                                                                                  | Evidence                                                                                                                                                                                                                                                       |
| ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Render models already sit in a Riverpod-free file                                     | `app/lib/features/chat/chat_models.dart` (399 LOC) imports only `dart:typed_data`. `ChatState` (L313) is app state and stays in the app.                                                                                                                       |
| Timeline entries take constructor data, but one `State` holds the whole state machine | `streaming_timeline_entry.dart:47-58` (ctor); `:63-134` (`_expanded`, `_userPinned`, `_collapseTimer`, reduced motion); render branches `:168` thinking, `:231` tool call, `:385` subagent                                                                     |
| Tool args and results render generically                                              | `streaming_timeline_entry.dart:324-382` `_toolDetails`: pretty JSON args and a monospace `SelectableText` result                                                                                                                                               |
| `ChatTimelineSection` and `ToolCallChip` are dead                                     | `grep` finds them only in their own files, in `test/widget/timeline_section_test.dart` and `test/widget/tool_call_chip_test.dart`, and in a doc comment at `streaming_timeline_entry.dart:43`                                                                  |
| Material comes from the `material_ui` package, not the SDK                            | Every chat widget imports `package:material_ui/material_ui.dart` or the façade `shared/platform/widgets.dart`; `app/pubspec.yaml` pins `material_ui: ^1.11.1` and `cupertino_ui: ^1.1.7`                                                                       |
| `flutter_smooth_markdown` still needs the framework `ThemeData`                       | `shared/theme/legacy_theme.dart` imports the deprecated `package:flutter/material.dart` for `MarkdownStyleSheet.fromTheme`                                                                                                                                     |
| Link taps are I/O                                                                     | `markdown_link_handler.dart` imports `url_launcher`                                                                                                                                                                                                            |
| SVG rendering imports `flutter_svg`                                                   | `svg_builder.dart:3`                                                                                                                                                                                                                                           |
| Bubble audio and attachments do I/O                                                   | `audio_player_widget.dart`, `voice_message_player.dart` and `read_aloud_action.dart` import `audioplayers`; `message_attachments.dart` imports `http`, `cached_network_image` and `share_service`                                                              |
| Platform-adaptive widgets live in the app façade                                      | `AdaptiveTextField` (`chat_input_row.dart:261`), `AdaptiveMediaContextMenu` (`message_bubble.dart:96,175`), `AdaptiveTextSelectionToolbar` (`:239`)                                                                                                            |
| Riverpod leaks in render widgets                                                      | `TurnProgressCard` is a `ConsumerStatefulWidget` with no parameters (`turn_progress_card.dart:34-35`) and calls `ref.watch` and `requestCancelTurn` (`:65,135`); `QueuedMessageBubble` has 1 `ref` read; `ConversationList` has 5 (`conversation_list.dart:23,44,49,85,133`) over `ConversationListEntry` items (`lib/api/models/stream_event.dart:65`); `VoiceRecorderButton` has 1 |
| The scroll-to-bottom button already exists                                            | `chat_screen.dart:86-91` `_onScroll` (80 dp threshold), `:561-571` FAB `scroll_to_bottom_button`; specified by `openspec/specs/chat-auto-scroll`                                                                                                               |
| Message actions decide internally from the message                                    | `message_meta_actions.dart:15-160` `MetaActionRow(message, capabilities, fetchMessageAudio, onRetry, onStop)` builds stop, read-aloud, copy, share, save and retry (keys `stop_action`, `read_aloud_action`, `copy_action`, `share_action`, `save_action`, `retry_button`); `MetaActionButton` takes a `color` (retry uses `colorScheme.error`, `:144`); an empty row is `SizedBox.shrink` (`:150`) |
| No golden tests exist yet; CI runs only `app/test`                                    | `grep matchesGoldenFile app/test` is empty; `.github/workflows/flutter.yml:54-58` runs `flutter analyze` and `flutter test` in `app/`                                                                                                                          |
| The Playwright chat baseline shows no conversation                                    | `crates/web-ui/e2e/tests/visual-regression.spec.ts:375-377` screenshots `/chat` with no conversation open: empty state, composer, sidebar. No timeline entry, bubble or action is covered. |
| Subagent tool calls render as compact rows                                            | `streaming_timeline_entry.dart:470-499`: one `Row` of icon, tool name and status icon per call; no args, no result, no expansion |
| The moved code uses raw colours                                                       | `streaming_timeline_entry.dart:309,319,408,414` (`Colors.green`, `Colors.amber`), `:542-543` (`Color(0xFFFFFFFF)` shader stops); the file is on `app/scripts/theme_color_allowlist.txt`. `chat_input_row.dart:125-136` uses `AssistantSpacing` and `AssistantColors.warning` |
| `flutter_smooth_markdown` does network I/O by default                                 | v0.8.1 pubspec lists `cached_network_image: ^3.3.0` and `flutter_svg`; `lib/src/renderer/builders/image_builder.dart:25-62` uses `context.imageBuilder` when set, else `CachedNetworkImage` for `http(s)` URLs. `SmoothMarkdown` and `StreamMarkdown` accept `imageBuilder` |
| Scroll follows every provider emission                                                | `chat_screen.dart:305-309` scrolls on any `chatProvider` emission while at the bottom; thinking tokens and subagent updates mutate in place (`chat_timeline_reducers.dart:175,245`); queued messages are list items outside `messages` (`chat_screen.dart:490`); conversation loads call `_scrollToBottom` (`:110,226,231`) |
| Screen order around the thread                                                        | Top: error banner, then history `LinearProgressIndicator` (`chat_screen.dart:416-459`). Bottom: `ReconnectBanner`, then `TurnProgressCard`, then the input row (`:578-583`) |

## Decision 1: Composition is constructor slots by default, inherited scopes for ambient config

|       | Approach                                                    | Verdict                                                                   |
| ----- | ----------------------------------------------------------- | ------------------------------------------------------------------------- |
| **A** | Constructor slots: `TimelineEntryShell(header: …, body: …)` | Default                                                                   |
| **B** | Inherited scope: `XScope.of(context)`                       | Only for config that is ambient _and_ read at more than one nesting level |
| **C** | Riverpod inside elements                                    | Banned by the package boundary                                            |

Three scopes qualify for B:

- `ThreadDensityScope`: every element at every depth reads density.
- `ChatMarkdownScope`: plugin registry, builder registry and `onTapLink`. `MessageBody`, `ThinkingBody` and the subagent inner timeline all render markdown, and the app owns the plugins that do I/O (`SvgBuilder`, link launching).
- `ToolResultRendererScope`: the resolver is per-thread configuration, like the markdown plugins. `ToolCallBody` is built by `TimelineEntryView`, which the app calls from the list's `itemBuilder`. A constructor parameter would have to be threaded through `TimelineEntryView` and every dispatcher case that siblings add, and changing the dispatcher's signature is what the seam is meant to avoid. tool-result-renderers installs `ToolResultRendererScope(resolve: registry.build)` once in `ChatScreen`. Subagent inner tool calls do **not** consult it: today they are compact rows with no details (`streaming_timeline_entry.dart:470-499`), and they stay that way. This is a settled product decision (Decision 8b), not a deferral.

`EntryState` stays a constructor parameter because it belongs to one entry and is not ambient.

Rejected: a single `AssistantUiScope` carrying everything. Density changes on every resize and would rebuild markdown consumers for nothing. Three small scopes also keep each dependency visible.

## Decision 2: The allowlist is `flutter`, `flutter_smooth_markdown` and `material_ui`. No platform branching.

The old draft allowed `flutter` and `flutter_smooth_markdown` only. That cannot build: `Theme`, `Icons`, `InkWell` and `CircularProgressIndicator` come from `material_ui` in this Flutter (3.50.1, `app/pubspec.yaml:8`). The three dependencies split as follows:

| Dependency                                                                                                         | Side    | Why                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------ | ------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `flutter`, `material_ui`                                                                                           | package | Pure rendering primitives                                                                                                           |
| `flutter_smooth_markdown`                                                                                          | package | Rendering, with one exception: v0.8.1 depends on `cached_network_image` and `flutter_svg`, and its default `ImageBuilder` fetches `http(s)` images. The package closes that path (below). It also brings `flutter_highlight` 0.7.0, relevant to chat-a11y-and-code-blocks. |
| `cupertino_ui`, `shared/platform/*`                                                                                | **app** | Platform branching is confined to the façade (`openspec/specs/platform-facade-discipline`). The package never branches on platform itself. `material_ui`'s `.adaptive` constructors are allowed (see below). |
| `url_launcher`, `audioplayers`, `record`, `file_picker`, `desktop_drop`, `http`, `cached_network_image`, `dart:io` | app     | I/O                                                                                                                                 |
| `flutter_svg`                                                                                                      | app     | Used only by `SvgBuilder`, which the app registers through `ChatMarkdownScope`. Present transitively through `flutter_smooth_markdown`, but never imported by the package. |

**Transitive I/O in `flutter_smooth_markdown`.** The allowlist bans direct imports, but the markdown package carries network image loading inside it. Every `SmoothMarkdown` and `StreamMarkdown` the package builds therefore passes an `imageBuilder`: `ChatMarkdownScope.imageBuilder` when the app supplies one, otherwise a package placeholder that shows the alt text. The app registers an image builder that keeps today's rendering (`CachedNetworkImage` for raster URLs, `SvgPicture.network` for `.svg`), so pixels in the app do not move, while goldens and the gallery never touch the network. A spec scenario asserts that a markdown image with no scope builds no network image widget.

**`.adaptive` constructors.** `CircularProgressIndicator.adaptive` appears 4 times in the moved code (`streaming_timeline_entry.dart:221,304,420`, `turn_progress_card.dart:101`) and renders a Cupertino spinner on Apple platforms. It branches inside `material_ui` (which depends on `cupertino_ui`), not in package source. We allow it: replacing it with a neutral spinner would change the Apple rendering, and a spinner slot on every body is noise. The boundary test bans `defaultTargetPlatform`, `.platform ==` and `TargetPlatform.` in package source, which is the rule the spec states.

**Injection rule.** Where the package needs something adaptive or I/O-backed, it exposes a slot or builder and ships a neutral Material default for the gallery and goldens:

- `Composer.inputBuilder`: the app passes `AdaptiveTextField`.
- `Composer.voiceButton`: the app passes `VoiceRecorderButton`. The recorder stays in the app, and the package's `ComposerVoiceButton` is the presentation half that the app's recorder drives.
- `MessageBubble.contextMenuBuilder` and `selectionToolbarBuilder`.
- `MessageBubble.attachments` and `audio`: the app passes `MessageAttachments` and the audio players.

The deprecated `package:flutter/material.dart` import that `flutter_smooth_markdown`'s `MarkdownStyleSheet.fromTheme` needs is confined to one file, `lib/src/theme/legacy_theme.dart`, mirroring `app/lib/shared/theme/legacy_theme.dart`.

**Amendment rule (for siblings).** A change that needs another package dependency must:

1. show the dependency is pure rendering (no I/O, no platform channel, no state management);
2. add a MODIFIED delta of the "Element package is pure" requirement listing the new allowlist;
3. update the boundary test in the same PR.

Otherwise the adapter stays in `app/lib`. generative-ui-a2ui's `genui` and any highlighter for chat-a11y-and-code-blocks are expected to take one route or the other in their own specs. `MessageBubble.extensions`, the shell `body` slot and `ChatMarkdownScope` exist so that keeping the adapter in the app is cheap.

Rejected: moving `shared/platform/` into its own package that `assistant_ui` depends on. That is 1,220 LOC across 18 files and every app importer, and it is a separate refactor.

## Decision 3: Extension points are shipped empty, with a generic default

Each sibling gets a named seam with a default that reproduces today's rendering. The seam's first PR changes no pixels.

| Seam                                                                                                                                                                                                                                                                | Default                                                                            | Consumer                                                                                                                                                         |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ToolResultRendererScope(resolve: ToolResultBuilder)`, where `typedef ToolResultBuilder = Widget? Function(BuildContext, ToolCallRecord)` and `null` means "use the fallback". The scope carries one function and knows nothing about registries, kinds or parsing. | `GenericToolResultView` (args JSON, result monospace)                              | tool-result-renderers: its `ToolResultRendererRegistry` (tool-name override, then `data['kind']`, then parse-or-fallback) supplies `registry.build` as `resolve` |
| `TimelineEntryShell.footer` (`Widget?`), rendered below the collapsible region so it stays visible when collapsed. `TimelineEntryView(footer:)` forwards it. Sibling text that says "`ToolCallBody.footer`" means this slot on a tool call entry. | none                                                                               | tool-approval-prompts: `ApprovalCard` in the footer of the matching tool call entry, `QuestionCard` in the footer of the ask-user tool call. `TimelineEntryType.interaction` is only the fallback when the provider gives no `tool_call_id`; tool-approval-prompts adds that `TimelineEntryView` case |
| `EntryState.awaitingInput`: the shell force-expands, never auto-collapses, ignores taps that would collapse, and shows an attention accent. Leaving it: to `complete` schedules auto-collapse unless pinned; to `active` stays expanded; to `stale` collapses unless pinned; an earlier pin survives | n/a (no producer yet)                                                              | tool-approval-prompts, generative-ui-a2ui                                                                                                                        |
| `MessageActions(actions: List<Widget>, leading: Widget?, trailing: List<Widget>)` plus the `MessageActionButton(icon, label, tone, selected, onPressed)` primitive; the app's `buildMessageActions` decides which actions apply | app builds stop, read-aloud, copy, share, save and retry as today                  | message-branching: Edit and Regenerate `MessageActionButton`s from app-side `buildMessageActions`, `BranchPicker` in `leading`, plus a pure `RegenerateMenuButton(models, onSelected)` element. `trailing` is composed by `buildMessageActions` in a fixed order: chat-a11y-and-code-blocks' `MessageTimingIndicator`, then message-feedback's `FeedbackButtons` |
| `MessageBubble.extensions` (`List<Widget>`), rendered inside the bubble below content                                                                                                                                                                               | empty                                                                              | generative-ui-a2ui (inline surfaces), later citations                                                                                                            |
| `ChatThreadLayout(banners, thread, aboveComposer, composer)`                                                                                                                                                                                                        | banners = error banner, history-loading indicator; aboveComposer = `ReconnectBanner`, `TurnProgressCard` (today's order) | follow-up-suggestions (chips, appended after `TurnProgressCard`); available to tool-approval-prompts, which uses the tool call footer instead                     |
| `ThreadEmptyState(title, subtitle, starters: Widget?)`                                                                                                                                                                                                              | today's copy from `empty_chat.dart`                                                | follow-up-suggestions (per-Persona starter prompts)                                                                                                              |
| `ChatMarkdownScope(plugins, builders, imageBuilder, onTapLink, styleSheet)`                                                                                                                                                                                         | Mermaid and SVG plugins as in `message_bubble.dart:213-225` and the network image builder, registered by the app | chat-a11y-and-code-blocks (code-block builder with copy); tool-result-renderers reads `onTapLink` for links in result views                                      |
| `ConversationListView(items, onSelect, onDelete: Future<bool> Function(id), header, trailingBuilder, now)` over a package `ConversationListItem`                                                                                                                    | today's list, no selection highlight and no selection parameter (Decision 8a)      | conversation-list-management (rename, search, pin, and the `selectedId` parameter that turns the highlight on)                                                   |
| `TimelineEntryView(message, entryState, footer)`: a pure dispatcher from `TimelineEntryType` to shell plus body                                                                                                                                                    | covers the existing five types                                                     | siblings add cases when they add entry types (`surface` for generative-ui-a2ui, `interaction` for tool-approval-prompts)                                         |

The siblings own the producers, models, wiring and widgets behind these seams. This change owns the seams and their tests.

**`ToolResultRendererRegistry` contract.** tool-result-renderers gives its registry a `Widget? build(BuildContext, ToolCallRecord)` that reads density from `ThreadDensityScope` and links from `ChatMarkdownScope.onTapLink`; its `resolve` stays internal. `ToolCallBody` has no registry parameter.

Rejected: designing the approval card, the renderer registry, the renderer widgets or `SurfaceFrame` here. generative-ui-a2ui owns `SurfaceFrame` and `TimelineEntryType.surface`; tool-result-renderers owns `ToolResultRendererRegistry`. Their data shapes belong to their changes (the interaction channel event and `ToolCallRecord.data`), and pre-building them would guess at wire formats.

## Decision 4: Move the models unchanged. Do not seal `ChatMessage`.

`ChatMessage` is a union of nullable fields (`chat_models.dart:77-218`). `ChatNotifier` mutates `content`, `isStreaming`, `isStale`, `status` and `subagentContent` in place during streaming. Sealing implies immutability, which means rewriting the 1,112-line provider. The body split delivers most of the benefit anyway, because each body reads only its own fields.

`ChatState` and `ChatToolResult` stay in the app. They are provider state and notification plumbing, not render input.

Siblings add fields to these models (for example `ToolCallRecord.data` and branch metadata). The move happens in PR 4 so that those additions land in the package and not in a file that is about to move.

## Decision 5: The reasoning panel stays scoped

The runtime emits `OrchestratorEvent::Thinking(String)` with no delimiters (`crates/runtime/src/orchestrator/stream_event.rs:70`), and `thinkingContent` is a flat `String?`. A step timeline would invent structure. `ThinkingBody` renders today's collapsible "Thinking… / Thought" section and takes an optional `elapsed`. When `elapsed` is non-null it reads "Thought for 4s". No data source exists yet, so nothing changes visually. Whichever change records thinking timing wires it. P5 still blocks the step timeline.

## Decision 6: `ThreadViewport` preserves `chat-auto-scroll` exactly

The old draft called the recovery pill new UI. It is not: `scroll_to_bottom_button` exists (`chat_screen.dart:565`). `ThreadViewport` packages the existing behaviour (80 dp threshold, pin when at bottom, FAB when scrolled away, scroll on conversation open) and takes an explicit `revision` signal in place of `ref.listen(chatProvider)` (`chat_screen.dart:305-309`).

`revision` must change whenever today's hook would fire. A value derived from the message count and the last message's content length would miss thinking tokens (they grow `thinkingContent`), subagent updates and tool status flips (mutated in place, `chat_timeline_reducers.dart:175,245`), and queued messages (list items outside `messages`, `chat_screen.dart:490`). `ChatScreen` therefore increments an `int` on every `chatProvider` emission and passes it. Alternatives:

| Approach | Verdict |
| -------- | ------- |
| Emission counter in `ChatScreen` | **Chosen.** Reproduces today's trigger exactly; trivial to test |
| Stay pinned whenever `maxScrollExtent` grows (`ScrollMetricsNotification`) | Rejected for now: also fires on layout changes such as an entry auto-collapsing, which today does not scroll; a behaviour change |
| Derived from content | Rejected: misses in-place mutations |

Conversation open and switch use `ThreadViewportController.scrollToBottom()`, called where `_scrollToBottom` is called today (`chat_screen.dart:110,226,231`). The input-focus scroll (`_onInputFocusChange`) becomes `scrollToBottomIfNear()`.

## Decision 6a: `ChatThreadLayout` keeps today's vertical order

Today the error banner and the history `LinearProgressIndicator` sit above the thread, and `ReconnectBanner` and `TurnProgressCard` sit below it, above the input row (`chat_screen.dart:416-459,578-583`). `ChatScreen` passes `banners: [ThreadErrorBanner?, LinearProgressIndicator?]` and `aboveComposer: [ReconnectBanner, TurnProgressCard]`. A layout scenario asserts the full order.

## Decision 7: Tooling

- **Pub workspace.** `app/pubspec.yaml` gains `workspace: [packages/assistant_ui, widgetbook]`, and the members declare `resolution: workspace`. A single `flutter pub get` then resolves everything, and `flutter analyze` from `app/` sees the members without extra steps. Rejected: separate `pub get` per directory in CI and the Makefile. That means three lockfiles that can drift.
- **Gallery.** `app/widgetbook/` depends on `widgetbook: 3.25.0`, the latest on pub.dev (published 2026-06-25, requires Flutter >= 3.44), and on `assistant_ui`. Use-cases are written by hand, without `widgetbook_generator` or build_runner. `flutter build web` in `app/` never compiles it, because it is a separate entry point.
- **Goldens.** Plain `matchesGoldenFile` from `flutter_test` with no extra dependency. Golden tests carry `@Tags(['golden'])`, and `dart_test.yaml` in the package declares the tag. CI (ubuntu) runs them. `make test-flutter` passes `--exclude-tags golden` unless `CI` is set or the host is Linux, so local runs on macOS stay green; `make update-goldens` runs in the Flutter Docker image pinned to `app/pubspec.yaml`'s Flutter version. The harness takes an injected clock (`now`) for anything that shows relative or elapsed time (`conversation_list.dart:166-173`, `turn_progress_card.dart:42-59`), and golden tests advance animations with `pump(Duration)`, never `pumpAndSettle` (`typing_dots.dart` repeats forever).
- **Characterization goldens first.** The Playwright chat baseline covers only the empty `/chat` page. Before any extraction, PR 1b captures goldens of the current app widgets (`StreamingTimelineEntry` per type, state and density; `MessageBubble` variants; `ChatInputRow`; `TurnProgressCard`; `ConversationList`) under `app/test/golden/characterization/`. Each extraction PR renders the same inputs through the new element and must match those images byte for byte before the app widget is deleted; the characterization test then points at the element. This is the before/after comparison for the 671-LOC state machine and the bubble. Rejected: `alchemist` 0.15.0. Its CI/platform golden split is nice, but it is a dependency the plain approach does not need.
- **Gates.** `check_no_raw_colors.sh` extends its scan to `packages/assistant_ui/lib/`, exempting `lib/src/theme/`. The moved code uses raw colours today (`streaming_timeline_entry.dart` is allowlisted), so the package defines tokens whose values equal what renders now: `AssistantUiColors.warning` (`0xFFB45309`, today's `AssistantColors.warning`, used by `chat_input_row.dart:128`), `toolOk` (`0xFF4CAF50` = `Colors.green`), `toolDenied` (`0xFFFFC107` = `Colors.amber`) and `fadeMask` (`0xFFFFFFFF`). Swapping `Colors.amber` for `warning` would move pixels, so we do not. `AssistantUiSpacing` copies `AssistantSpacing`. The app's `AssistantColors.warning` and `AssistantSpacing` are defined in terms of the package tokens so the values cannot drift. Each moved file leaves the allowlist when it is deleted from the app.
- **ADR.** The package boundary, the pub workspace, the allowlist amendment rule and the composition rule bind nine sibling changes, so they are recorded in `docs/adr/adr-0021-assistant-ui-element-package.md` (0017–0019 are taken by tool-approval-prompts, message-branching and generative-ui-a2ui, and 0020 by tool-result-renderers).

## Decision 8: Settled product decisions

### 8a. No selection highlight in `ConversationListView`; conversation-list-management turns it on

The list ships with no highlight and no `selectedId` parameter. This change is a behaviour-preserving extraction guarded by byte-for-byte goldens, and today's list has no selection style, so any highlight here would move pixels in the PR that is meant to move none. conversation-list-management already owns row actions, pinning and search, which restyle the rows anyway, and it adds `selectedId` in the same PR that decides what the highlight looks like (its design lists `selectedId` as its own parameter). Rejected: shipping an unused `selectedId` here. It would be a seam with no producer and no test of its look.

### 8b. Subagent tool calls stay compact rows

`SubagentBody` keeps one compact row per inner tool call (icon, name, status icon) at every density, does not expand them, and never calls the `ToolResultRendererScope` resolver. Subagent timelines are long, nested, and read as progress, not as results, and the compact row is what ships today (`streaming_timeline_entry.dart:470-499`). Rich renderers there would also need per-row expansion state, nested scroll handling and a decision about which records carry `data`. Rejected: letting subagent rows consult the resolver, as `ToolResultRendererScope`'s `InheritedWidget` shape would permit. The spec and tests (task 7b.1) fix the compact behaviour. tool-result-renderers must not rely on the scope reaching subagent rows.

### 8c. `ReconnectBanner` position is unchanged

The banner stays the first item of `aboveComposer`, directly above `TurnProgressCard` and the composer. Moving it would change the layout the characterization goldens and the Playwright baselines guard, and nothing in the sibling changes needs it elsewhere; interaction cards and suggestion chips append after `TurnProgressCard`. The layout test (task 10.3) asserts the order and a separate case asserts the banner sits below the thread.

### 8d. Decisions from sibling changes that touch these seams

- **Stopped state (chat-stop-cancels-turn).** The "Stopped" label survives reload because the server persists a `finish_reason` on messages and the client maps it to `MessageStatus.stopped`. The label belongs to the app's `buildMessageActions` (`stopped_label` replaces the Stop action, Copy stays), so PR 8a must carry it over from the `MetaActionRow` that chat-stop PR 4 edits first. A user message cancelled before a worker claimed it renders as failed with Retry, which `MessageBubble`'s failed indicator and the `retry_button` action already provide. Pending tool calls become `cancelled` with the muted block icon, which PR 4 and PR 6 carry.
- **Approval rollout (tool-approval-prompts).** The stack is reordered so the PR that turns interactive approvals on for web turns lands after the client `ApprovalCard` and `QuestionCard` exist, with no feature flag. For this change that means PR 5b (`awaitingInput`, `footer`) and PR 6 (`TimelineEntryView.footer` forwarding on tool call entries) must merge before tool-approval-prompts PR 16 and before that switch; this is already their order in the unblock map.
- **Branching with a pending approval (message-branching).** Branching auto-cancels the pending approval, so the entry leaves `awaitingInput` through the existing transitions (to `complete` or `stale`, with the pin rules above). No new shell state is needed.
- **Compact density (tool-result-renderers).** Tap-to-expand at the compact (phone) density is implemented inside the widget the resolver returns (summary line that expands in place). The shell's own tap still only shows or hides the body, and `ToolResultBuilder` keeps its signature.
- **ADR number.** tool-result-renderers takes 0020, so this change's ADR is 0021.

## Risks

- **Merge conflicts across parallel PRs.** Every element PR appends to the barrel, `kAssistantUiElements`, `ELEMENTS.md` and the widgetbook directory. Mitigation: keep each list one entry per line, sorted. Conflicts are then trivial rebases.
- **`chat_screen.dart` hotspot.** PRs 5a, 7a, 7c, 8b, 9, 10, 11a and 12 of this change edit `chat_screen.dart`, and so do chat-stop-cancels-turn PR 5, chat-a11y-and-code-blocks C1 and several sibling wiring PRs. Mitigation: each PR's wiring goes into a small helper file under `features/chat/` (for example `chat_thread_slots.dart`, `chat_markdown_config.dart`) so `chat_screen.dart` gains one call per PR; PR 10 is stacked on PR 9; PR 5a and PR 11a merge by rebasing in tasks order; chat-a11y C1 lands after PR 5a.
- **Moving files siblings also edit.** chat-stop-cancels-turn edits `chat_models.dart`, `streaming_timeline_entry.dart`, `message_meta_actions.dart`, `turn_progress_card.dart`, `chat_input_row.dart` and `chat_screen.dart`. The edges in `tasks.md` order these: chat-stop PR 4 stacks on PR 2 (so it adds no arms to deleted files), PR 4 here comes after chat-stop PR 4 (so the moved enums include `stopped` and `cancelled`), and PRs 9 and 11a here come after chat-stop PR 5 (so `TurnProgressCard`'s Skip→`stopTurn` and the composer's `isStopping` move with them).
- **Visual baselines.** About ten sibling PRs update the Playwright PNG baselines. Each PR of this change runs the visual suite against the baselines current on `main` at that time. Nobody merges PNGs by hand: after a rebase, regenerate the baselines with the e2e-testing skill and record the diff in the PR body.
- **Sibling drift.** A sibling could build a widget in `app/lib/features/chat/` before its seam lands. Mitigation: `tasks.md` names the exact PR each sibling depends on, and the boundary README states the rule.
- **Golden flake.** Linux-only goldens, the default test font, a fixed clock, `pump(Duration)` instead of `pumpAndSettle`, and no network images in goldens: attachments render through the app-supplied slot, and markdown images through the placeholder, both of which goldens stub.
- **`flutter_smooth_markdown` upgrades.** 0.12.1 is out, while we pin `^0.8.1`. An upgrade may drop the legacy `ThemeData` dependency. That is out of scope, and the single `legacy_theme.dart` file keeps the blast radius to one place.
- **Test churn.** About 3,000 LOC of chat widget tests (`app/test/widget/features/chat/`) move or re-point. Each PR moves the tests of the widgets it moves, so coverage never drops between PRs.

## Open questions

None. The product questions that were open are settled in Decision 8. Sibling-owned choices (renderer data shape, interaction card layout, A2UI dependency route) are decided in their own changes against the seams above.
