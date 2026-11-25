# Tasks: chat-element-library

This change is a stack of 20 PRs. Each one stays under about 500 changed lines and leaves `main` shippable. It is client-only: there are no server PRs, no API changes and no migrations.

**Numbering.** The PR numbers 1 to 13 are kept from the earlier plan so that sibling citations stay valid; PRs that were too large are split with letter suffixes (5a/5b, 7a/7b/7c, 8a/8b, 11a/11b/11c), and the characterization PR is 1b. A sibling citation of a bare split number (for example "PR 7") means all of its parts. **The sibling unblock map below is the source of truth.** Siblings should cite the seam name alongside the PR number (for example "PR 6 (`ToolResultRendererScope`)"), so later splits do not break them.

Every element PR follows the same checklist, so it is not repeated per PR:

- it adds the element's entry to `kAssistantUiElements`;
- it adds a widgetbook use-case;
- it adds at least one golden test tagged `golden`, with a fixed clock where time is shown and `pump(Duration)` instead of `pumpAndSettle`;
- for an extracted widget, it first renders the PR 1b characterization inputs through the new element and matches those goldens **byte for byte**, then points the characterization test at the element, then deletes the app widget;
- it updates the element's `ELEMENTS.md` row;
- it moves the moved widget's existing tests alongside it;
- it keeps every existing widget `Key`;
- it runs `flutter analyze --fatal-infos` and `flutter test` (app and package) green;
- it runs the Playwright visual suite (`crates/web-ui/e2e`, see the e2e-testing skill) against the chat baselines current on `main` at that time, with no baseline update. A diff is a regression to fix. Never merge PNG baselines by hand: if a rebase brings in a sibling's baseline change, regenerate with the e2e-testing skill and record the diff in the PR body;
- its `chat_screen.dart` wiring goes through one small helper file under `features/chat/` where possible, so `chat_screen.dart` gains one call per PR (see Risks in design.md).

## Parallelism

```
PR 2 (dead code) ─────────────────────────┬──────────────── PR 6
                                          │
PR 1 ──┬─ PR 1b (characterization) ───────┤
       ├─ PR 3 (gallery) ──┐              │
       └─ PR 4 (models)* ──┴─┬─ PR 5a ── PR 5b ── PR 6 ── PR 7a ── PR 7b ── PR 7c ──┐
                             ├─ PR 8a (actions) ─────────────┬─ PR 8b (bubble) ←7a ─┤
                             ├─ PR 9* ── PR 10                                      ├─ PR 13
                             ├─ PR 11a* ── PR 11b                                   │
                             ├─ PR 11c                                              │
                             └─ PR 12 ──────────────────────────────────────────────┘
* also waits on chat-stop-cancels-turn: PR 4 after chat-stop PR 4; PR 9 and PR 11a after chat-stop PR 5.
Every extraction PR (5b onward, 8a, 8b, 9, 11a–c, 12) also depends on PR 1b.
```

- **Start immediately, in parallel:** PR 1 and PR 2.
- **Once PR 1 merges:** PR 1b, PR 3, and PR 4 (PR 4 also needs chat-stop PR 4).
- **Once PR 1b, PR 3 and PR 4 merge**, these run in parallel: the timeline chain (5a → 5b → 6 → 7a → 7b → 7c), PR 8a, PR 9 → PR 10, PR 11a → PR 11b, PR 11c, PR 12.
- PR 6 also needs PR 2 (it moves the tool status icons whose test coverage PR 2 ports).
- PR 8b needs PR 7a (`ChatMarkdownScope`) and PR 8a.
- PR 10 is stacked on PR 9 (both edit the same region of `chat_screen.dart`).
- PR 5a and PR 11a both touch `chat_screen.dart`; whichever lands second rebases.
- PR 13 needs everything.

## Stack waves

The cross-change plan in `openspec/chat-ui-parity-stack.md` assigns these waves (its "CEL PR 5", "PR 7" and "PR 11" mean all of the lettered parts here; 1b, 7b/7c and 11b/11c follow their lettered siblings). Only the order matters, and the "Depends on" lines above are authoritative.

| Wave | PRs |
| ---- | --- |
| 1 | PR 1, PR 2 |
| 2 | PR 1b, PR 3, PR 4 (after chat-stop PR 4) |
| 3 | PR 5a, PR 5b, PR 8a, PR 9 and PR 11a (after chat-stop PR 5), PR 10 (stacked on PR 9), PR 11c, PR 12 |
| 4 | PR 6, PR 11b |
| 5 | PR 7a, PR 7b, PR 7c |
| 6 | PR 8b |
| 7 | PR 13 (includes the archive, task 13.8) |

Hotspot rule in `chat_screen.dart`: PR 9, then PR 10 (stacked), with PR 5a and PR 11a rebasing.

## Sibling unblock map

| Sibling                      | Depends on (PR and seam)                                                                                                                                                                                                                                                                                                          |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| tool-result-renderers        | C2: PR 3 (gallery, registry) and PR 4 (`ToolCallRecord` in the package, to add `data`). C3: PR 6 (`ToolResultRendererScope`), installed in `ChatScreen` as `ToolResultRendererScope(resolve: registry.build)`. No `ToolCallBody` constructor change.                                                                              |
| tool-approval-prompts        | Elements (PR 13, PR 14): PR 1 and PR 3. PR 16: PR 5b (`EntryState.awaitingInput`, `TimelineEntryShell.footer`) and PR 6 (`TimelineEntryView.footer` forwarding on tool call entries). Its `TimelineEntryType.interaction` fallback adds a `TimelineEntryView` case after PR 7c. The PR that turns interactive approvals on for web turns lands after the client `ApprovalCard` and `QuestionCard` wiring, so PR 5b and PR 6 must be merged first; no feature flag (Decision 8d).                                                   |
| message-branching            | Elements (PR 9): PR 1 and PR 3. Branching auto-cancels a pending approval, which leaves `awaitingInput` through the existing shell transitions (Decision 8d). PR 10: PR 8a (`MessageActions.leading`, app-side `buildMessageActions`); it adds Edit and Regenerate `MessageActionButton`s there plus a pure `RegenerateMenuButton(models, onSelected)` element, not new parameters on `MessageActions`.                                                         |
| message-feedback             | PR 5 (elements): PR 3 and PR 8a (`MessageActions.trailing`, `MessageActionButton.selected`).                                                                                                                                                                                                                                      |
| follow-up-suggestions        | PR 6: PR 1 and PR 3. PR 7: PR 10 (`ChatThreadLayout.aboveComposer`, appended after `TurnProgressCard`). PR 8: PR 9 (`ThreadEmptyState.starters`).                                                                                                                                                                                 |
| generative-ui-a2ui           | PR 9: PR 3 (gallery, registry) and PR 5b (shell `body` slot); its `MessageBubble.extensions` use needs PR 8b; its `TimelineEntryType.surface` case needs PR 7c (`TimelineEntryView`).                                                                                                                                             |
| conversation-list-management | Element PRs: PR 3 and PR 4. PRs 11 and 15: PR 12 (`ConversationListView`, `onDelete: Future<bool>`); the `selectedId` parameter and the selection highlight are theirs to add (off here, Decision 8a).                                                                                                                                                                                    |
| chat-a11y-and-code-blocks    | C1 (breakpoints): after PR 5a (`ThreadDensityScope`). C3/C4/C10: PR 1, PR 3, PR 4. C5: PR 7a (`ChatMarkdownScope`) and PR 8b. C6: PR 5b and PR 8b. C7/C8: PR 9 and PR 11a. C9: PR 10 (`ThreadViewport`). C11: PR 8a (`MessageActions.trailing`, timing first). Archive after this change.                                         |
| chat-stop-cancels-turn       | Its PR 4 (including `stopped_label` in `MetaActionRow`) stacks on PR 2 here and adds no arms to `tool_call_chip.dart` or `timeline_section.dart`. Then PR 4 here moves the enums, including `MessageStatus.stopped` and `ToolCallStatus.cancelled`. PRs 9 and 11a here come after its PR 5, so `TurnProgressCard` moves with Skip→`stopTurn`, and `Composer` takes `isStopping`. |

---

## PR 1 — build(app): scaffold assistant_ui package with purity boundary

Layer: client
Depends on: none
Parallel with: PR 2

- [ ] 1.1 Write `app/test/unit/assistant_ui_boundary_test.dart`. It fails if (a) `packages/assistant_ui/pubspec.yaml` `dependencies` differs from `{flutter, flutter_smooth_markdown, material_ui}`; (b) any `import` or `export` under `packages/assistant_ui/lib/` references `flutter_riverpod`, `assistant_api`, `cupertino_ui`, `package:flutter/cupertino.dart`, `dart:io`, `record`, `file_picker`, `desktop_drop`, `url_launcher`, `audioplayers`, `http`, `cached_network_image`, `flutter_svg`, `share_plus` or `path_provider`; (c) `package:flutter/material.dart` appears outside `lib/src/theme/legacy_theme.dart`; (d) source contains `defaultTargetPlatform`, `.platform ==` or `TargetPlatform.`. `.adaptive` constructors are allowed (design.md Decision 2). Run it and confirm RED, because the package does not exist yet.
- [ ] 1.2 Scaffold `app/packages/assistant_ui/` with `pubspec.yaml` (allowlist only, `resolution: workspace`), `lib/assistant_ui.dart` barrel, `lib/models.dart` (models only, filled in PR 4), `lib/src/`, `analysis_options.yaml` including `flutter_lints`, and `dart_test.yaml` declaring the `golden` tag. Turn `app/pubspec.yaml` into the workspace root (`workspace: [packages/assistant_ui]`) and add the path dependency. Confirm 1.1 GREEN.
- [ ] 1.3 Test first: `packages/assistant_ui/test/theme/tokens_test.dart` asserts `AssistantUiColors.toolOk == Colors.green` (ARGB `0xFF4CAF50`), `toolDenied == Colors.amber` (`0xFFFFC107`), `fadeMask == Color(0xFFFFFFFF)`, and an app test asserts `AssistantColors.warning == AssistantUiColors.warning` and each `AssistantSpacing` value equals its `AssistantUiSpacing` twin. Confirm RED. Add `lib/src/theme/legacy_theme.dart` (`legacyThemeOf`), `assistant_ui_colors.dart` and `assistant_ui_spacing.dart`. Redefine the app's `AssistantColors.warning` and `AssistantSpacing` in terms of them. Extend `app/scripts/check_no_raw_colors.sh` to scan `packages/assistant_ui/lib/`, exempting only `lib/src/theme/`. Confirm GREEN.
- [ ] 1.4 Add the golden harness `packages/assistant_ui/test/support/golden.dart`: `pumpGolden(tester, widget, {size, brightness, now})` wraps in `MaterialApp` with the app's light and dark `ThemeData` built from `ColorScheme.fromSeed`, disables animations where needed, and exposes a fixed `now` for elements that show time. It advances with `pump(Duration)`; `pumpAndSettle` is banned in golden tests. Add a smoke golden of a `SizedBox`. Add Makefile target `update-goldens`, running `flutter test --update-goldens --tags golden` in the package and in `app/` inside the Flutter Docker image pinned to `app/pubspec.yaml`'s Flutter version.
- [ ] 1.5 Make `make test-flutter` run both the app and the package suites, passing `--exclude-tags golden` unless `CI` is set or `uname` is `Linux`. Add a `flutter test` step in `packages/assistant_ui` to `.github/workflows/flutter.yml` `analyze-and-test` (CI runs goldens).
- [ ] 1.6 Write `packages/assistant_ui/README.md` covering the purity rule (including the `.adaptive` and `imageBuilder` notes), the injection rule, the composition rule (constructor slots, plus three named scopes), the allowlist amendment rule (design.md Decision 2), and the per-PR checklist above.
- [ ] 1.7 Move `openspec/changes/chat-element-library/elements.md` to `packages/assistant_ui/ELEMENTS.md` and link it from the README.
- [ ] 1.8 Write `docs/adr/adr-0021-assistant-ui-element-package.md`: the package boundary and allowlist, the amendment rule, the pub workspace, slots versus the three scopes, and the characterization-golden rule. (0017–0019 are taken by tool-approval-prompts, message-branching and generative-ui-a2ui, and 0020 by tool-result-renderers; confirm 0021 is still free when the PR is cut.)
- [ ] 1.9 `make lint-flutter && make test-flutter` green on macOS (goldens skipped) and in CI (goldens run).

## PR 1b — test(app): characterization goldens of the current chat widgets

Layer: client
Depends on: PR 1
Parallel with: PR 2, PR 3, PR 4

- [ ] 1b.1 Add `app/test/support/golden.dart`, matching the package harness (same themes, fixed clock, `pump(Duration)`).
- [ ] 1b.2 Under `app/test/golden/characterization/`, capture goldens of the **current** app widgets, all tagged `golden`: `StreamingTimelineEntry` for thinking (active with stream, complete), tool call (pending, ok, error, denied; collapsed and expanded), subagent (running, complete with summary, more than 2 inner calls) and command, each at the three densities and in the stale state; `MessageBubble` (user, user with attachment, user voice, assistant markdown, assistant with attachment, grouped, failed, streaming-empty) with stubbed media; `MetaActionRow` (all actions, retry only, empty); `ChatInputRow` (idle, sending, queue badge, attachments, slash menu open); `TurnProgressCard` (active, stalled with Skip) at a fixed `now`; `ConversationList` (empty, several items) at a fixed `now`; `EmptyChat`. Generate them with `make update-goldens`.
- [ ] 1b.3 These files are the before-image. Each extraction PR re-points its cases at the new element and must keep the images unchanged. If a sibling intentionally changes one of these widgets first (for example chat-stop-cancels-turn's cancelled icon), it regenerates the affected goldens in its own PR.
- [ ] 1b.4 CI green.

## PR 2 — refactor(app): delete dead ToolCallChip and ChatTimelineSection

Layer: client
Depends on: none
Parallel with: PR 1, PR 1b, PR 3, PR 4
Unblocks: PR 6, chat-stop-cancels-turn PR 4

- [ ] 2.1 Test first: compare the status-icon assertions in `tool_call_chip_test.dart` (spinner, check, cross, prohibition) with `app/test/widget/features/chat/streaming_timeline_entry_test.dart`. Port any that are missing into the latter, so `tool-call-display` "status icons are distinct and accessible" keeps coverage against the live widget. Any ported assertion that fails exposes a real gap in the live widget; fix it in this PR.
- [ ] 2.2 Delete `app/lib/features/chat/tool_call_chip.dart`, `timeline_section.dart`, `app/test/widget/tool_call_chip_test.dart` and `app/test/widget/timeline_section_test.dart`. Remove both from `app/scripts/theme_color_allowlist.txt`. Fix the doc comment at `streaming_timeline_entry.dart:43`. Do not add a shim.
- [ ] 2.3 Grep `app/lib` and `app/test` for `ToolCallChip` and `ChatTimelineSection`; both must have zero hits.
- [ ] 2.4 `flutter analyze --fatal-infos` and `flutter test` green.

## PR 3 — build(app): add widgetbook gallery and element registry

Layer: client
Depends on: PR 1
Parallel with: PR 1b, PR 2, PR 4

- [ ] 3.1 Test first, in `packages/assistant_ui/test/registry_test.dart`. It parses the `export` directives of `lib/assistant_ui.dart` and fails if an exported element is missing from `kAssistantUiElements`; it also fails if a registry entry that names a catalogue element has no Adopt, Adopt (scoped) or Adopt (have) row in `ELEMENTS.md`. (The reverse check, that every Adopt row is registered, is added in PR 13, because rows precede their elements.) Confirm RED, because `kAssistantUiElements` does not exist yet.
- [ ] 3.2 Add `kAssistantUiElements`, an empty, sorted, one-entry-per-line `const List<String>`. Confirm GREEN.
- [ ] 3.3 Scaffold `app/widgetbook/` as a workspace member (`resolution: workspace`), depending on `widgetbook: 3.25.0` and `assistant_ui` only. Use hand-written `WidgetbookComponent`s, with no generator. Add knobs helpers for `EntryState` and `TimelineDensity`, and light and dark theme addons.
- [ ] 3.4 Test first, in `app/widgetbook/test/coverage_test.dart`. It fails if any `kAssistantUiElements` entry lacks a use-case in the widgetbook directory, or lacks a `golden`-tagged test in the package. It is GREEN trivially while the registry is empty.
- [ ] 3.5 Verify `make build-web` output contains no widgetbook symbols (grep `app/build/web/main.dart.js` for `Widgetbook`). Record the check in the PR body. Add `cd app/widgetbook && flutter test` to CI.
- [ ] 3.6 `make lint-flutter && make test-flutter` green.

## PR 4 — refactor(app): move render models into assistant_ui

Layer: client
Depends on: PR 1, **chat-stop-cancels-turn PR 4** (so `MessageStatus.stopped` and `ToolCallStatus.cancelled` move with the enums)
Parallel with: PR 1b, PR 2, PR 3
Unblocks: tool-result-renderers C2, conversation-list-management element PRs

- [ ] 4.1 Test first: `packages/assistant_ui/test/models/chat_message_test.dart` constructs `ChatMessage`, `ToolCallRecord` and `PendingAttachment` from `package:assistant_ui/models.dart`, mutates `content`, `isStreaming`, `isStale`, `status` and `ToolCallRecord.status` in place, checks `copyWith`, and asserts the enum value sets (including `stopped` and `cancelled`). Confirm RED.
- [ ] 4.2 Move these from `app/lib/features/chat/chat_models.dart` to `packages/assistant_ui/lib/src/models/`, unchanged: `streamingPlaceholderId`, `MessageStatus`, `ToolCallStatus`, `ToolCallRecord`, `TimelineEntryType`, `ChatAttachment`, `ChatMessage`, `PendingMessage`, `TurnEventKind` and `TurnStatusSnapshot`. Move `PendingAttachment` out of `attachment_provider.dart`, and `CommandEntry` and `CommandArg` out of `commands_provider.dart`. Move `TimelineDensity` and `EntryState` out of `streaming_timeline_entry.dart:13-41`. `ChatState` and `ChatToolResult` stay. Export them from `lib/models.dart` and from the barrel. Confirm GREEN.
- [ ] 4.3 Re-point importers. `chat_provider.dart` re-exports `package:assistant_ui/models.dart` only (as it re-exports `chat_models.dart` today at L20), not the whole barrel, so widget names never collide at import sites. Move model-only tests from `app/test/unit/chat/` if they test only moved types.
- [ ] 4.4 `flutter analyze --fatal-infos` and `flutter test` green.

## PR 5a — feat(app): ThreadDensityScope

Layer: client
Depends on: PR 1b, PR 3, PR 4
Parallel with: PR 8a, PR 9, PR 11a, PR 11c, PR 12
Unblocks: chat-a11y-and-code-blocks C1

- [ ] 5a.1 Test first, `test/thread_density_scope_test.dart`: a descendant resolves `compact`; rebuilding with `expanded` rebuilds it; without a scope it resolves `normal`. Confirm RED.
- [ ] 5a.2 Implement `ThreadDensityScope` (`InheritedWidget`, `updateShouldNotify` on value change) using `TimelineDensity.fromWidth`. Confirm GREEN.
- [ ] 5a.3 `ChatScreen` wraps the message list in `ThreadDensityScope`, replacing the per-item `TimelineDensity.fromWidth` call at `chat_screen.dart:510-512`. `StreamingTimelineEntry` drops its `density` parameter and reads the scope. Characterization goldens unchanged.
- [ ] 5a.4 Use-case and golden.

## PR 5b — feat(app): TimelineEntryShell with footer slot and awaitingInput

Layer: client
Depends on: PR 5a
Parallel with: PR 8a, PR 9, PR 11a, PR 11c, PR 12
Unblocks: tool-approval-prompts PR 16, generative-ui-a2ui PR 9 (body slot), chat-a11y-and-code-blocks C6

- [ ] 5b.1 Test first, `test/timeline_entry_shell_test.dart`, by moving the state-machine cases of `app/test/widget/features/chat/streaming_timeline_entry_test.dart` (git move, then edit): active to complete auto-collapses after 500 ms; pinning survives complete and stale; reduced motion collapses without a timer; tap toggles and pins; stale renders at 0.6 opacity. Add the new cases: a footer renders while collapsed and the body does not; awaitingInput expands and cancels a pending collapse timer; tap does not collapse while awaitingInput; awaitingInput→complete schedules auto-collapse unless pinned (with the reduced-motion variant); awaitingInput→active stays expanded at every density; awaitingInput→stale collapses unless pinned; a pin made while active survives active→awaitingInput→complete. Confirm RED.
- [ ] 5b.2 Add `EntryState.awaitingInput`. Implement `TimelineEntryShell(icon, label, status, entryState, expandable, forceExpand, inline, body, footer)` by lifting `_expanded`, `_userPinned`, `_collapseTimer`, `_scheduleAutoCollapse`, `_onTap`, `_buildSection` and `_fadeContainer` out of `streaming_timeline_entry.dart:63-160,529-671`. The footer renders after the `AnimatedSize` collapsible region (`:633-660`), outside it. Replace the raw colours with `AssistantUiColors.fadeMask`. Do not leave a copy behind: `StreamingTimelineEntry` becomes stateless and builds a shell per type. Confirm GREEN, with characterization goldens unchanged.
- [ ] 5b.3 Use-cases (the 12-cell `EntryState` × `TimelineDensity` matrix, plus with a footer) and goldens (active, complete, stale and awaitingInput at normal density; compact active; collapsed with footer).

## PR 6 — feat(app): ToolCallBody with tool result resolver scope

Layer: client
Depends on: PR 5b, PR 2
Parallel with: PR 8a, PR 9, PR 10, PR 11a, PR 11b, PR 11c, PR 12
Unblocks: tool-result-renderers C3, tool-approval-prompts PR 16

- [ ] 6.1 Test first, `test/bodies/tool_call_body_test.dart`: the resolver result wins; a null result falls back; no scope means the generic view; the error status force-expands (current behaviour, `streaming_timeline_entry.dart:257`); duration badge text such as `1.2s` and `1m 5s`; status icons for pending, ok (`toolOk`), error, denied (`toolDenied`) and cancelled (the muted block icon from chat-stop-cancels-turn, which PR 4 carries along). Confirm RED.
- [ ] 6.2 Implement `ToolResultRendererScope(resolve)`, `typedef ToolResultBuilder = Widget? Function(BuildContext, ToolCallRecord)` (no registry type: tool-result-renderers owns `ToolResultRendererRegistry` and passes `registry.build`), `GenericToolResultView` (lifted from `_toolDetails`, `:324-382`) and `ToolCallBody` (lifted from `_buildToolCall`, `_toolStatusWidget` and `_toolStatusIcon`, `:231-322`), with `Colors.green` and `Colors.amber` replaced by the equal-valued tokens. Export `ToolStatusIcon` for reuse by `SubagentBody`. Confirm GREEN.
- [ ] 6.3 `StreamingTimelineEntry`'s tool branch delegates to `ToolCallBody` and forwards a `footer` argument to the shell. Move the tool-call cases of `streaming_timeline_entry_test.dart` to the package. Characterization goldens unchanged.
- [ ] 6.4 `ChatScreen` installs no `ToolResultRendererScope` yet, because the absent scope means the generic view. tool-result-renderers C3 adds it.
- [ ] 6.5 Use-cases and goldens: pending, ok, error and denied; with a footer; with a custom resolver.

## PR 7a — feat(app): ChatMarkdownScope and ThinkingBody

Layer: client
Depends on: PR 6
Parallel with: PR 8a, PR 9, PR 10, PR 11a, PR 11b, PR 11c, PR 12
Unblocks: PR 8b, chat-a11y-and-code-blocks C5 (code-block builder)

- [ ] 7a.1 Test first, `test/chat_markdown_scope_test.dart`: a registered `mermaid` builder renders; a scope `imageBuilder` renders in place of a markdown image; without a scope a markdown image renders the alt-text placeholder and no `CachedNetworkImage`, `Image.network` or `SvgPicture.network` is in the tree; `onTapLink` receives the URL; without a scope, links are ignored and do not throw. Confirm RED.
- [ ] 7a.2 Implement `ChatMarkdownScope(plugins, builders, imageBuilder, onTapLink, styleSheetOverride)` and the placeholder image builder. Add `features/chat/chat_markdown_config.dart` in the app, which builds the scope with the Mermaid and SVG plugins and builders from `message_bubble.dart:213-225`, `MarkdownLinkHandler`, and an image builder that keeps today's rendering (`CachedNetworkImage` for raster URLs, `SvgPicture.network` for `.svg`, as `flutter_smooth_markdown`'s default `ImageBuilder` does). `ChatScreen` installs it. `SvgBuilder`, `ThemeAwareMermaidBuilder` and `MarkdownLinkHandler` stay in the app. Confirm GREEN.
- [ ] 7a.3 Test first, `test/bodies/thinking_body_test.dart`: active with a stream renders `StreamMarkdown` with the scope's `onTapLink` and no plugins (a scope `mermaid` builder is not applied); complete renders `**bold**` literally as selectable text; empty is not expandable; `elapsed` label ("Thought for 4s" and "Thought"); no per-chunk divisions. Confirm RED.
- [ ] 7a.4 Lift `ThinkingBody` (`streaming_timeline_entry.dart:168-229`) unchanged in behaviour. `StreamingTimelineEntry`'s thinking branch delegates to it. Confirm GREEN, characterization goldens unchanged.
- [ ] 7a.5 Use-cases and goldens for `ChatMarkdownScope` (via a sample body) and `ThinkingBody`.

## PR 7b — feat(app): SubagentBody and CommandBody

Layer: client
Depends on: PR 7a
Parallel with: PR 8a, PR 9, PR 10, PR 11a, PR 11b, PR 11c, PR 12

- [ ] 7b.1 Test first: `test/bodies/subagent_body_test.dart` (running; complete with summary; normal density shows the last 2 inner calls, expanded shows all; each inner call is one compact row of icon, name and `ToolStatusIcon` at every density, does not expand, and does not call the resolver of an ancestor `ToolResultRendererScope`: rich tool renderers never appear in subagent timelines, design.md Decision 8b) and `command_body_test.dart` (port `command_event_tile_test.dart`). Confirm RED.
- [ ] 7b.2 Lift `SubagentBody` (`:385-527`, inner rows as at `:470-499`, `Colors.amber`/`Colors.green` replaced by the equal tokens) and `CommandBody` (from `command_event_tile.dart`). Delete `command_event_tile.dart`. Confirm GREEN, characterization goldens unchanged.
- [ ] 7b.3 Use-cases and goldens for each body.

## PR 7c — refactor(app): TimelineEntryView replaces StreamingTimelineEntry

Layer: client
Depends on: PR 7b
Parallel with: PR 8a, PR 9, PR 10, PR 11a, PR 11b, PR 11c, PR 12
Unblocks: generative-ui-a2ui (`surface` case), tool-approval-prompts (`interaction` fallback case)

- [ ] 7c.1 Test first, `test/timeline_entry_view_test.dart`: each `TimelineEntryType` maps to its body inside a shell; `footer` reaches the shell; `entryState` reaches the shell. Confirm RED.
- [ ] 7c.2 Implement `TimelineEntryView(message, entryState, footer)`. `ChatScreen` uses it for every non-message entry, including commands. Delete `streaming_timeline_entry.dart` and remove it from `theme_color_allowlist.txt`. Move its remaining tests into the package. Re-point the timeline characterization goldens at `TimelineEntryView`. Confirm GREEN, images unchanged.
- [ ] 7c.3 Use-case and golden.

## PR 8a — feat(app): MessageActions and MessageActionButton

Layer: client
Depends on: PR 1b, PR 3, PR 4
Parallel with: the timeline chain, PR 9–12
Unblocks: message-feedback PR 5, message-branching PR 10, chat-a11y-and-code-blocks C11

- [ ] 8a.1 Test first, `test/message_actions_test.dart`: order is `leading`, actions in the order given, then `trailing` items in order; all empty renders a zero-size box; `MessageActionButton(selected: true)` reports selected semantics and uses the selected colour; `tone: danger` uses `colorScheme.error`; a tap fires `onPressed`. Confirm RED.
- [ ] 8a.2 Implement `MessageActionButton(icon, label, tone, selected, onPressed)` (lifted from `MetaActionButton`, `message_meta_actions.dart:165-195`) and `MessageActions(actions, leading, trailing: List<Widget>)`. Confirm GREEN.
- [ ] 8a.3 Test first, `app/test/unit/chat/message_actions_builder_test.dart`: `buildMessageActions(message, capabilities, …)` returns stop, read-aloud, copy, share, save and retry in today's order and conditions, with keys `stop_action`, `read_aloud_action`, `copy_action`, `share_action`, `save_action` and `retry_button`, and retry in the `danger` tone; it composes `trailing` in the fixed order timing, then feedback (both empty until chat-a11y-and-code-blocks C11 and message-feedback fill them); and for an assistant message with status `stopped` (live or reloaded from the persisted finish reason) it returns a `stopped_label` reading "Stopped" in place of the Stop action and keeps Copy, carrying over the behaviour chat-stop-cancels-turn adds to `MetaActionRow`. Confirm RED. Replace `MetaActionRow` with `buildMessageActions` in `features/chat/message_actions_builder.dart` feeding `MessageActions`; `ReadAloudAction` stays in the app as one of the supplied widgets. Delete `message_meta_actions.dart`. Confirm GREEN, `MetaActionRow` characterization goldens unchanged.
- [ ] 8a.4 Use-cases and goldens: muted, danger, selected; a full row.

## PR 8b — feat(app): MessageBubble, MessageBody and TypingIndicator

Layer: client
Depends on: PR 7a, PR 8a
Parallel with: PR 9, PR 10, PR 11a, PR 11b, PR 11c, PR 12
Unblocks: generative-ui-a2ui (`extensions`), chat-a11y-and-code-blocks C5 and C6

- [ ] 8b.1 Test first, `test/message_bubble_test.dart`: user and assistant alignment and colours; `isGrouped` margin; failed icon; typing indicator while streaming with empty content; user `attachments` above the text; user `audio` replaces the text row; assistant content as markdown through `MessageBody` with `attachments` then `audio` below; `extensions` inside the decoration below the content; `actions` below the bubble; a markdown image without a scope builds no network image. Confirm RED.
- [ ] 8b.2 Implement `MessageBody` (streaming `StreamMarkdown` or static `SmoothMarkdown`, lifted from `message_bubble.dart:139-164`, always passing an `imageBuilder`), `TypingIndicator` (moved from `typing_dots.dart`) and `MessageBubble` with slots and `contextMenuBuilder` and `selectionToolbarBuilder` builders. Confirm GREEN.
- [ ] 8b.3 The app's `message_bubble.dart` becomes `ChatMessageBubbleAdapter` (distinct name), which passes `MessageAttachments`, `VoiceMessagePlayer` and `AudioPlayerWidget` (inside `AdaptiveMediaContextMenu`), `AdaptiveTextSelectionToolbar` and the `buildMessageActions` row into the slots. Re-point the bubble characterization goldens; images unchanged.
- [ ] 8b.4 Use-cases and goldens: user, user with attachment, user voice, assistant, grouped, failed, streaming-empty, and with an extension.

## PR 9 — refactor(app): pure TurnProgressCard, error banner and empty state

Layer: client
Depends on: PR 1b, PR 3, PR 4, **chat-stop-cancels-turn PR 5**
Parallel with: the timeline chain, PR 8a, PR 11a–c, PR 12
Unblocks: PR 10, follow-up-suggestions PR 8 (starters)

- [ ] 9.1 Test first: port `app/test/widget/features/chat/turn_progress_card_test.dart` to `packages/assistant_ui/test/turn_progress_card_test.dart`, pumping with literal `status`, `now`, `showSkip` and `onSkip` and no `ProviderScope` (null renders nothing; active label; stall label; skip fires `onSkip`). Confirm RED.
- [ ] 9.2 Move `TurnProgressCard`, `kTurnStallThreshold`, `turnStatusLabel` and `stalledTurnStatusLabel` (from `turn_status_label.dart`) into the package, with parameters replacing `ref.watch` and `ref.read` (`turn_progress_card.dart:65,135`). The 1-second ticker (`:42-59`) becomes an injectable clock; the app passes the real one. `kSkipButtonEnabled` and the `stopTurn()` call (from chat-stop-cancels-turn) stay in `ChatScreen`. Confirm GREEN, characterization goldens unchanged.
- [ ] 9.3 Test first: `ThreadErrorBanner(message, onDismiss)` (lifted from `chat_screen.dart:416-456`) and `ThreadEmptyState(title, subtitle, starters)` (from `empty_chat.dart`, default copy plus the starters slot). Confirm RED, implement, delete `empty_chat.dart`. Confirm GREEN.
- [ ] 9.4 Use-cases and goldens for all three.

## PR 10 — feat(app): chat thread layout and viewport

Layer: client
Depends on: PR 9 (stacked; same `chat_screen.dart` region)
Parallel with: the timeline chain, PR 8a, PR 11a, PR 11b, PR 11c, PR 12
Unblocks: follow-up-suggestions PR 7, chat-a11y-and-code-blocks C9

- [ ] 10.1 Test first, `test/thread_viewport_test.dart`, from `openspec/specs/chat-auto-scroll` and the spec scenarios: 80 dp threshold; at bottom a higher `revision` animates to the bottom; scrolled away it does not move and shows `scroll_to_bottom_button`; the button returns and hides; `ThreadViewportController.scrollToBottom()` after a conversation switch scrolls even when scrolled away. Port the matching cases from `chat_screen_test.dart`. Add app-level tests in `chat_screen_test.dart` that, at the bottom, each of these scrolls: thinking tokens growing `thinkingContent` only; a subagent tool call appended in place; a tool status flip; a queued message added; opening a different conversation. Confirm RED.
- [ ] 10.2 Implement `ThreadViewport(itemCount, itemBuilder, revision, controller)` and `ThreadViewportController(scrollToBottom, scrollToBottomIfNear)`, lifting `_onScroll`, `_scrollToBottom` and the FAB (`chat_screen.dart:86-91,146-156,560-571`). Only once it is green, delete those members from `_ChatScreenState`. `ChatScreen` keeps a `ref.listen(chatProvider)` that increments an `int _revision` on every emission (no `setState` needed beyond what the watch already triggers) and passes it; the conversation-load calls at `:110,226,231` become `controller.scrollToBottom()`; `_onInputFocusChange` calls `scrollToBottomIfNear()`. Confirm GREEN.
- [ ] 10.3 Test first: `ChatThreadLayout` (banner and aboveComposer order, empty lists render nothing), a `chat_screen_test.dart` case asserting today's vertical order: error banner, history `LinearProgressIndicator`, thread, `ReconnectBanner`, `TurnProgressCard`, composer, and a case with a reconnect in progress and no active turn asserting `ReconnectBanner` sits below the thread, directly above the composer (position unchanged, Decision 8c). Confirm RED. Implement and adopt in `ChatScreen` with `banners: [ThreadErrorBanner?, LinearProgressIndicator?]` and `aboveComposer: [ReconnectBanner, TurnProgressCard]`. Confirm GREEN.
- [ ] 10.4 Use-cases and goldens for the viewport (at bottom and scrolled away) and the layout.

## PR 11a — refactor(app): Composer and SlashCommandMenu in assistant_ui

Layer: client
Depends on: PR 1b, PR 3, PR 4, **chat-stop-cancels-turn PR 5** (`isStopping`)
Parallel with: the timeline chain, PR 8a, PR 9, PR 11c, PR 12
Unblocks: PR 11b, chat-a11y-and-code-blocks C7/C8

- [ ] 11a.1 Test first, `test/composer/composer_test.dart`: send fires `onSend`; `isSending` shows `stop_button`, which fires `onStop`; `isStopping` disables it with the semantics label "Stopping…"; the queue badge shows the count; `inputBuilder` is used; the default input renders without the façade; the slash menu shows on `/` and selecting a no-arg command fires `onSend` (current behaviour, `chat_input_row.dart:87-101`); a `voiceButton` slot renders where the mic is today. Port `command_autocomplete_test.dart`. Confirm RED.
- [ ] 11a.2 Move `ChatInputRow` to `Composer` and `CommandAutocompletePopup` to `SlashCommandMenu`. Replace `ServerCapabilities` with `bool showVoice`, `AdaptiveTextField` with `inputBuilder`, and `VoiceRecorderButton` with a `voiceButton` slot (the app passes today's `VoiceRecorderButton`). Use `AssistantUiColors.warning` and `AssistantUiSpacing`. The attachment tray moves with `Composer` as private code for now. Drop the dead `Clipboard.getData` `KeyboardListener` (`chat_input_row.dart:245-260`) together with the unused `onPasteImage` parameter (`:27,42`) and its `ChatScreen` wiring (`chat_screen.dart:602`). Delete `chat_input_row.dart` and `command_autocomplete.dart`. Confirm GREEN, `ChatInputRow` characterization goldens unchanged.
- [ ] 11a.3 Use-cases and goldens: idle, sending, stopping, queue badge, slash menu open.

## PR 11b — refactor(app): AttachmentTray and ComposerVoiceButton

Layer: client
Depends on: PR 11a
Parallel with: the timeline chain, PR 8a, PR 9, PR 10, PR 11c, PR 12

- [ ] 11b.1 Test first: `AttachmentTray` (`remove_attachment_0` fires `onRemove(0)` and the list is unchanged; image thumbnail versus file tile) and `ComposerVoiceButton` (`onStart`; recording shows elapsed and stop). Confirm RED.
- [ ] 11b.2 Extract `AttachmentTray` from the composer's private tray (formerly `chat_input_row.dart:149-217`). Split `VoiceRecorderButton`: its presentation becomes `ComposerVoiceButton`, and the `record`-backed driver stays in `features/chat/voice_recorder_button.dart`, which renders `ComposerVoiceButton`. Confirm GREEN, characterization goldens unchanged.
- [ ] 11b.3 Use-cases and goldens for both.

## PR 11c — refactor(app): QueuedMessageBubble in assistant_ui

Layer: client
Depends on: PR 1b, PR 3, PR 4
Parallel with: everything after PR 4

- [ ] 11c.1 Test first: port `queued_message_bubble_test.dart` with `onLongPress` in place of the `ref` read. Confirm RED.
- [ ] 11c.2 Move `QueuedMessageBubble` to the package; the remove action sheet (`showAdaptiveActionSheet`) and the provider call stay in the app and are passed through `onLongPress`. Confirm GREEN.
- [ ] 11c.3 Use-case and golden.

## PR 12 — refactor(app): pure ConversationListView

Layer: client
Depends on: PR 1b, PR 3, PR 4
Parallel with: everything after PR 4
Unblocks: conversation-list-management PRs 11 and 15

- [ ] 12.1 Test first, `test/conversation_list_view_test.dart`: items render title and relative date against an injected `now`; a tap fires `onSelect` without navigating; a swipe with `onDelete` resolving `false` (declined) keeps the item; resolving `false` (failed delete) keeps the item and leaves no dismissed `Dismissible`; resolving `true` dismisses it; the empty list renders the `header` and no rows; the `trailingBuilder` renders. No row is highlighted as selected, whichever conversation is open, and the view declares no `selectedId` (Decision 8a; conversation-list-management adds it and turns the highlight on). Confirm RED.
- [ ] 12.2 Add the `ConversationListItem` model and `ConversationListView` (lifted from `conversation_list.dart:14-174`). In the app, `ConversationList` becomes `ConversationListPane` (distinct name), keeps its 5 `ref` uses, maps `ConversationListEntry` (`lib/api/models/stream_event.dart:65`) to `ConversationListItem`, supplies "New Chat" and `context.go`, and implements `onDelete` as today's `confirmDismiss` body: `showAdaptiveConfirmDialog`, then `await deleteConversation`, returning `false` on decline or on a caught error (`conversation_list.dart:123-140`). Confirm GREEN, characterization goldens unchanged.
- [ ] 12.3 Use-cases and goldens: empty, several items.

## PR 13 — refactor(app): chat screen adapter and element index reconciliation

Layer: client
Depends on: all PRs above
Parallel with: none

- [ ] 13.1 Test first: extend the boundary test so that `app/lib/features/chat/` declares no widget whose name equals a `kAssistantUiElements` entry (adapters are named `ChatMessageBubbleAdapter`, `ConversationListPane`, and so on). Confirm RED if any duplicate remains.
- [ ] 13.2 Reduce `chat_screen.dart` to wiring: provider reads, `file_picker`, `desktop_drop`, breakpoints, and the scopes (density, markdown, renderers). Target under ~300 lines. Remove leftover private widgets. Confirm GREEN.
- [ ] 13.3 Run the full pre-existing chat widget suite (`app/test/widget/features/chat/`, with imports updated). Every previously passing assertion must pass.
- [ ] 13.4 Delete `app/test/golden/characterization/` cases that are now duplicated by package goldens, keeping any that cover app-only composition (the adapters).
- [ ] 13.5 Test first: add the reverse check to `registry_test.dart`, failing if any Adopt (have) or Adopt (scoped) row in `ELEMENTS.md` names an element absent from `kAssistantUiElements`. Confirm GREEN (or fix the row). Reconcile `ELEMENTS.md` end to end: 59 elements (37 rows plus one 22-element category row), one verdict each; every Defer names its prerequisite; every "Adopt via" names a sibling change and seam; the tally matches.
- [ ] 13.6 Manual smoke test: stream a turn with thinking, a tool call and a subagent; scroll away mid-stream and use the button; switch conversations; record a voice message; drag and drop an attachment; run a slash command; delete a conversation, and delete one while offline (the row stays). Auto-collapse and pinning should behave as before.
- [ ] 13.7 `make lint-flutter && make test-flutter`, then `make lint && make format && make test`, all green.
- [ ] 13.8 Archive: `openspec archive chat-element-library`. This must run **before** chat-a11y-and-code-blocks is archived, because that change carries a MODIFIED delta of this capability. It removes the `tool-call-display` chip requirement and rewords the rest.

## Notes

- No migrations and no API changes, so no `make dump-openapi` or `make generate-flutter-client` tasks.
- ADR-0021 (task 1.8) records the package boundary; 0017–0020 are taken by siblings (tool-result-renderers holds 0020).
- If a sibling changes `ToolCallRecord` or `ChatMessage` before PR 4 merges, it must rebase onto PR 4 and edit the package copy. No duplicate model may exist in `app/lib`.
- Spec archive order across the family: chat-element-library, then chat-a11y-and-code-blocks; chat-stop-cancels-turn, then follow-up-suggestions, then tool-approval-prompts.
- Sibling follow-ups filed by this review (each sibling applies them from the same finding text): generative-ui-a2ui `tasks.md:11,99` and tool-approval-prompts `tasks.md:255,308` cite "PR 2 (shell/bodies)" and "PR 5 (widgetbook)"; the current numbers are PR 3 (gallery and registry) and PR 5b/PR 6 (shell, footer, bodies). message-branching `tasks.md:7,116,126` cites PR 5 for widgetbook and `MessageActions`; the current numbers are PR 3 and PR 8a.
