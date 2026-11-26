# Element Index

This file lists every element in the [assistant-ui Elements](https://www.assistant-ui.com/elements) catalogue (59: 37 individual rows plus one category row for the 22 Generative elements) with an explicit verdict. It is the standing record of what the `assistant_ui` package adopts, what sibling changes adopt through its seams, and what is deliberately left out.

This file is maintained here, next to the package. Adding an element without adding its row fails review. So does adopting a Decline without first changing its verdict here.

## Verdicts

| Verdict                | Meaning                                                                                         |
| ---------------------- | ----------------------------------------------------------------------------------------------- |
| **Adopt (have)**       | Already implemented in `features/chat`. This change packages and names it.                      |
| **Adopt (scoped)**     | Packaged by this change in a deliberately reduced form; the reason is in the row.               |
| **Adopt via <change>** | Built by the named sibling change on a seam this change provides. The seam is named in the row. |
| **Defer**              | Wanted, but blocked on a named prerequisite.                                                    |
| **Decline**            | Not wanted. The row gives the reason.                                                           |

### Prerequisites

The list keeps the old numbering. Three prerequisites are now owned by siblings.

- **P1: tool structured data.** `ToolOutput.data` is discarded when building `OrchestratorEvent::ToolResult` (`crates/runtime/src/orchestrator/dispatch.rs:283,452`), which carries only a truncated `result` string. **Owned by tool-result-renderers.** It renders through `ToolResultRendererScope` (PR 6), installed by the app as `ToolResultRendererScope(resolve: registry.build)`.
- **P2: client→server return channel.** `ConfirmationCallback::confirm()` is synchronous (`crates/runtime/src/orchestrator/mod.rs:53`), and web turns hardwire `AutoDenyConfirmation` (`crates/web-ui/src/lib.rs:329`, `space_tasks.rs:157`). **Owned by tool-approval-prompts** (the interaction channel). It renders through the tool call entry footer (`TimelineEntryShell.footer`, forwarded by `TimelineEntryView`) and `EntryState.awaitingInput`.
- **P3: agent-composed UI.** **Owned by generative-ui-a2ui** (A2UI protocol and Flutter genui). It renders through its own `SurfaceFrame` hosted in the shell `body` slot and `MessageBubble.extensions`.
- **P4: live per-conversation token usage.** Nothing on the web API reports token usage: `grep -rn "token_usage\|input_tokens" crates/web-ui/src app/lib` is empty. The only token figure in the runtime is the compaction estimate (`crates/runtime/src/compaction.rs:64`). chat-a11y-and-code-blocks adds per-message timing, including tokens per second. That covers part of P4, but not a context fill level.
- **P5: reasoning step boundaries.** `OrchestratorEvent::Thinking(String)` is undelimited (`stream_event.rs:70`), and `ChatMessage.thinkingContent` is a flat `String?`. A step timeline would invent boundaries.

---

## Reasoning (5)

| #   | Element            | Verdict            | Flutter element    | Note                                                                                                           |
| --- | ------------------ | ------------------ | ------------------ | -------------------------------------------------------------------------------------------------------------- |
| 1   | Loading state      | Adopt (have)       | `TurnProgressCard` | Drops its 2 `ref` reads (`turn_progress_card.dart:65,135`) and takes `status`, `showSkip` and `onSkip`. PR 9.  |
| 2   | Thinking indicator | Adopt (have)       | `TurnProgressCard` | Label via `turnStatusLabel` / `stalledTurnStatusLabel` (`turn_status_label.dart`). PR 9.                       |
| 3   | Reasoning panel    | **Adopt (scoped)** | `ThinkingBody`     | The collapsible section with an optional `elapsed` ("Thought for 4s"). No step timeline; that is **P5**. PR 7a. |
| 4   | Streaming text     | Adopt (have)       | `MessageBody`      | `StreamMarkdown` / `SmoothMarkdown` via `ChatMarkdownScope`. PR 8b.                                            |
| 5   | Typing indicator   | Adopt (have)       | `TypingIndicator`  | From `typing_dots.dart`. PR 8b.                                                                                |

## Messages (5)

| #   | Element               | Verdict                             | Flutter element                                               | Note                                                                                                                                                                       |
| --- | --------------------- | ----------------------------------- | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 6   | Message pair          | Adopt (have)                        | `MessageBubble`                                               | From `message_bubble.dart`. Media and context menus come through slots. PR 8b.                                                                                             |
| 7   | Message branches      | **Adopt via message-branching**     | `BranchPicker` (sibling) in `MessageActions.leading`          | Edit and resend, regenerate, branch navigation. The data model is that change's (ADR-0018).                                                                                |
| 8   | Message actions       | Adopt (have)                        | `MessageActions`, `MessageActionButton`                       | From `MetaActionRow` / `MetaActionButton`. The app decides which actions apply. Rate is added by **message-feedback**; edit and regenerate by **message-branching**. PR 8a.|
| 9   | Follow-up suggestions | **Adopt via follow-up-suggestions** | `ChatThreadLayout.aboveComposer`, `ThreadEmptyState.starters` | Chips after a turn, plus per-Persona starter prompts.                                                                                                                      |
| 10  | Error state           | Adopt (have)                        | `ThreadErrorBanner`                                           | From the inline banner at `chat_screen.dart:415-456` (dismiss). Message-level retry stays on `MessageActions`. PR 9.                                                       |

## Tool use (4)

| #   | Element        | Verdict                             | Flutter element                           | Note                                                                                                                                                |
| --- | -------------- | ----------------------------------- | ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| 11  | Tool call      | Adopt (have)                        | `ToolCallBody`, `GenericToolResultView`   | Generic fallback. Per-tool views plug into `ToolResultRendererScope`. PR 6.                                                                         |
| 12  | Tool timeline  | Adopt (have)                        | `TimelineEntryShell`, `TimelineEntryView` | PR 5b and PR 7c.                                                                                                                                    |
| 13  | Terminal block | **Adopt via tool-result-renderers** | `TerminalResultView` (sibling)            | `terminal` contract over `bash` `.data`, resolved through `ToolResultRendererScope`.                                                                |
| 14  | Code diff      | **Adopt via tool-result-renderers** | `CodeDiffResultView` (sibling)            | `code_diff` contract; that change makes `file-edit` / `file-write` populate `.data`. Highlighting comes from chat-a11y-and-code-blocks `CodeBlock`. |

## Knowledge (4)

| #   | Element          | Verdict                             | Flutter element                                        | Note                                                                                                             |
| --- | ---------------- | ----------------------------------- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| 15  | Web search       | **Adopt via tool-result-renderers** | `SourcesResultView` (sibling)                          | `sources` contract over `web-search` `.data`.                                                                    |
| 16  | Sources          | **Adopt via tool-result-renderers** | `SourcesResultView`, `LinkPreviewResultView` (sibling) | `sources` and `link_preview` contracts; that change makes `web-fetch` populate `.data`.                          |
| 17  | Inline citation  | **Defer: P1 + citation markers**    | —                                                      | Also needs the model to emit citation markers in message text. A candidate for `MessageBubble.extensions` later. |
| 18  | Image generation | Adopt (have)                        | `MessageBubble.attachments` slot                       | The app supplies `MessageAttachments` (I/O: `http`, `cached_network_image`, share). PR 8b.                       |

## Structured output (2)

| #   | Element       | Verdict                             | Flutter element                 | Note                                                                       |
| --- | ------------- | ----------------------------------- | ------------------------------- | -------------------------------------------------------------------------- |
| 19  | Data table    | **Adopt via tool-result-renderers** | `DataTableResultView` (sibling) | Generic `data_table` contract; first producers `list-tasks` and `process`. |
| 20  | Number ticker | **Decline**                         | —                               | Animation with nothing to animate.                                         |

## Agents (6)

| #   | Element             | Verdict                                         | Flutter element                                                                     | Note                                                                                                         |
| --- | ------------------- | ----------------------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| 21  | Agent plan          | **Adopt via tool-result-renderers** (read-only) | `PlanResultView` (sibling)                                                          | `plan` contract, read-only, reachable via MCP `structuredContent`. Interactive checkboxes stay out of scope. |
| 22  | Subagent list       | Adopt (have)                                    | `SubagentBody`                                                                      | Inner tool calls stay compact rows (name and status), as today. PR 7b.                                                |
| 23  | Agent status        | Adopt (have)                                    | `TurnProgressCard`                                                                  | PR 9.                                                                                                        |
| 24  | Approval card       | **Adopt via tool-approval-prompts**             | Tool call entry footer (`TimelineEntryView.footer` → `TimelineEntryShell.footer`), `EntryState.awaitingInput` | `ApprovalCard` in the footer of the matching tool call entry; ask-the-user `QuestionCard` in the footer of the ask-user tool call. `TimelineEntryType.interaction` only when the provider gives no `tool_call_id`. Over the interaction channel (ADR-0017). |
| 25  | Recommendation card | **Adopt via generative-ui-a2ui**                | `SurfaceFrame` (sibling)                                                            | An A2UI surface answered over the interaction channel, not a hand-built widget.                              |
| 26  | Artifact card       | **Decline**                                     | —                                                                                   | The runtime has no versioned-artifact concept. That is a product decision, not a UI one.                     |

## Composer (7)

| #   | Element              | Verdict                | Flutter element       | Note                                                                                                                                        |
| --- | -------------------- | ---------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| 27  | Composer             | Adopt (have)           | `Composer`            | From `ChatInputRow` (`chat_input_row.dart`). `inputBuilder` and `voiceButton` slots. PR 11a.                                                |
| 28  | Slash commands       | Adopt (have)           | `SlashCommandMenu`    | From `CommandAutocompletePopup`. PR 11a.                                                                                                    |
| 29  | Mentions             | **Decline**            | —                     | There is no mentionable-entity concept (`grep -rni mention app/lib crates/web-ui/src` is empty).                                            |
| 30  | Attachments          | Adopt (have)           | `AttachmentTray`      | Presentation only. `file_picker` and `desktop_drop` stay in the app. PR 11b.                                                                |
| 31  | Model picker         | **Decline** (composer) | —                     | Persona switching lives in the nav bar. message-branching's "regenerate with another model" is a per-message action, not a composer picker. |
| 32  | Voice                | Adopt (have)           | `ComposerVoiceButton` | Presentation half. The `record` driver stays in the app. PR 11b.                                                                            |
| 33  | Context (token ring) | **Defer: P4**          | —                     | Needs live usage on the stream.                                                                                                             |

## Thread (4)

| #   | Element       | Verdict      | Flutter element        | Note                                                                                                                   |
| --- | ------------- | ------------ | ---------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| 34  | Chat panel    | Adopt (have) | `ChatThreadLayout`     | Banners, thread, `aboveComposer`, composer. PR 10.                                                                     |
| 35  | Empty state   | Adopt (have) | `ThreadEmptyState`     | From `empty_chat.dart`. The `starters` slot is filled by follow-up-suggestions. PR 9.                                  |
| 36  | Thread list   | Adopt (have) | `ConversationListView` | From `ConversationList`. Rename, search and pin are added by conversation-list-management. PR 12.                      |
| 37  | Scroll anchor | Adopt (have) | `ThreadViewport`       | `scroll_to_bottom_button` already exists (`chat_screen.dart:565`, spec `chat-auto-scroll`). Packaged unchanged. PR 10. |

## Generative (22)

Stay card · table reservation · order tracking · flight tracker · portfolio overview · event creation · event viewing · weather · ride status · email composer · shopping cart · playlist · channel message · purchase receipt · area chart · line chart · bar chart · player card · conference session · confirmation dialog · task creation · software purchase

| Verdict                                             | Note                                                                                                                                                                                                                                                    |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Adopt via generative-ui-a2ui (category, all 22)** | Adopted as a category through the A2UI protocol and the genui catalog, hosted in the sibling's `SurfaceFrame` via the shell `body` slot / `MessageBubble.extensions`. None of these is ever built as an individual hand-written widget in this package. |

---

## Package-only elements (no catalogue row)

`GenericToolResultView`, `ToolStatusIcon`, `MessageActionButton`, `TimelineEntryView` and the scopes (`ThreadDensityScope`, `ChatMarkdownScope`, `ToolResultRendererScope`) are seams or primitives with no assistant-ui counterpart. They are in `kAssistantUiElements` and the gallery, but they have no verdict row.

## Tally

| Verdict                                       | Count  |
| --------------------------------------------- | ------ |
| Adopt (have), packaged by this change         | 20     |
| Adopt (scoped)                                | 1      |
| Adopt via sibling change, individual elements | 10     |
| Adopt via generative-ui-a2ui, category        | 22     |
| Defer                                         | 2      |
| Decline                                       | 4      |
| **Total**                                     | **59** |

The tally is derived from the rows above. Recompute it whenever a verdict changes. Task 13.5 reconciles it before archive, and each sibling reconciles its own rows when it archives.
