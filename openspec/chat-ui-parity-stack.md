# Chat UI parity: execution stack

Order of the PRs across the ten chat-UI-parity OpenSpec changes. Each change's `tasks.md` is authoritative for its own PRs. Product decisions are in [chat-ui-parity-decisions.md](chat-ui-parity-decisions.md).

Waves are computed from the `Depends on:` lines in every `tasks.md`: a PR's wave is one more than the latest wave among its dependencies. Every PR in a wave can start once the earlier waves have merged, each in its own worktree from `main`. A PR may also start earlier, stacked on the branches of its unmerged dependencies. Regenerate `openapi.json`, `assistant_api` and Playwright baselines after a rebase; never merge them by hand.

| Change | Abbrev. | PRs | Server | Client | Both |
|---|---|---|---|---|---|
| chat-element-library | CEL | 20 | 0 | 20 | 0 |
| chat-stop-cancels-turn | STOP | 9 | 5 | 4 | 0 |
| tool-result-renderers | TRR | 22 | 13 | 8 | 1 |
| tool-approval-prompts | TAP | 29 | 16 | 10 | 2 |
| message-branching | MB | 24 | 16 | 7 | 1 |
| generative-ui-a2ui | A2UI | 23 | 14 | 7 | 1 |
| message-feedback | MF | 8 | 3 | 3 | 2 |
| follow-up-suggestions | FUS | 12 | 5 | 6 | 0 |
| conversation-list-management | CLM | 18 | 10 | 7 | 1 |
| chat-a11y-and-code-blocks | A11Y | 20 | 5 | 14 | 0 |
| **Total** | | **185** | | | |

## Pre-allocated space.db migrations (register in both lists in lib.rs):
- 046_message_tree (MB1)
- 047_interactions (TAP3)
- 048_tool_permission_rules (TAP4)
- 049_ui_surfaces (A2UI4)
- 050_message_feedback (MF1)
- 051_persona_starter_prompts (FUS4)
- 052_conversation_pinned_at (CLM3)
- 053_message_timings (A11Y S1)
- 054_add_tool_result_metadata_to_messages (TRR S3)
- 055_conversation_summaries (MB3)
- 056_messages_fts (CLM5)
- 057_message_traceparent (MF3)
- 058_conversation_archived_at (CLM8)
- 059_push_subscription_user (TAP12)
- 060_message_model (MB8)
- 061_message_finish_reason (STOP 2a)


## Wave 1 (32 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A11Y C10b | client | feat(app): "Show response timing" preference and settings switch | none |
| A11Y C2a | client | feat(app): ChatAnnouncer policy and announcement sink | none |
| A11Y S1 | server | feat(storage): add message timing store | none |
| A11Y S2a | server | test(llm-provider): timed streaming scripts for ScriptedLlmProvider | none |
| A2UI 1 | docs | docs(adr): ADR-0019 generative UI via A2UI | none |
| A2UI 2 | server | feat(a2ui): A2UI v1.0 types and assistant catalog | none |
| A2UI 4 | server | feat(storage): SurfaceStore with ui_surfaces table | none |
| CEL 1 | client | build(app): scaffold assistant_ui package with purity boundary | none |
| CEL 2 | client | refactor(app): delete dead ToolCallChip and ChatTimelineSection | none |
| CLM 1 | server | refactor(web-ui): single ConversationSummary projection | none |
| CLM 10 | client | fix(app): new chats start untitled so auto-titling runs | none |
| CLM 2 | server | fix(runtime): auto-title only writes unlocked conversations | none |
| CLM 3 | server | feat(storage): conversation pinning and metadata patch | none |
| CLM 6a | server | feat(storage): search query, folding and highlight segment helpers | none |
| FUS 1 | server | feat(runtime): follow-up suggestion generator and [suggestions] config | none |
| FUS 2 | server | fix(web-ui): run replay and turn status outlive the done frame | none |
| FUS 4 | server | feat(storage): persona starter_prompts column | none |
| FUS 7a | client | feat(app): parse the suggestions stream event | none |
| MB 0 | server | docs(adr): ADR-0018 conversation message tree | none |
| MB 1 | server | feat(storage): persist messages as a tree with an active leaf | none |
| MB 5 | server | refactor(web-ui): extract the streamed turn runner from send_message | none |
| MF 1 | server | feat(storage): message feedback store | none |
| STOP 1 | server | fix(runtime): honour cancel for turns not yet claimed by a worker | none |
| TAP 1 | docs | docs(adr): ADR-0017 turn interactions and tool approvals | none |
| TAP 3 | server | feat(storage): interactions store | none |
| TAP 4 | server | feat(storage): tool permission rules store | none |
| TAP 5a | server | feat(core): suggest permission rules for approvals | none |
| TAP 5b | server | feat(core): saved-rule stages in the permission pipeline | none |
| TAP 6a | server | refactor(runtime)!: async typed ConfirmationCallback, no AutoDenyConfirmation | none |
| TRR S1a | server | feat(core): tool render data contract harness with terminal and sources | none |
| TRR S2a | server | feat(runtime): carry tool data and truthful status on ToolResult events | none |
| TRR S9 | server | feat(mcp-client): forward MCP structuredContent as tool data | none |

## Wave 2 (25 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A11Y C3 | client | feat(app): add highlighter and contrast-checked code themes to assistant_ui | CEL 1 |
| A11Y S2b | server | refactor(runtime): carry TurnTiming on TurnResult and across the bus | A11Y S1 |
| A2UI 10 | server | fix(web-ui): keep persisting run frames after the client disconnects | MB 5 |
| A2UI 3 | server | feat(a2ui): structural validation, limits, regex safety and URL policy | A2UI 2 |
| A2UI 6 | server | feat(runtime): SurfaceUpdate event and projections | A2UI 2 |
| CEL 1b | client | test(app): characterization goldens of the current chat widgets | CEL 1 |
| CEL 3 | client | build(app): add widgetbook gallery and element registry | CEL 1 |
| CLM 4 | server | feat(web-ui): partial conversation PATCH with pin and validation | CLM 1, CLM 3 |
| FUS 3 | server | feat(web-ui): suggestion phase on the shared turn runner | FUS 1, FUS 2, MB 5 |
| FUS 5 | server | feat(web-ui): PATCH /api/personas/{id} with starter_prompts | FUS 4 |
| MB 2 | server | feat(storage): branch insert and navigation on ConversationStore | MB 1 |
| MB 2b | server | feat(storage): share attachments across message versions | MB 1 |
| MB 3a | server | feat(runtime): conversation summaries store and summary-applied history builder | MB 1 |
| MF 2a | both | feat(web-ui): message feedback PUT and DELETE endpoints | MF 1 |
| MF 2b | both | feat(web-ui): project caller feedback into conversation history | MF 1 |
| STOP 1b | server | fix(web-ui): end the stream at once when a turn is cancelled before it starts | MB 5, STOP 1 |
| STOP 2a | server | feat(storage): persist a finish_reason on messages | MB 1 |
| STOP 4 | client | fix(app): reconcile cancelled turns from the agent_error frame | CEL 2 |
| TAP 10 | server | feat(web-ui): list and revoke saved tool permission rules | TAP 4 |
| TAP 2 | server | fix(runtime): heartbeat bus claims while a turn runs | STOP 1 |
| TAP 6b | server | feat(runtime): explicit interactive submit option and run id on turns | STOP 1, TAP 6a |
| TAP 7a | server | feat(runtime): interaction broker and kind registry | TAP 3, TAP 5b, TAP 6a |
| TRR S1b | server | feat(core): remaining tool render data contracts | TRR S1a |
| TRR S2b | server | fix(web-ui): document tool data on SseToolResultEvent and verify replay | TRR S2a |
| TRR S5 | server | feat(tool-executor): terminal and sources render data for bash and web-search | TRR S1a |

## Wave 3 (20 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A11Y C4 | client | feat(app): CodeBlock element | A11Y C3, CEL 3 |
| A2UI 5 | server | feat(runtime): drop rich-client tools outside interactive web turns | TAP 6a, TAP 6b |
| CEL 4 | client | refactor(app): move render models into assistant_ui | CEL 1, STOP 4 |
| CLM 12 | client | feat(app): rename dialog and list metadata actions | CLM 4 |
| FUS 6 | client | feat(app): SuggestionChips and StarterPrompts elements | CEL 1, CEL 3 |
| FUS 8a | client | feat(app): persona starter prompts editor | FUS 5 |
| MB 3b | server | refactor(runtime): non-destructive context compaction | MB 3a |
| MB 4a | server | refactor(runtime): SubmitTurn options struct | TAP 6a, TAP 6b |
| MB 9 | client | feat(app): BranchPicker and MessageEditor elements | CEL 1, CEL 3 |
| MF 7a | client | feat(app): message feedback model and controller | MF 2a, MF 2b |
| STOP 2 | server | feat(web-ui): persist the partial reply of a cancelled turn | MB 5, STOP 1b, STOP 2a |
| STOP 5a | client | feat(app): Stop cancels the server turn | STOP 4 |
| TAP 13a | client | feat(app): interaction models and ApprovalCard | CEL 1, CEL 3 |
| TAP 7b | server | feat(runtime): relay run events across processes over the bus | MB 5, TAP 6b |
| TAP 8a | server | feat(web-ui): respond to turn interactions | TAP 7a |
| TRR S10 | server | feat(tool-executor): plan-update built-in producing plan render data | TRR S1b |
| TRR S3 | server | feat(storage): persist tool call id, status and data on tool rows | MB 1, TAP 6a, TAP 6b, TRR S2a |
| TRR S6 | server | feat(tool-executor): link preview and code file data for web-fetch and file-read | TRR S1b |
| TRR S7 | server | feat(tool-executor): code diff data for file-edit and file-write | TRR S1b |
| TRR S8 | server | feat(tool-executor): data table render data for list-tasks and process list | TRR S1b |

## Wave 4 (20 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A11Y C10 | client | feat(app): MessageTiming model and MessageTimingIndicator element | CEL 3, CEL 4 |
| A2UI 7 | server | feat(tool-executor): render-surface handler and [ui] org settings | A2UI 3, A2UI 5 |
| CEL 11c | client | refactor(app): QueuedMessageBubble in assistant_ui | CEL 1b, CEL 3, CEL 4 |
| CEL 12 | client | refactor(app): pure ConversationListView | CEL 1b, CEL 3, CEL 4 |
| CEL 5a | client | feat(app): ThreadDensityScope | CEL 1b, CEL 3, CEL 4 |
| CEL 8a | client | feat(app): MessageActions and MessageActionButton | CEL 1b, CEL 3, CEL 4 |
| CLM 14 | client | feat(app): highlighted search result elements | CEL 3, CEL 4 |
| CLM 5 | server | feat(storage): full-text index over user and assistant messages | MB 3a, MB 3b |
| MB 4b | server | feat(runtime): one orchestrator-owned turn registry | MB 4a |
| MB 4c | server | feat(runtime): edit and regenerate turn anchors with idempotent redelivery | MB 2, MB 2b, MB 4a |
| MB 8a | server | feat(runtime): per-turn model override and recorded model | MB 4a, TRR S3 |
| MF 3 | server | feat(runtime): record LLM span traceparent on assistant replies | MB 1, TRR S3 |
| STOP 3 | server | feat(web-ui): run voice turns on the shared turn runner | MB 5, STOP 2 |
| STOP 5b | client | fix(app): bound every Stop wait and resume the queue | STOP 5a |
| TAP 13b | client | test(app): ApprovalCard goldens, stories and UnknownInteractionCard | TAP 13a |
| TAP 14a | client | feat(app): QuestionCard element | TAP 13a |
| TAP 14b | client | feat(app): PermissionRuleTile element | TAP 13a |
| TAP 7c | server | feat(runtime): interaction stream events and end-of-run cleanup | FUS 3, MB 5, TAP 7a, TAP 7b |
| TRR S3b | server | fix(runtime): keep tool metadata and turn indices when compacting | TRR S3 |
| TRR S4 | server | feat(web-ui): expose tool data in history and fix the per-call join | TRR S3 |

## Wave 5 (16 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A11Y C1 | client | refactor(app): shared layout breakpoint tokens | CEL 5a |
| A11Y C2b | client | feat(app): turn lifecycle events drive chat announcements | A11Y C2a, STOP 4, STOP 5b |
| A11Y S2c | server | feat(runtime): measure turn timing in run_turn_core | A11Y S1, A11Y S2a, A11Y S2b, MF 3 |
| A2UI 7b | server | feat(web-ui): expose the link allowlist on /api/capabilities | A2UI 7 |
| A2UI 8 | server | feat(runtime): emit and persist surfaces from the surface hook | A2UI 4, A2UI 6, A2UI 7, TRR S2a, TRR S2b, TRR S3 |
| CEL 5b | client | feat(app): TimelineEntryShell with footer slot and awaitingInput | CEL 5a |
| CLM 11 | client | feat(app): pinned group and selection in the conversation list | CEL 12, CLM 4 |
| CLM 6b | server | feat(storage): conversation search on ConversationStore | CLM 5, CLM 6a |
| MB 4d | server | feat(runtime): cancel a turn that is waiting for the user before branching | MB 4b, STOP 1 |
| MF 4 | server | feat(runtime): emit gen_ai.evaluation.result for user feedback | MF 2a, MF 3 |
| MF 5 | client | feat(app): FeedbackButtons and FeedbackCommentField elements | CEL 3, CEL 8a |
| STOP 5c | client | feat(app): show Stopped after reload | STOP 2a, STOP 4, STOP 5b |
| TAP 12 | both | feat(web-ui): push the turn owner when an interaction waits | MB 5, TAP 7c |
| TAP 8b | server | feat(web-ui): list interactions and report awaiting_input | FUS 2, TAP 7c, TAP 8a |
| TAP 9 | server | feat(runtime): approve tool calls through interactions, with saved rules | TAP 4, TAP 5a, TAP 5b, TAP 7c, TAP 8a |
| TRR C1 | client | feat(app): carry tool result data from stream and history | CEL 4, TRR S2a, TRR S4 |

## Wave 6 (16 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A11Y S3 | server | feat(web-ui): typed done frame with timing, and timing in history | A11Y S2c, STOP 3 |
| A2UI 12 | server | feat(runtime): surface_action interaction kind for in-turn waits | A2UI 10, A2UI 8, TAP 7a, TAP 7b, TAP 7c, TAP 8a, TAP 8b |
| A2UI 13 | client | feat(assistant_ui): surface model and SurfaceFrame element | CEL 3, CEL 4, CEL 5a, CEL 5b |
| A2UI 9 | server | feat(web-ui): surfaces in conversation history | A2UI 8, TRR S4 |
| CEL 11a | client | refactor(app): Composer and SlashCommandMenu in assistant_ui | CEL 1b, CEL 3, CEL 4, STOP 5a, STOP 5b, STOP 5c |
| CEL 6 | client | feat(app): ToolCallBody with tool result resolver scope | CEL 2, CEL 5b |
| CEL 9 | client | refactor(app): pure TurnProgressCard, error banner and empty state | CEL 1b, CEL 3, CEL 4, STOP 5a, STOP 5b, STOP 5c |
| CLM 13 | client | feat(app): conversation row actions | CLM 11, CLM 12 |
| CLM 7 | server | feat(web-ui): GET /api/conversations/search | CLM 1, CLM 6b |
| CLM 8 | server | feat(storage): archive conversations | CLM 3, CLM 6b, MB 2 |
| FUS 7b | client | feat(app): listen for follow-up suggestions after done | FUS 3, FUS 7a, STOP 5a, STOP 5b, STOP 5c |
| MB 10 | client | feat(app): RegenerateMenuButton and branch entries in buildMessageActions | CEL 5a, CEL 5b, CEL 8a, MB 9 |
| MB 6 | server | feat(web-ui): branch metadata and switch-branch endpoint | CLM 1, MB 2, MB 4b, MB 4d |
| TAP 11 | server | feat(runtime): ask-user tool | TAP 7a, TAP 8a, TAP 9 |
| TAP 15 | client | feat(app): interaction stream events and API calls | TAP 10, TAP 8a, TAP 8b |
| TRR C2 | client | feat(app): tool result renderer registry with compact tap-to-expand | CEL 3, CEL 4, CEL 5a, CEL 5b, TRR S1a |

## Wave 7 (15 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A11Y C11 | client | feat(app): show per-message timing from done frames and history | A11Y C10, A11Y C10b, A11Y S3, CEL 8a |
| A2UI 14 | client | feat(app): A2UI renderer adapter | A2UI 13 |
| CEL 10 | client | feat(app): chat thread layout and viewport | CEL 9 |
| CEL 11b | client | refactor(app): AttachmentTray and ComposerVoiceButton | CEL 11a |
| CEL 7a | client | feat(app): ChatMarkdownScope and ThinkingBody | CEL 6 |
| CLM 15 | client | feat(app): search conversations from the sidebar | CEL 12, CLM 14, CLM 7 |
| CLM 9 | server | feat(web-ui): archive via PATCH and archived listing | CLM 4, CLM 7, CLM 8 |
| FUS 8b | client | feat(app): show Persona starter prompts in the empty chat | CEL 9, FUS 6, FUS 8a |
| MB 11 | client | feat(app): branch-aware history and switch-branch wiring | CEL 4, MB 6, MB 9 |
| MB 7a | server | feat(web-ui): edit and regenerate message endpoints | MB 4c, MB 4d, MB 5, MB 6, TAP 6a, TAP 6b, TAP 7a, TAP 7b, TAP 7c |
| TAP 16 | client | feat(app): approve tool calls in chat | CEL 5a, CEL 5b, TAP 13b, TAP 15 |
| TAP 18a | client | feat(app): tool approvals settings section | TAP 10, TAP 14b, TAP 15 |
| TRR C2b | client | feat(app): JSON tree for structured data in the generic tool view | CEL 6, TRR C2 |
| TRR C6 | client | feat(app): data table result view | TRR C2 |
| TRR C7 | client | feat(app): code file and code diff result views | TRR C2 |

## Wave 8 (11 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A11Y C7 | client | fix(app): label icon-only chat controls and size their targets | CEL 11a, CEL 11b, CEL 9 |
| A2UI 14b | client | feat(app): surface URL policy, regex cap and link confirmation | A2UI 14, A2UI 7b |
| CEL 7b | client | feat(app): SubagentBody and CommandBody | CEL 7a |
| CEL 8b | client | feat(app): MessageBubble, MessageBody and TypingIndicator | CEL 7a, CEL 8a |
| CLM 16 | client | feat(app): archive action and Archived section | CLM 13, CLM 9 |
| FUS 7c | client | feat(app): render follow-up chips above the composer | CEL 10, FUS 6, FUS 7b |
| MB 7b | server | fix(web-ui): anchor-aware cancelled reply for branch turns | MB 7a |
| MB 8b | server | feat(web-ui): list models and regenerate with a model | MB 7a, MB 8a |
| TAP 17 | client | feat(app): ask-user questions, receipts after reload, pending discovery | TAP 11, TAP 14a, TAP 16, TRR S4 |
| TAP 18b | client | feat(app): admins review and revoke members' saved approvals | TAP 18a |
| TAP 19 | client | feat(app): notify on approval requests when backgrounded | TAP 16 |

## Wave 9 (11 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A11Y C5 | client | feat(app): render markdown code fences through CodeBlock | A11Y C2a, A11Y C4, CEL 7a, CEL 8b |
| A11Y C6 | client | feat(app): semantics for message bubbles and timeline headers | CEL 5b, CEL 8b, STOP 4 |
| A11Y C8 | client | feat(app): composer focus order and keyboard flow | A11Y C7, CEL 11a |
| A11Y C9 | client | feat(app): honour reduced motion in thread viewport and typing indicator | CEL 10, CEL 8b |
| A2UI 11 | server | feat(web-ui): post-turn surface actions | A2UI 8, A2UI 9, MB 4a, MB 4b, MB 4c, MB 4d, MB 5, MB 7a, MB 7b |
| CEL 7c | client | refactor(app): TimelineEntryView replaces StreamingTimelineEntry | CEL 7b |
| CLM 17 | both | docs(web-ui): document conversation list management and search | CLM 1, CLM 10, CLM 11, CLM 12, CLM 13, CLM 14, CLM 15, CLM 16, CLM 2, CLM 3, CLM 4, CLM 5, CLM 7, CLM 8, CLM 9 |
| FUS 9 | docs | chore(openspec): archive follow-up-suggestions | FUS 1, FUS 2, FUS 3, FUS 4, FUS 5, FUS 6, FUS 7a, FUS 7b, FUS 7c, FUS 8a, FUS 8b |
| MB 12a | client | feat(app): stream edit and regenerate responses | FUS 7a, FUS 7b, FUS 7c, MB 7a |
| MF 7b | client | feat(app): show message feedback in the chat thread | CEL 8a, CEL 8b, MF 5, MF 7a |
| TAP 20a | server | feat(cli,web-ui): enable tool approvals and questions in web chat | TAP 11, TAP 12, TAP 16, TAP 17, TAP 9 |

## Wave 10 (7 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A11Y C12 | client | test(app): chat accessibility guideline suite and user docs | A11Y C1, A11Y C10, A11Y C11, A11Y C3, A11Y C4, A11Y C5, A11Y C6, A11Y C7, A11Y C8, A11Y C9, A11Y S3, STOP 5c |
| CEL 13 | client | refactor(app): chat screen adapter and element index reconciliation | CEL 1, CEL 10, CEL 11a, CEL 11b, CEL 11c, CEL 12, CEL 1b, CEL 2, CEL 3, CEL 4, CEL 5a, CEL 5b, CEL 6, CEL 7a, CEL 7b, CEL 7c, CEL 8a, CEL 8b, CEL 9 |
| MB 12b | client | feat(app): edit, regenerate and retry in ChatNotifier | MB 12a, TAP 15 |
| TAP 20b | both | docs(app): approvals and questions guide; ship | TAP 18a, TAP 18b, TAP 19, TAP 20a |
| TRR C3 | client | feat(app): install the tool result renderer scope in the chat screen | CEL 6, CEL 7a, CEL 7b, CEL 7c, TRR C1, TRR C2 |
| TRR C4 | client | feat(app): terminal and sources result views | CEL 7a, CEL 7b, CEL 7c, TRR C2 |
| TRR C5 | client | feat(app): link preview and plan result views | CEL 7a, CEL 7b, CEL 7c, TRR C2 |

## Wave 11 (3 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A11Y C13 | none | chore(openspec): archive chat-a11y-and-code-blocks | A11Y C12 |
| MB 12c | client | feat(app): edit, regenerate and branch picker in the chat timeline | MB 10, MB 11, MB 12b |
| TRR C8 | both | docs(app): tool result views, baselines and archive | TRR C1, TRR C2, TRR C2b, TRR C3, TRR C4, TRR C5, TRR C6, TRR C7, TRR S10, TRR S1a, TRR S3, TRR S4, TRR S5, TRR S6, TRR S7, TRR S8, TRR S9 |

## Wave 12 (2 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A2UI 15 | client | feat(app): stream surface_update into the live timeline | A2UI 14, A2UI 6, FUS 7a, FUS 7b, FUS 7c, MB 12a, MB 12b, MB 12c, STOP 4, STOP 5a, STOP 5b, STOP 5c, TRR C1 |
| MB 13 | client | feat(app): regenerate with another model | MB 12c, MB 8b |

## Wave 13 (2 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A2UI 16 | client | feat(app): surfaces in conversation history | A2UI 15, A2UI 9, TRR C1 |
| MB 14 | both | docs(web-ui): conversation branching guide and ship | MB 13, MB 3b, MB 7b |

## Wave 14 (1 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A2UI 17 | client | feat(app): post-turn surface actions | A2UI 11, A2UI 16 |

## Wave 15 (2 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A2UI 18 | client | feat(app): answer in-turn surface interactions | A2UI 12, A2UI 17, TAP 15, TAP 16 |
| A2UI 19 | server | feat(tool-executor): register render-surface and advertise it on web | A2UI 14b, A2UI 15, A2UI 16, A2UI 17, A2UI 5, A2UI 8, TAP 9 |

## Wave 16 (2 PRs)

| PR | Layer | Title | Depends on |
|---|---|---|---|
| A2UI 20 | server | feat(tool-executor): offer wait_for_action | A2UI 12, A2UI 18, A2UI 19 |
| A2UI 21 | both | docs(chat): generative UI docs, baselines and final checks | A2UI 19 |

## Hotspot merge rules (from the cross-change review; PR ids may predate the split PRs) within a wave:
- Runner (messages.rs): FUS 3 before TAP 7 in w3; MB 7 before A11Y S3 in w5.
- dispatch.rs: TAP 6 before TRR S3 in w2.
- chat_screen.dart: CEL 9 then CEL 10 (stacked), with CEL 5 and CEL 11 rebasing, in w3.
