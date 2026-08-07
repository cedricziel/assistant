# Element Index

Every element in the [assistant-ui Elements](https://www.assistant-ui.com/elements)
catalogue (59), with an explicit verdict. This file is the living record of what
the `assistant_ui` package adopts and what it deliberately does not.

**On PR 1 this file moves to `app/packages/assistant_ui/ELEMENTS.md`** and is
maintained there. Adding an element to the package without adding its row is a
review failure; so is adopting something previously marked Decline without
changing its verdict here first.

## Verdicts

| Verdict          | Meaning                                                                    |
| ---------------- | -------------------------------------------------------------------------- |
| **Adopt**        | In scope for `chat-element-library`.                                       |
| **Adopt (have)** | Already implemented in `features/chat`; this change packages and names it. |
| **Defer**        | Wanted, blocked on a named prerequisite.                                   |
| **Decline**      | Not wanted. Reason given.                                                  |

Three prerequisites are referenced repeatedly:

- **P1 — tool structured data.** `ToolOutput.data` exists on every tool but is
  dropped at `OrchestratorEvent::ToolResult`, which carries only a truncated
  `result: Option<String>`. Six builtins (`bash`, `process`, `web_search`,
  `file_glob`, `agent_spawn`, `voice_response`) already populate shapes that map
  directly onto elements below.
- **P2 — client→server return channel.** `ConfirmationCallback::confirm() -> bool`
  is synchronous and a turn cannot suspend. `crates/web-ui/src/lib.rs:530`
  hardwires `AutoDenyConfirmation`, so every gated tool is silently denied for
  every web user. This is a live defect, tracked separately.
- **P3 — agent-composed block vocabulary.** A closed, versioned set of semantic
  blocks a tool can return, degrading to text for the CLI/Slack/Matrix/Signal
  interfaces. Depends on P1 and P2.

---

## Reasoning (5)

| #   | Element            | Verdict      | Flutter element    | Note                                                                                    |
| --- | ------------------ | ------------ | ------------------ | --------------------------------------------------------------------------------------- |
| 1   | Loading state      | Adopt (have) | `TurnProgressCard` | Drops its 3 `ref` reads.                                                                |
| 2   | Thinking indicator | Adopt (have) | `TurnStatusLabel`  |                                                                                         |
| 3   | Reasoning panel    | **Adopt**    | `ReasoningPanel`   | New. Today's thinking render is a flat blob; this is a step sequence with elapsed time. |
| 4   | Streaming text     | Adopt (have) | `MessageBody`      | Via `flutter_smooth_markdown`.                                                          |
| 5   | Typing indicator   | Adopt (have) | `TypingIndicator`  | Extracted from `_Dot` in `chat_screen.dart`.                                            |

## Messages (5)

| #   | Element               | Verdict      | Flutter element     | Note                                                                                             |
| --- | --------------------- | ------------ | ------------------- | ------------------------------------------------------------------------------------------------ |
| 6   | Message pair          | Adopt (have) | `MessageBody`       | Extracted from `_MessageBubble`.                                                                 |
| 7   | Message branches      | **Decline**  | —                   | No branch/regeneration concept server-side. Would need a conversation-tree model, not a widget.  |
| 8   | Message actions       | Adopt (have) | `MessageActions`    | From `_MetaActionRow`. Copy + read-aloud only; rate and regenerate have no endpoint.             |
| 9   | Follow-up suggestions | **Decline**  | —                   | Nothing produces suggestions. Would need either P3 or a client heuristic; neither is wanted now. |
| 10  | Error state           | **Adopt**    | `ThreadErrorBanner` | Non-modal banner with retry over the existing send path.                                         |

## Tool use (4)

| #   | Element        | Verdict        | Flutter element                 | Note                                                                                          |
| --- | -------------- | -------------- | ------------------------------- | --------------------------------------------------------------------------------------------- |
| 11  | Tool call      | Adopt (have)   | `ToolCallChip` + `ToolCallBody` | The generic fallback renderer.                                                                |
| 12  | Tool timeline  | Adopt (have)   | `TimelineEntryShell`            | Session view is the shell over a list.                                                        |
| 13  | Terminal block | **Defer — P1** | —                               | `bash` already emits `{exit_code, stdout, stderr}`. Renderer is trivial once the field ships. |
| 14  | Code diff      | **Defer — P1** | —                               | Also needs `file_edit` to populate `.data`, which it currently does not.                      |

## Knowledge (4)

| #   | Element          | Verdict        | Flutter element        | Note                                                                   |
| --- | ---------------- | -------------- | ---------------------- | ---------------------------------------------------------------------- |
| 15  | Web search       | **Defer — P1** | —                      | `web_search` already emits `{results: [{title, url, snippet}]}`.       |
| 16  | Sources          | **Defer — P1** | —                      | Same payload as #15; also needs `web_fetch` to populate `.data`.       |
| 17  | Inline citation  | **Defer — P1** | —                      | Additionally needs the model to emit citation markers in message text. |
| 18  | Image generation | Adopt (have)   | `AttachmentThumbnails` | Attachments already stream and render.                                 |

## Structured output (2)

| #   | Element       | Verdict        | Flutter element | Note                                                                                      |
| --- | ------------- | -------------- | --------------- | ----------------------------------------------------------------------------------------- |
| 19  | Data table    | **Defer — P1** | —               | Needs a generic `{columns, rows}` shape; no builtin emits one yet.                        |
| 20  | Number ticker | **Decline**    | —               | Animation flourish with no data to animate. Revisit only if a metric lands in the stream. |

## Agents (6)

| #   | Element             | Verdict           | Flutter element    | Note                                                                                                              |
| --- | ------------------- | ----------------- | ------------------ | ----------------------------------------------------------------------------------------------------------------- |
| 21  | Agent plan          | **Decline**       | —                  | Read-only variant needs P1; interactive checkboxes need P2. No plan concept exists in the runtime.                |
| 22  | Subagent list       | Adopt (have)      | `SubagentBody`     | `SubagentStarted`/`SubagentCompleted` events already stream.                                                      |
| 23  | Agent status        | Adopt (have)      | `TurnProgressCard` |                                                                                                                   |
| 24  | Approval card       | **Defer — P2**    | —                  | ⚠ The server already gates on `requires_confirmation`; the web client cannot answer. Highest-value deferred item. |
| 25  | Recommendation card | **Defer — P2/P3** | —                  | Accept/reject is a return channel.                                                                                |
| 26  | Artifact card       | **Decline**       | —                  | No versioned-artifact concept in the runtime. Product decision, not a UI one.                                     |

## Composer (7)

| #   | Element              | Verdict        | Flutter element       | Note                                                                                                                     |
| --- | -------------------- | -------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| 27  | Composer             | Adopt (have)   | `Composer`            | From `_InputRow` — already 13 constructor params, all callbacks out, zero `ref`.                                         |
| 28  | Slash commands       | Adopt (have)   | `SlashCommandMenu`    | From `command_autocomplete.dart`. Command list supplied by the app.                                                      |
| 29  | Mentions             | **Decline**    | —                     | No mentionable-entity concept: `grep -rn "mention"` over `app/lib` and `crates/web-ui` returns nothing.                  |
| 30  | Attachments          | Adopt (have)   | `AttachmentTray`      | Presentation only — `file_picker` / `desktop_drop` I/O stays in the app.                                                 |
| 31  | Model picker         | **Decline**    | —                     | Persona switching already exists in the nav bar. Moving it into the composer is a UX change, not an extraction.          |
| 32  | Voice                | Adopt (have)   | `ComposerVoiceButton` | Presentation split from I/O: mic affordance, countdown and stop live in the package; `record` stays in the app.          |
| 33  | Context (token ring) | **Defer — P1** | —                     | Token counts exist only as analytics aggregates (`token_usage_over_time`). No per-conversation live usage in the stream. |

## Thread (4)

| #   | Element       | Verdict      | Flutter element      | Note                                                                          |
| --- | ------------- | ------------ | -------------------- | ----------------------------------------------------------------------------- |
| 34  | Chat panel    | Adopt (have) | `ConversationThread` |                                                                               |
| 35  | Empty state   | Adopt (have) | `ThreadEmptyState`   | From `_EmptyChat`.                                                            |
| 36  | Thread list   | Adopt (have) | `ConversationList`   | Drops its 5 `ref` reads; the app supplies the list.                           |
| 37  | Scroll anchor | **Adopt**    | `ThreadViewport`     | New. Today's `_atBottom` logic has no recovery affordance when scrolled away. |

## Generative (22)

Stay card · table reservation · order tracking · flight tracker · portfolio
overview · event creation · event viewing · weather · ride status · email
composer · shopping cart · playlist · channel message · purchase receipt · area
chart · line chart · bar chart · player card · conference session · confirmation
dialog · task creation · software purchase

| Verdict              | Note                                                                                                                                                                                                                                                                                                                                            |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Decline (all 22)** | These are demonstrations of _generative UI_, not chat chrome: the agent composes a bespoke surface per response. That is P3, and P3 depends on P1 and P2. Adopting any one of them as a hand-built widget would encode a domain this product does not have. Revisit as a category once a block vocabulary exists — never as individual widgets. |

---

## Tally

|                                        | Count  |
| -------------------------------------- | ------ |
| Adopt (new)                            | 3      |
| Adopt (have — packaged by this change) | 15     |
| Defer — P1 (tool structured data)      | 7      |
| Defer — P2 (return channel)            | 2      |
| Decline                                | 32     |
| **Total**                              | **59** |
