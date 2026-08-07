# Tasks — chat-element-library

Sequenced as four stacked PRs. Each PR is independently reviewable and leaves
`main` shippable. PR boundaries are marked; phases within a PR are ≤ 2h chunks.

---

## PR 1 — package scaffold + models

### 1. Package boundary (TDD red first)

- [ ] 1.1 Add `app/test/unit/assistant_ui_boundary_test.dart`: read `app/packages/assistant_ui/pubspec.yaml` and assert its `dependencies` contain neither `flutter_riverpod` nor `assistant_api`. Run `flutter test` — confirm RED (the package does not exist).
- [ ] 1.2 Scaffold `app/packages/assistant_ui/` with `pubspec.yaml` (deps: `flutter`, `flutter_smooth_markdown` only), `lib/assistant_ui.dart` barrel, and `lib/src/`. Add the path dependency to `app/pubspec.yaml`. Confirm 1.1 GREEN.
- [ ] 1.3 Write `app/packages/assistant_ui/README.md`: the purity rule (no state management, **no I/O**), why the package boundary exists rather than a folder, and the constructor-slots-plus-density-scope composition rule from design.md.
- [ ] 1.4 Move `openspec/changes/chat-element-library/elements.md` to `app/packages/assistant_ui/ELEMENTS.md`. Link it from the README as the standing index. From here on, every task that adopts an element also updates its row.

### 2. Move render-facing models

- [ ] 2.1 Move `ChatMessage`, `ChatAttachment`, `ToolCallRecord`, `MessageStatus`, `ToolCallStatus`, `TimelineEntryType` from `chat_provider.dart` into `assistant_ui/lib/src/models/`. Export from the barrel. Do **not** change their shape — no sealing, no immutability (design.md).
- [ ] 2.2 Re-point `chat_provider.dart` and all existing importers at the package. Move the corresponding tests from `app/test/unit/chat/` into `app/packages/assistant_ui/test/` where they test only the models.
- [ ] 2.3 `flutter analyze --fatal-infos` → 0 issues; `flutter test` → all previously passing tests still pass.
- [ ] 2.4 PR: `refactor(app): extract assistant_ui package with render-facing models`.

---

## PR 2 — shell/body split + density scope

### 3. Density scope (TDD red first)

- [ ] 3.1 Add `app/packages/assistant_ui/test/thread_density_scope_test.dart`: assert a descendant resolves `compact` from an enclosing scope, and that rebuilding the scope with `expanded` rebuilds the descendant. Confirm RED.
- [ ] 3.2 Implement `ThreadDensityScope` (`InheritedWidget`) with `TimelineDensity.fromWidth`. Confirm GREEN.

### 4. Timeline entry shell (TDD red first)

- [ ] 4.1 Add `app/packages/assistant_ui/test/timeline_entry_shell_test.dart` covering the four spec scenarios: active→complete auto-collapses; user-pinned survives complete and stale; `disableAnimations` collapses without a timer; tapping toggles and pins. Confirm RED.
- [ ] 4.2 Implement `TimelineEntryShell` — expansion, `_userPinned`, auto-collapse timer, reduced motion, `EntryState`, header row, `body` slot. Lift this logic out of `StreamingTimelineEntry`; do not leave a copy behind. Confirm GREEN.

### 5. Entry bodies

- [ ] 5.1 Add golden tests for `MessageBody`, `ThinkingBody`, `ToolCallBody`, `SubagentBody`, `CommandBody` — one file per body under `app/packages/assistant_ui/test/bodies/`. Confirm RED.
- [ ] 5.2 Extract each body as a `StatelessWidget` in its own file from `StreamingTimelineEntry`'s render branches. Each reads only the fields for its own `TimelineEntryType`. Confirm GREEN.
- [ ] 5.3 Delete `ChatTimelineSection` — its collapse behaviour is now `TimelineEntryShell` and its rendering is now the bodies. Migrate its existing tests onto the shell/body pair. No compatibility shim.
- [ ] 5.4 Move `ToolCallChip`, `TurnStatusLabel`, `AudioPlayerWidget`, `CommandEventTile`, `SvgBuilder` into the package with their existing tests. These are already Riverpod-free.

### 6. Close the Riverpod leak

- [ ] 6.1 Add a widget test pumping `TurnProgressCard` with literal arguments and **no** `ProviderScope`. Confirm RED.
- [ ] 6.2 Convert `TurnProgressCard` to take turn status by constructor parameter; move it into the package; have `ChatScreen` supply the value via `ref.watch`. Confirm GREEN.
- [ ] 6.3 `flutter analyze --fatal-infos` → 0; `flutter test` → green.
- [ ] 6.4 PR: `refactor(app): split timeline entry shell from entry bodies`.

---

## PR 3 — new elements

### 7. Reasoning panel (TDD red first)

- [ ] 7.1 Add `app/packages/assistant_ui/test/reasoning_panel_test.dart` for the three spec scenarios: multiple steps render separately; elapsed time shows while active; empty reasoning renders nothing. Confirm RED.
- [ ] 7.2 Implement `ReasoningPanel` as a step sequence with per-step elapsed time, replacing the flat thinking blob in `ThinkingBody`. Confirm GREEN.

### 8. Thread viewport scroll anchor (TDD red first)

- [ ] 8.1 Add `app/packages/assistant_ui/test/thread_viewport_test.dart` for the three spec scenarios: at-bottom stays pinned; scrolled-away offset does not change; recovery pill appears, returns to bottom, then hides. Confirm RED.
- [ ] 8.2 Implement `ThreadViewport` owning `_atBottom` / scroll-to-bottom / recovery pill. Remove the equivalent logic and the `ref.listen(chatProvider)` scroll hook from `_ChatScreenState`. Confirm GREEN.
- [ ] 8.3 Update the `ReasoningPanel` and `ThreadViewport` rows in `ELEMENTS.md` from Adopt to Adopt (have).
- [ ] 8.4 PR: `feat(app): reasoning panel and thread scroll anchor`.

---

## PR 4 — composer

### 9. Composer extraction (TDD red first)

- [ ] 9.1 Add `app/packages/assistant_ui/test/composer_test.dart`: pump `Composer` with literal arguments and no `ProviderScope`; assert activating send fires `onSend` and that the element makes no API call. Confirm RED.
- [ ] 9.2 Move `_InputRow` from `chat_screen.dart` into the package as `Composer`, unchanged in shape — it already takes 13 constructor parameters with every interaction as a callback. Move its existing tests. Confirm GREEN.
- [ ] 9.3 Move `command_autocomplete.dart` into the package as `SlashCommandMenu`, with the command list supplied by the caller. Add a test that filtering narrows the list and that selection fires `onSelected` without dispatching.

### 10. Attachment tray and voice split

- [ ] 10.1 Add a widget test for `AttachmentTray`: two pending attachments, activating remove on the first fires `onRemove(0)` and the tray does not mutate the list. Confirm RED.
- [ ] 10.2 Extract `AttachmentTray` from `_MessageBubble._attachmentThumbnails` and the composer's pending-attachment strip. `file_picker` / `desktop_drop` I/O stays in `ChatScreen`. Confirm GREEN.
- [ ] 10.3 Add tests for `ComposerVoiceButton`: with `isRecording` false, activating fires `onStart` and starts no recorder; with `isRecording` true plus an elapsed duration, the countdown and stop affordance render. Confirm RED.
- [ ] 10.4 Split `VoiceRecorderButton` in two — presentation (`ComposerVoiceButton`, in the package, driven by `isRecording`/`elapsed`, emitting `onStart`/`onStop`) and the `record`-backed driver (stays in `features/chat`, keeps its 2 `ref` reads). Confirm GREEN.
- [ ] 10.5 Move `QueuedMessageBubble` into the package, dropping its 2 `ref` reads in favour of parameters.
- [ ] 10.6 Update the seven Composer rows in `ELEMENTS.md`. Confirm the two Decline rows (mentions, model picker) and the one Defer row (context ring) still read accurately.
- [ ] 10.7 `flutter analyze --fatal-infos` → 0; `flutter test` → green.
- [ ] 10.8 PR: `refactor(app): extract composer elements into assistant_ui`.

---

## PR 5 — gallery, adapter, ship

### 11. Widgetbook

- [ ] 11.1 Scaffold `app/widgetbook/` as a Flutter app depending on `assistant_ui` only. Verify it is excluded from `flutter build web` for the embedded SPA.
- [ ] 11.2 Register a use-case for every exported element, composer included. For timeline entries, expose the full `EntryState` × `TimelineDensity` matrix as knobs; for `ComposerVoiceButton`, expose `isRecording` and elapsed duration.
- [ ] 11.3 Add a test asserting every `assistant_ui` export has at least one registered gallery entry, so the gallery cannot silently fall behind.
- [ ] 11.4 Add a test asserting every `assistant_ui` export carries an Adopt or Adopt (have) row in `ELEMENTS.md`, so the index cannot silently fall behind either.

### 12. Chat screen becomes an adapter

- [ ] 12.1 Rewrite `chat_screen.dart` to keep chrome, layout, I/O (`record`, `file_picker`, `desktop_drop`) and every `ref.watch`, delegating both the conversation surface and the composer to package elements. Target under ~300 lines per the project convention.
- [ ] 12.2 Run the pre-existing chat widget test suite with updated imports — every previously passing assertion must still pass.

### 13. Baselines and ship

- [ ] 13.1 Run the Playwright visual suite. `ReasoningPanel` and the scroll-anchor pill are intentional new UI — update the screenshot baselines and record the visual diff for the PR body (see `.claude/skills/e2e-testing`).
- [ ] 13.2 Reconcile `ELEMENTS.md` end to end: 59 rows, one verdict each, every Defer naming its prerequisite, tally matching the rows.
- [ ] 13.3 `make lint-flutter && make test-flutter` → green.
- [ ] 13.4 `make lint && make format && make test` → green.
- [ ] 13.5 Manual smoke: stream a turn with thinking, a tool call and a subagent; scroll away mid-stream and confirm the recovery pill; record a voice message; drag-drop an attachment; run a slash command. Confirm auto-collapse and pinning behave as before.
- [ ] 13.6 PR: `feat(app): element gallery and chat screen adapter`.
- [ ] 13.7 Archive: `openspec archive chat-element-library`.
