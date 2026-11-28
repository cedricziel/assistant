## ADDED Requirements

### Requirement: Messages form a tree with a per-conversation active leaf

The system SHALL store each message with an optional `parent_id` referencing another message in the same conversation, an optional `run_id` naming the turn run that wrote it, and each conversation with an optional `active_leaf_id`. A message with no parent is a root. Messages sharing the same parent (or roots of the same conversation) are siblings. The active path SHALL be the ancestor chain of `active_leaf_id`, ordered root first. `parent_id` SHALL carry no `ON DELETE` action. Deleting a conversation SHALL remove all of its messages through the `conversation_id` cascade, whatever the length of the chain.

#### Scenario: Existing conversations are backfilled as a single chain

- **WHEN** the `message_tree` migration runs on a database with a conversation of rows ordered by `(turn, created_at)`
- **THEN** each row's `parent_id` is the previous row in that order and the first row has no parent
- **AND** the conversation's `active_leaf_id` is its last row
- **AND** `load_history` returns the same rows in the same order as before the migration

#### Scenario: Empty conversation has no active leaf

- **WHEN** a conversation is created
- **THEN** its `active_leaf_id` is absent and `load_history` returns an empty list

#### Scenario: Deleting a long conversation

- **WHEN** a conversation whose active path is a chain of 1,500 messages is deleted
- **THEN** the delete succeeds and no message of that conversation remains

### Requirement: Saving a message appends to the active leaf atomically

`ConversationStore::save_message` SHALL, in a single `BEGIN IMMEDIATE` transaction, insert the message with `parent_id` set to the conversation's current `active_leaf_id` and set `active_leaf_id` to the new message. `ConversationStore::branch_message(msg, parent_id)` SHALL insert the message under the given parent (`None` creates a new root) and make it the active leaf. When the insert conflicts with an existing message id, neither method SHALL change `active_leaf_id`. Both `SqliteConversationStore` and `InMemoryConversationStore` SHALL satisfy the same contract tests.

#### Scenario: Sequential saves form a chain

- **WHEN** messages A, B, C are saved with `save_message` into an empty conversation
- **THEN** A has no parent, B's parent is A, C's parent is B, and the active leaf is C

#### Scenario: Branching under an earlier message

- **WHEN** the active path is A → B → C and `branch_message(D, Some(A))` is called
- **THEN** D's parent is A, D is the active leaf, and `load_history` returns A, D
- **AND** B and C remain stored

#### Scenario: Branching a new root

- **WHEN** `branch_message(R, None)` is called on a conversation whose root is A
- **THEN** A and R are siblings and `load_history` returns R

#### Scenario: A conflicting insert does not move the leaf

- **WHEN** the active path is A → B → C and `save_message` is called again with B's id
- **THEN** no row is inserted and the active leaf is still C

### Requirement: Compaction persistence keeps a loadable history during rollout

Until non-destructive compaction replaces it, `ConversationStore::replace_history` SHALL delete the conversation's messages and reset `active_leaf_id` to absent in the same transaction, before it re-inserts the kept messages.

#### Scenario: History survives destructive compaction

- **WHEN** compaction persists a summary plus recent messages through `replace_history`
- **THEN** `load_history` returns exactly the re-inserted messages, chained in order

### Requirement: LLM context and history readers use the active path only

`ConversationStore::load_history` SHALL return only the active path. The title generator and the conversation indexer SHALL read the active path through `load_history`. The orchestrator, `/status` and `/compact` SHALL build history through one shared summary-applied history builder over the path, and SHALL never see messages from inactive branches. `turn` SHALL be the iteration index along a path: a turn's user message gets `base_turn`, every row written in iteration `i` gets `base_turn + i + 1`, rows from one iteration share it, and `(turn, tool_name)` is unique along a path. `base_turn` SHALL be the number of messages before the turn's user message on its path: the active path's length for an append, the length of P's ancestor chain including P for an edit under P, and the length of U's ancestor chain excluding U for a regenerate of U.

#### Scenario: Inactive branch is invisible to the model

- **WHEN** a user message has two replies R1 (inactive) and R2 (active) and the user sends a follow-up
- **THEN** the LLM request history contains R2 and does not contain R1

#### Scenario: Messenger interfaces continue the active branch

- **WHEN** a web user switches a conversation's active branch and a messenger interface then dispatches a turn for that conversation
- **THEN** the messenger's user message is appended as a child of the selected branch's leaf

#### Scenario: Rows of one iteration share a turn

- **WHEN** an append turn with base_turn 4 makes two tool calls in its first iteration and answers in its second
- **THEN** the user row has turn 4, both tool-call rows and their results have turn 5, and the final answer has turn 6

#### Scenario: Edit turn numbering

- **WHEN** the active path is U1 → A1 → U2 → A2 and U2 is edited
- **THEN** the new user message U2' has turn 2 and its reply rows start at turn 3

#### Scenario: Regenerate turn numbering

- **WHEN** the active path is U1 → A1 → U2 → A2, where A2 has turn 3, and U2 is regenerated
- **THEN** the first row of the new reply has turn 3

#### Scenario: /status counts the summary-applied history

- **WHEN** a conversation has a summary on its active path and the user runs `/status`
- **THEN** the token estimate is computed over the summary plus the path messages after it, not over the full path

### Requirement: Context compaction does not delete messages

Context compaction SHALL persist a summary row in `conversation_summaries` with the id of the last message it summarises (`through_message_id`) instead of replacing the conversation's messages. The shared history builder SHALL use the newest summary whose `through_message_id` is on the path, followed by the path messages after it. `/compact` SHALL compact the summary-applied history. `ConversationStore::replace_history` SHALL be removed.

#### Scenario: Compaction keeps all rows

- **WHEN** compaction runs on a conversation with 40 messages
- **THEN** all 40 messages remain stored and `GET /api/conversations/{id}` still lists them
- **AND** the next turn's LLM history starts with the summary text followed by the kept recent messages

#### Scenario: Summary does not apply to a branch that diverged earlier

- **WHEN** a summary covers messages through M10 and the user edits M4, creating a new branch
- **THEN** the new branch's LLM history contains no summary and consists of the path messages only

#### Scenario: Unresolvable split point skips persistence

- **WHEN** the compaction split index maps to no persisted message id
- **THEN** no summary row is written and the in-memory compacted history is still used for the current turn

#### Scenario: /compact builds on an existing summary

- **WHEN** a summary through M10 exists on the path and the user runs `/compact`
- **THEN** the compaction input is the existing summary plus the messages after M10

### Requirement: Branch endpoints return the error envelope and enforce write access

The edit, regenerate and switch-branch endpoints SHALL require `Access::Write` on the space and, for token callers, the `conversations:write` scope. `GET /api/models` SHALL require `Access::Read`. Every 400, 403, 404 and 409 from these endpoints SHALL use the `{"error": ...}` envelope.

#### Scenario: Edit without conversations:write

- **WHEN** an API key without `conversations:write` calls edit
- **THEN** the response is 403 with an `{"error": ...}` body and no message is stored

#### Scenario: Regenerate without conversations:write

- **WHEN** an API key without `conversations:write` calls regenerate
- **THEN** the response is 403 with an `{"error": ...}` body and no turn is started

#### Scenario: Switch-branch without conversations:write

- **WHEN** an API key without `conversations:write` calls switch-branch
- **THEN** the response is 403 with an `{"error": ...}` body and the active leaf is unchanged

#### Scenario: Read-only member

- **WHEN** a member with read-only access to the space calls edit, regenerate or switch-branch
- **THEN** the response is 403 with an `{"error": ...}` body

#### Scenario: Models endpoint requires authentication

- **WHEN** `GET /api/models` is called without credentials
- **THEN** the response is 401
- **AND** a read-only member receives 200

### Requirement: Editing a user message creates a sibling branch and runs a turn

The system SHALL provide `POST /api/conversations/{id}/messages/{message_id}/edit` (operationId `edit_message`) accepting `EditMessageRequest { message }`. It SHALL insert a new user message as a sibling of `message_id` (same parent), make it the active leaf, run a turn whose history is the new message's ancestor chain, and stream the response with the same SSE frames as `POST /api/conversations/{id}/messages`. The original message and its descendants SHALL remain stored. Editing a message that is not on the active path SHALL be accepted and SHALL make the new branch active. The new message SHALL carry the original message's attachments forward: every attachment of the original SHALL also be attached to the new message, and the turn's history SHALL include them. The edit request changes the text only. Editing SHALL be accepted whichever interface created the conversation.

#### Scenario: Edit streams a new reply on a new branch

- **WHEN** the active path is U1 → A1 → U2 → A2 and the client edits U2 with "corrected text"
- **THEN** the response is an SSE stream starting with `run_started` and ending with `done`
- **AND** afterwards the active path is U1 → A1 → U2' → A2' where U2' has content "corrected text"
- **AND** U2 and A2 are still stored as an inactive branch

#### Scenario: Editing a message on an inactive branch

- **WHEN** U1 has replies A1 (inactive, followed by U2) and A1' (active), and the client edits U2
- **THEN** U2' is a child of A1, the active path is U1 → A1 → U2' → A2', and the LLM history contains A1 and not A1'

#### Scenario: Editing a message with attachments

- **WHEN** U2 has attachments X and Y and the client edits U2 with "corrected text"
- **THEN** U2' lists X and Y as its attachments, and U2 still lists them
- **AND** the LLM history for the turn includes X and Y on U2'
- **AND** no attachment bytes are copied

#### Scenario: Redelivered edit does not duplicate attachment links

- **WHEN** an edit of an attachment-bearing U2 fails transiently after U2' was saved and is redelivered
- **THEN** U2' lists each attachment exactly once

#### Scenario: Editing in a conversation a messenger also uses

- **WHEN** a conversation that a Slack interface created has the active path U1 → A1 and a web member edits U1
- **THEN** the edit is accepted and U1' becomes the active branch
- **AND** nothing is posted to the Slack thread
- **AND** the next Slack message in that conversation is appended under the active leaf of the edited branch

#### Scenario: Editing a non-user message is rejected

- **WHEN** the client calls edit with the id of an assistant message
- **THEN** the response is 400 with `{"error": "Only user messages can be edited"}`

#### Scenario: Empty edit is rejected

- **WHEN** the client calls edit with a blank `message`
- **THEN** the response is 400 with an `{"error": ...}` body

#### Scenario: Unknown or foreign message

- **WHEN** `message_id` does not exist or belongs to another conversation
- **THEN** the response is 404 with an `{"error": ...}` body

#### Scenario: Connection drops during an edit

- **WHEN** the SSE connection drops while an edit of U2 is streaming and the turn completes on the server
- **THEN** `GET /api/conversations/{id}` lists U1 → A1 → U2' → A2' and U2' carries `branch: {index: 1, count: 2}`

### Requirement: Attachments can be shared across message versions

The system SHALL store extra attachment references in a `message_attachment_links(message_id, attachment_id)` table with a composite primary key and `ON DELETE CASCADE` on both foreign keys. `AttachmentStore::list_for_message` SHALL return the attachments whose `message_id` matches plus those reached through a link. `AttachmentStore::carry_forward(from_message_id, to_message_id)` SHALL link every attachment of the source message to the target and SHALL be idempotent. Deleting a conversation SHALL remove its links.

#### Scenario: Carry forward is idempotent

- **WHEN** `carry_forward(U2, U2')` is called twice
- **THEN** `list_for_message(U2')` returns each attachment of U2 once

#### Scenario: Original keeps its attachments

- **WHEN** `carry_forward(U2, U2')` has run
- **THEN** `list_for_message(U2)` is unchanged

### Requirement: Regenerating creates a sibling reply

The system SHALL provide `POST /api/conversations/{id}/messages/{message_id}/regenerate` (operationId `regenerate_message`) accepting `RegenerateMessageRequest { model? }`. `message_id` MAY name a user message or any message of a reply; for a reply message the server SHALL resolve the nearest user-message ancestor U. The server SHALL run a turn that answers U using U's ancestor chain, including U and U's attachments, without persisting a new user message. The first persisted row of the new reply SHALL be a child of U and become the active leaf. The response SHALL stream the same SSE frames as `send_message`. U MAY be on an inactive branch.

#### Scenario: Regenerate adds a second reply

- **WHEN** the active path is U1 → A1 and the client regenerates A1
- **THEN** a new reply A1' is persisted as a child of U1 and becomes the active leaf
- **AND** no new user message is stored
- **AND** U1 now has two children

#### Scenario: Regenerate failure before any output keeps the old reply

- **WHEN** a regenerate turn fails before persisting any row
- **THEN** the active leaf is unchanged and A1 is still on the active path

#### Scenario: Regenerating a message on an inactive branch

- **WHEN** U2 sits on an inactive branch and the client regenerates U2
- **THEN** the LLM history is U2's ancestor chain, the new reply is a child of U2, and it becomes the active leaf

### Requirement: Anchored turns are idempotent under redelivery

Every row a turn persists SHALL carry the turn's `run_id`: the request id when present, otherwise the bus message id, which is stable across redelivery. The user message id SHALL be fixed before the first run. When a turn runs again after a nack, the orchestrator SHALL delete the rows the earlier attempt persisted under that `run_id` (except the user message), reset the active leaf to the turn's anchor point, and re-save the user message under its fixed id, which is a no-op if it already exists. If the delete fails, the orchestrator SHALL log a warning and only reset the leaf.

#### Scenario: Redelivered regenerate leaves one new reply

- **WHEN** a regenerate of U persists a thinking row, fails with a transient error, is redelivered and then completes
- **THEN** U has exactly two children: the old reply and the completed new reply
- **AND** the thinking row from the failed attempt is gone

#### Scenario: Redelivered edit reuses the new user message

- **WHEN** an edit creating U' fails transiently after U' was saved and is redelivered
- **THEN** exactly one U' exists, the reply chains after it, and the active leaf is the reply's last row

#### Scenario: Redelivered append does not duplicate the user message

- **WHEN** a web append turn fails transiently after saving its user message and is redelivered
- **THEN** the conversation contains that user message once

### Requirement: Cancelling a branch turn keeps the tree consistent

When a web turn is cancelled, the partial reply the runner persists SHALL follow the turn's anchor. For a regenerate that has persisted no row, the partial reply SHALL be saved as a child of U. Otherwise it SHALL be appended to the run's own leaf. The "already saved" race guard SHALL consider only rows carrying the run's `run_id`. The saved row SHALL carry the cancelled `finish_reason` persisted by `chat-stop-cancels-turn`. A cancel that leaves no text SHALL still persist the short stopped row that change specifies, anchored by the same rule. A turn cancelled before a worker claimed it SHALL persist nothing and SHALL leave the active leaf where it was.

#### Scenario: Cancel a regenerate before its first row

- **WHEN** a regenerate of U has streamed "Partial" and has persisted no row when the user cancels it
- **THEN** a message "Partial" is stored as a child of U and is the active leaf
- **AND** the old reply is unchanged and is now a sibling branch

#### Scenario: Cancel a regenerate after its first row

- **WHEN** a regenerate of U has persisted a tool-call row and streamed "Partial" when the user cancels it
- **THEN** "Partial" is appended after the tool-call row on the new branch and no row is added under the old reply

#### Scenario: Cancel a regenerate before any text

- **WHEN** a regenerate of U is cancelled after the worker claimed it and before any text or row was produced
- **THEN** a "_Stopped._" assistant message with the cancelled `finish_reason` is stored as a child of U and is the active leaf
- **AND** the old reply is a sibling version

#### Scenario: Cancel before a worker claims the turn

- **WHEN** a regenerate of U is cancelled before any worker claims it
- **THEN** no row is stored and the old reply is still the active leaf

#### Scenario: Cancel an edit

- **WHEN** an edit has saved U' and streamed "Partial" when the user cancels it
- **THEN** "Partial" is a child of U' and the active path ends with it

### Requirement: Regenerate with a different model

`RegenerateMessageRequest.model`, when present, SHALL be either the org's `[llm] model` or an entry in `[llm] available_models`. The turn SHALL run on a provider for that model with the org's provider kind, endpoint, and credentials. Any member with write access to the space MAY use any listed model. The override SHALL apply to that turn only and SHALL NOT change the conversation's `/model` setting. Each persisted assistant message SHALL record the model name that produced it in `messages.model`.

#### Scenario: Regenerate on an allowed model

- **WHEN** `available_models = ["model-b"]`, the default is `model-a`, and the client regenerates with `{"model": "model-b"}`
- **THEN** the new reply is produced by `model-b` and its `MessageSummary.model` is `"model-b"`
- **AND** the next normal send uses `model-a`

#### Scenario: A non-admin member may regenerate with a listed model

- **WHEN** a member with write access who is not a space admin regenerates with a model in `available_models`
- **THEN** the turn runs on that model

#### Scenario: Unknown model is rejected

- **WHEN** the client regenerates with `{"model": "not-listed"}`
- **THEN** the response is 400 with an `{"error": ...}` body and no turn is started

### Requirement: Models available for regenerate are listed

The system SHALL provide `GET /api/models` (operationId `list_models`) returning a bare JSON array of `ModelSummary { name, is_default }` containing the org's `[llm] model` (with `is_default = true`) followed by each `available_models` entry.

#### Scenario: Only the default is configured

- **WHEN** `available_models` is empty
- **THEN** the response is `[{"name": "<default>", "is_default": true}]`

### Requirement: Conversation detail carries branch metadata

`MessageSummary` in `GET /api/conversations/{id}` SHALL list only the active path and SHALL carry `parent_id` (omitted for roots) and, when the message has siblings, `branch: BranchInfo { index, count, sibling_ids }` with `index` zero-based in creation order. For reply versions, the `BranchInfo` of the reply's first row SHALL be projected onto the last visible assistant row of that reply. When the active reply has no visible assistant row, the reply's `BranchInfo` SHALL be carried on its user message as `reply_branch`.

#### Scenario: User message with two versions

- **WHEN** U2 was edited once and U2' is active
- **THEN** U2' carries `branch: {index: 1, count: 2, sibling_ids: [U2, U2']}`

#### Scenario: Reply versions shown on the visible reply

- **WHEN** U1 has two replies whose first rows are a tool-call row and a thinking row respectively
- **THEN** the last visible assistant message of the active reply carries `branch.count == 2` and `sibling_ids` are the two first rows

#### Scenario: Reply with no visible assistant row

- **WHEN** U1 has two replies and the active one consists only of a tool-call row
- **THEN** U1 carries `reply_branch: {index: 1, count: 2, ...}`

#### Scenario: No siblings

- **WHEN** a message has no siblings
- **THEN** its `branch` field is omitted

### Requirement: Switching the active branch

The system SHALL provide `POST /api/conversations/{id}/switch-branch` (operationId `switch_conversation_branch`) accepting `SwitchBranchRequest { message_id }`. The server SHALL set `active_leaf_id` to the leaf reached by descending from `message_id` through the most recently created child at each level, and SHALL respond 200 with the updated `ConversationDetail`. Switching SHALL NOT change the conversation's `updated_at` and SHALL broadcast a conversation upsert.

#### Scenario: Switch to an older reply

- **WHEN** U1 has replies A1 (older) and A1' (active) and the client switches to A1
- **THEN** the response's messages are the path through A1 and its descendants

#### Scenario: Switch to a message with nested branches

- **WHEN** the target message has descendants with forks
- **THEN** the new leaf is reached by following the most recently created child at each fork

#### Scenario: Switch does not reorder the conversation list

- **WHEN** a switch succeeds
- **THEN** the conversation's `updated_at` is unchanged

### Requirement: One orchestrator-owned registry tracks in-flight turns

The orchestrator SHALL own a single registry of in-flight turns per conversation, which replaces the web, channel-runner and CLI `active_turns` maps and backs `/stop` and `/status`. Acquiring a lease SHALL be an atomic check-and-insert. Shared leases SHALL be refused while an exclusive lease is held. Exclusive leases SHALL be refused while any lease is held. Every lease SHALL be released on every exit path, including errors, cancellation and a dropped handler. Web handlers SHALL acquire the lease before submitting the turn. `send_message` SHALL take a shared lease. Edit and regenerate SHALL take an exclusive lease for the whole turn. Switch-branch SHALL take an exclusive lease for its update. A `send_message` request SHALL be refused with 409 while an exclusive lease is held. The worker SHALL take a shared lease for every claimed turn not already registered, and SHALL defer such a turn with a delayed nack while an exclusive lease is held. The registry is per process.

#### Scenario: Switch during a running turn

- **WHEN** a turn is streaming for conversation C and the client calls switch-branch on C
- **THEN** the response is 409 with an `{"error": ...}` body and the active leaf is unchanged

#### Scenario: Send during an edit

- **WHEN** an edit turn is running for conversation C and the client posts a message to C
- **THEN** the response is 409 with an `{"error": ...}` body and no user message is stored

#### Scenario: Messenger turn deferred during a regenerate

- **WHEN** a regenerate turn holds the exclusive lease for conversation C and a messenger turn for C is claimed by a worker
- **THEN** the worker defers the messenger turn with a delayed nack and does not run it
- **AND** once the lease is released the redelivered turn runs and appends under the active leaf

#### Scenario: Two rapid edits

- **WHEN** two edit requests for conversation C arrive concurrently
- **THEN** exactly one starts a turn and the other receives 409

#### Scenario: Entry clears after cancellation

- **WHEN** a running turn is cancelled via `POST .../turns/{id}/cancel`
- **THEN** a subsequent regenerate on that conversation is accepted

#### Scenario: Scheduler turn blocks branch operations

- **WHEN** a scheduler-published turn for conversation C is running in the worker and the client calls regenerate on C
- **THEN** the response is 409

#### Scenario: Web /stop finds the running turn

- **WHEN** a web turn is running for conversation C and the user runs `/stop` in C
- **THEN** the turn is cancelled

### Requirement: Branch operations cancel a turn that is waiting for the user

When edit, regenerate or switch-branch finds the conversation's exclusive lease refused, and every in-flight run of that conversation has a pending interaction (a tool approval, a question or a surface wait), the system SHALL cancel those runs through the same path as `/stop`, wait up to 10 seconds for their leases to be released, and then acquire the exclusive lease and proceed. The cancelled interactions SHALL resolve as `cancelled` and the gated tool call SHALL NOT run. When any in-flight run has no pending interaction, the system SHALL return 409 and cancel nothing. When the leases are not released within the wait, the system SHALL return 409.

#### Scenario: Edit while an approval is pending

- **WHEN** a turn is blocked on a pending approval for a `bash` call and the client edits an earlier message
- **THEN** the approval resolves as `cancelled` and the `bash` call does not run
- **AND** the edit is accepted and streams a new reply on a new branch
- **AND** the old branch ends with a stopped reply

#### Scenario: Regenerate while a question is pending

- **WHEN** a turn is blocked on an `ask-user` question and the client regenerates the previous reply
- **THEN** the question resolves as `cancelled` and the regenerate is accepted

#### Scenario: Switch while an approval is pending

- **WHEN** a turn is blocked on a pending approval and the client switches to another branch
- **THEN** the approval resolves as `cancelled` and the response is 200 with the new path

#### Scenario: Working turn is not cancelled

- **WHEN** a turn is streaming text with no pending interaction and the client calls edit
- **THEN** the response is 409 and the turn keeps running

#### Scenario: Run does not release in time

- **WHEN** the cancelled run's lease is still held after 10 seconds
- **THEN** the response is 409 with an `{"error": ...}` body

### Requirement: Branch runs reuse the existing SSE frame set

Edit and regenerate SHALL emit no new SSE event types and SHALL be driven by the same streamed-turn runner as `send_message`, with the same `interactive` setting as `send_message` and with `suggestions` enabled. The three entry points SHALL build their run options through one shared constructor, so a change to web-turn interactivity applies to all of them together. The `run_started` payload of text, edit and regenerate runs SHALL include `user_message_id`: the id under which the run will persist its user message (for regenerate, the existing message U). It SHALL survive replay via `GET /api/conversations/{id}/runs/{run_id}/events/stream`.

#### Scenario: run_started identifies the answered message

- **WHEN** a client regenerates user message U
- **THEN** the `run_started` frame payload is `{"run_id": "<run>", "user_message_id": "<U>"}`
- **AND** replaying the run from `since=0` returns the same payload

#### Scenario: Send pre-assigns the user message id

- **WHEN** a client sends a message and the turn completes
- **THEN** the persisted user message's id equals `run_started.user_message_id`

#### Scenario: Branch turns are as interactive as send

- **WHEN** web turns are interactive (`tool-approval-prompts` has enabled them) and an edit or regenerate turn reaches a tool call whose permission decision is Ask
- **THEN** the turn is marked interactive and the call surfaces as an approval prompt instead of being auto-denied

#### Scenario: Interactivity is not enabled by this change

- **WHEN** web turns are not yet interactive
- **THEN** edit and regenerate turns are not interactive either, and an Ask decision is denied as it is for `send_message`
