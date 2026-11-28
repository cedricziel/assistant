# Approvals and questions

A turn in web chat can stop and ask you something before it continues. There are
two kinds of prompt:

- an **approval** asks whether a tool call may run;
- a **question** asks you something the model needs answered to finish the task.

Both appear as cards in the conversation, next to the tool call they belong to.
When a call has no provider id to attach to, the card appears as its own entry
at that point in the turn. Both wait for you. A turn that is waiting shows
"Waiting for your approval" or "Waiting for your answer" in place of its usual
activity label.

The checks that decide which calls need approval are covered in
[Tool permissions](../tool-permissions.md).

## Approval cards

When the permission pipeline leaves a call undecided, an interactive web chat
turn asks you instead of denying the call. The tool does not run until you
answer. Calls that a deny rule or the workspace boundary refuses never reach
you.

The card shows what is being asked:

- the tool name and a one-line summary of its main argument, such as the
  `bash` command or the `web-fetch` URL;
- **Show arguments** for the full request. If the request was too large to
  send, the card says the arguments were shortened. The tool itself still runs
  with the full arguments;
- the pipeline's reason for asking;
- a countdown to the expiry;
- only the answer buttons the server offered.

The card is built from the tool call and the permission decision, never from
text the model wrote about it.

### The four answers

| Button        | What it does                                                                        |
| ------------- | ----------------------------------------------------------------------------------- |
| Allow once    | Runs the call. Nothing is saved.                                                    |
| Always allow  | Runs the call and saves an allow rule for you in this space.                        |
| Reject once   | Does not run the call. The model is told you denied it and can continue without it. |
| Always reject | Does not run the call and saves a deny rule for you in this space.                  |

"Always allow" and "Always reject" save a narrow rule derived from the request.
`git status --short` saves `bash(git status*)`, and an `https://docs.rs/…` fetch
saves `web-fetch(https://docs.rs/*)`. The rule is stored on the server; the
browser never sends a pattern of its own.

With a focused card, Enter means **Allow once** and Escape does nothing, so a
stray keypress cannot deny a call. A screen reader announces "Approval needed",
and each button names the action and the tool, for example "Allow bash once".

### When "always" is withheld

Some calls only get **Allow once** and **Reject once**. A saved rule for them
would grant too much, for every future turn, including turns nobody is
watching:

- the command contains a shell metacharacter (`|`, `;`, `&`, `>`, `` ` ``,
  `$(`, and so on);
- it starts with a shell or interpreter (`sh`, `bash`, `zsh`, `fish`, `dash`,
  `python`, `node`, `deno`, `bun`, `ruby`, `perl`, `php`, `lua`) or a wrapper
  that runs other commands (`env`, `xargs`, `eval`, `exec`, `nohup`, `time`,
  `watch`);
- it starts with a destructive command (`rm`, `mv`, `chmod`, `chown`, `dd`,
  `truncate`, `shred`);
- a configured `ask` rule produced the prompt. That rule means a human decides
  every time, so saving an allow would defeat it.

The card still shows the reason and the arguments, so you can decide on the one
call. The CLI prompt offers only the once answers as well.

### Saved rules

A saved rule belongs to one user in one space. It applies to every turn carrying
your identity in that space, whatever the interface. That includes your
scheduled tasks and your messages from Slack or Matrix, which never show a card:
a saved allow lets a matching call run unattended, and a saved deny blocks it.

Other users are not affected, and neither are your turns in another space.

Saved rules never outrank configuration. A built-in or configured deny wins over
a saved allow, a configured `ask` rule still asks, and a saved allow keeps the
shell-metacharacter guard of allow rules, so `bash(git status*)` does not allow
`git status; rm -rf x`.

### Revoking saved rules

**Settings → Tool approvals** lists your saved rules in the current space,
grouped as **Allowed** and **Denied**. Each row shows the rule and the date it
was saved. Revoking asks for confirmation, then deletes the rule. The next
matching call is decided without it, so on an interactive turn it can prompt
again.

Space admins and org admins also see a **Members' approvals** group with the
other members' rules, labelled by owner, and can revoke those. Admins cannot
create or edit a rule, and cannot answer someone else's approval. The server
enforces the role independently of the settings screen.

Saved rules can also be listed and deleted through `/api/tool-permission-rules`;
see [Tool permissions](../tool-permissions.md#approvals-and-saved-rules).

## Question cards

The assistant can call the `ask-user` tool when it needs an answer mid-turn.
It accepts one to four questions. Each is single-choice, multiple-choice, free
text, or a combination, and the card labels the whole request as coming from
your assistant. The questions are written by the model.

- **Submit** stays disabled until every question has an answer.
- **I won't answer** declines the request.
- The answers go back to the model in the order the questions were asked.
- Declining and expiry are not errors: the model is told no answer was given
  and continues the turn.

Once answered, the card becomes a receipt that shows "Answered" with the chosen
answers, "Declined", "Expired" or "Cancelled".

## Timeouts and notifications

An approval or question that nobody answers expires after **10 minutes** by
default. An org can change this with `[permissions] interaction_timeout_secs`
in `org.toml`; the allowed range is 30 seconds to 1 hour. The expiry is also
clamped to the turn's own deadline, so a card never promises more time than the
turn has.

An expired approval does not run the call. The model is told the request expired
and continues without it.

If an interaction is still waiting after **30 seconds**, the server pushes a
notification to the turn owner's subscribed devices. The title is "Approval
needed" for an approval and "The assistant has a question" for a question, and
tapping it opens the conversation. Only the owner's devices are notified. A
repeat push for the same interaction replaces the earlier notification instead
of stacking another one.

On native apps, the **Approval requests** toggle in notification settings turns
the local notification on or off. On web, a device with an active push
subscription does not also show a local notification.

## While a turn waits

- **Reload or close the tab.** The turn keeps running on the server. Reopening
  the conversation shows the pending card again, and answered cards come back
  as receipts. If the push reaches another device, you can answer there.
- **Edit or regenerate a message.** Branching cancels a turn that is only
  waiting for you, then starts the new branch. The waiting card ends
  "Cancelled" and the tool never ran. A turn that is still doing work refuses
  the edit with `409` until it finishes.
- **Restart the orchestrator.** The waiting turn is gone, so its request has no
  live waiter and is marked expired. The card ends "Expired without an answer".
  If the turn is retried, you may see a fresh approval card for the retried
  call.

## Which turns can ask

Only interactive turns can show a card:

- streaming web chat turns: send, edit and regenerate;
- the CLI REPL, which prompts on stdin with the once answers only.

Scheduled tasks, Slack, Matrix, Signal, Mattermost and Nextcloud, A2A, MCP,
voice, quick messages, one-shot CLI commands and Subagent Processes never
prompt. An undecided call there is denied, as before. They do consult your saved
rules, so an allow can let a scheduled call run and a deny blocks it.

The `ask-user` tool is only offered to interactive web turns, so a scheduled
task cannot ask you a question mid-run and the CLI REPL only prompts for
approvals.
