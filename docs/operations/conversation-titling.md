# Conversation Titling

Conversations are auto-titled by a background worker that consumes
`turn.result` messages from the assistant's internal message bus. The worker
is interface-agnostic: it titles conversations that originate from the web
UI, Slack, Matrix, Mattermost, Nextcloud, Signal, the CLI REPL, MCP clients,
and scheduler-driven runs.

## How it works

```text
Any interface    ┌──────────────────┐
─────────────▶   │  turn.result     │    ┌─────────────────────┐
(turn done)      │  on MessageBus   │ ─▶ │  Title-generator    │
                 └──────────────────┘    │  worker             │
                                         │  • check eligibility│
                                         │  • call LLM (short  │
                                         │    summary prompt)  │
                                         │  • update_title     │
                                         │    sets title_locked│
                                         │  • broadcasts       │
                                         │    ConversationUp-  │
                                         │    serted to UIs    │
                                         └─────────────────────┘
```

Each conversation row in SQLite gains a `title_locked` boolean. The worker
will never overwrite a row where `title_locked = 1`. That flag is set:

- when a user (or another caller) sets a title via `PATCH /api/conversations/{id}`,
- when a client creates a conversation with an explicit title via
  `POST /api/conversations`,
- when the title-generator worker successfully writes a title.

Migrations lock all pre-existing titled conversations on first deploy.

## Configuration

Add a `[titling]` block to `orgs/{slug}/org.toml` (or the global config
during single-org operation):

```toml
[titling]
# Master switch. When false, the worker still consumes and acks turn.result
# events but never calls the LLM.
enabled = true

# Minimum turn number at which the worker will title an unlocked conversation
# under normal flow. Default: 2 (worker waits until the second assistant
# response has landed before producing a title).
min_turns = 2

# Escape hatch: an unusually long first user message can be titled after
# turn 1 even if min_turns has not been reached.
long_first_message_chars = 200
```

The worker uses the conversation's primary LLM provider for the title call.
A per-org model override is intentionally deferred until the `LlmProvider`
trait gains a per-call model-selection knob.

Omitting `[titling]` applies all defaults.

## Cost

Each title costs one short LLM call:

| Model                    | Approx cost per title |
| ------------------------ | --------------------- |
| Local Ollama (llama3:8b) | free                  |
| Claude Haiku             | < $0.001              |
| Claude Sonnet            | < $0.005              |
| GPT-4o-mini              | < $0.001              |

If your primary model is more expensive than you'd like for titling, the
mitigation today is to disable titling for that org (`enabled = false`)
and use manual rename via `PATCH /api/conversations/{id}`.

## Disabling titling

Set `enabled = false` in `[titling]`. Existing titles remain; new
conversations stay `NULL`-titled in the database and display as "Untitled"
in every UI that surfaces conversation lists.

## Privacy considerations

The title may reflect the first few messages of the conversation. It is
sent to the same LLM provider that handles your conversations, so the
trust boundary is identical. Orgs with stricter data-handling requirements
should disable titling.

## Operator playbook

| Symptom                        | Likely cause                                | Fix                                                                                                        |
| ------------------------------ | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Conversations never get titled | Worker not running, or `enabled = false`    | Check `[titling].enabled`; check process logs for "Title-generator worker started"                         |
| Titles arrive but look wrong   | Model under-performing                      | Switch the conversation's primary model in `[llm]` — titling rides on the same provider                    |
| Many "Untitled" old rows       | Pre-existing data; backfill is out of scope | Manually rename via `PATCH /api/conversations/{id}`                                                        |
| LLM timeouts in logs           | Provider slow/down                          | Bus retries with exponential backoff; titles eventually appear, or stay NULL after `MAX_TURN_REDELIVERIES` |

## Related

- ADR 0008: Conversation Titling (`docs/adr/adr-0008-conversation-titling.md`)
- Worker source: `crates/runtime/src/title_generator.rs`
- Storage column: `conversations.title_locked` (migration `041`)
