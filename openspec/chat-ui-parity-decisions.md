# Chat UI parity: open product decisions

The ten chat-UI-parity OpenSpec changes leave the product questions below open.
Each row shows the **default** the specs take now. Override a row by editing it
before implementation starts. Decisions the user answered directly are final.

## chat-element-library

| Question                                            | Default                                                       |
| --------------------------------------------------- | ------------------------------------------------------------- |
| Highlight the selected conversation in the sidebar? | Off by default here; conversation-list-management turns it on |
| Rich tool renderers inside subagent timelines?      | No. Subagent tool calls stay compact rows                     |
| Reconnect banner position                           | Unchanged (above the composer)                                |

## chat-stop-cancels-turn

| Question                                               | Default                                                                    |
| ------------------------------------------------------ | -------------------------------------------------------------------------- |
| Show "Stopped" after reload                            | Yes, via a persisted `finish_reason` on messages                           |
| Stop before any text                                   | Persist a short "_Stopped._" assistant row so reload and LLM context agree |
| Cancelled before a worker claimed the turn             | Show the user message as failed, with Retry                                |
| "Stopping…" ceiling                                    | 10 s, also when Stop is pressed before the run id is known                 |
| Auto-title a conversation whose first turn was stopped | Yes, when there is user text                                               |

## tool-result-renderers

| Question                                            | Default                                           |
| --------------------------------------------------- | ------------------------------------------------- |
| `ToolOutput.success = false` shown as error         | Yes. Non-zero bash exit counts as an error        |
| Expand at compact (phone) density                   | Yes, tap to expand the full view                  |
| Built-in `plan-update` tool                         | Yes                                               |
| Terminal/diff expanded by default at normal density | No. Collapsed behind the summary, expand on error |
| ADR number                                          | 0020                                              |

## tool-approval-prompts

| Question                                                      | Default                                                     |
| ------------------------------------------------------------- | ----------------------------------------------------------- |
| Web/orchestrator process split                                | Cross-process channel over space.db and the bus (design D2) |
| Scope of saved "always" rules | All of the user's turns in the space, including scheduled and messenger turns |
| Admins can list and revoke other members' rules               | Yes, space admins can                                       |
| Approval timeout / push delay                                 | 10 min / 30 s                                               |
| "Always allow" for shells, interpreters, destructive commands | Never; allow-once only                                      |
| Server-before-client rollout | Reorder the stack: the PR that turns approvals on lands after the client cards; no feature flag |

## message-branching

| Question                                          | Default                                                           |
| ------------------------------------------------- | ----------------------------------------------------------------- |
| Models for regenerate                             | Curated `[llm] available_models` allowlist; any member may use it |
| Branch switch lands on                            | Most recently created child                                       |
| Branching while an approval is pending            | Auto-cancel the pending approval                                  |
| Edit/regenerate on messenger-shared conversations | Allowed; the messenger continues the active branch                |
| Edit a message with attachments                   | Carry attachments forward                                         |
| Plain send during edit/regenerate                 | 409; messenger turns are deferred and retried                     |

## generative-ui-a2ui

| Question                           | Default                                        |
| ---------------------------------- | ---------------------------------------------- |
| Image hosts on surfaces | Any `https:` host |
| `openUrl` outside the allowlist    | Show the destination and ask before opening    |
| Surface answerable until           | The next user turn                             |
| User bubble after a submit         | `Submitted "<title>"` with a "via form" marker |
| Material surfaces on iOS and macOS | Acceptable for v1                              |

## message-feedback

| Question                            | Default                              |
| ----------------------------------- | ------------------------------------ |
| Export comment text to telemetry    | Never; only `has_comment`            |
| Comment field on positive votes     | Yes, optional on both                |
| Wait for non-destructive compaction | Yes, ship after message-branching D5 |
| Emit a `cleared` event              | Yes                                  |

## follow-up-suggestions

| Question                                     | Default                            |
| -------------------------------------------- | ---------------------------------- |
| `[suggestions].enabled` default | On |
| Chips return when a conversation is reopened | No; live and reconnected runs only |
| Who edits starters on org-owned Personas     | Persona owner or space admin       |
| Org-level fallback starters                  | No                                 |
| Suggestions for voice turns                  | No                                 |
| Extra replay request per turn                | Accepted                           |

## conversation-list-management

| Question                            | Default                          |
| ----------------------------------- | -------------------------------- |
| Archive in v1 | Ship it |
| New message unarchives              | Yes                              |
| Pin an archived conversation        | Unarchive and pin                |
| Search thinking text                | No                               |
| Tokens spread across messages match | Yes                              |
| Swipe to pin                        | No; pin only in the action sheet |

## chat-a11y-and-code-blocks

| Question                                       | Default                                                    |
| ---------------------------------------------- | ---------------------------------------------------------- |
| Completion announcement                        | Persona name plus a 200-character excerpt                  |
| Response timing indicator                      | Behind a "Show response timing" preference, off by default |
| Conversation list breakpoint                   | Compare the chat pane width                                |
| Announce completion after a replayed reconnect | Yes                                                        |
