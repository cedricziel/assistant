## ADDED Requirements

### Requirement: LLM error classifier MUST recognize permanent billing errors

The `is_transient_error_message` classifier in the LLM provider crate SHALL return `false` (i.e., not transient) when the error message contains any of the following case-insensitive markers, even if the message also contains an HTTP status code that would otherwise be considered transient (such as `429`):

- `insufficient balance`
- `insufficient quota` or `insufficient_quota`
- `account is suspended` or `account suspended`
- `quota exceeded`
- `please recharge`
- `payment required`
- `billing` co-occurring with any of: `required`, `hard limit`, `exceeded`

This classification SHALL apply to errors from any LLM provider (Moonshot, OpenAI, Anthropic, Ollama-with-paid-backend) without provider-specific code paths.

#### Scenario: Moonshot insufficient-balance 429 is permanent

- **WHEN** the classifier receives `"API error (429 Too Many Requests): Your account ... is suspended due to insufficient balance, please recharge"`
- **THEN** it returns `false` (permanent)
- **AND** the worker treats the turn error as terminal, not transient

#### Scenario: OpenAI insufficient_quota 429 is permanent

- **WHEN** the classifier receives `"API error: 429 — insufficient_quota: You exceeded your current quota"`
- **THEN** it returns `false` (permanent)

#### Scenario: A real transient 429 is still transient

- **WHEN** the classifier receives `"API error 429: Too Many Requests — please retry after 30s"` with no billing/quota markers
- **THEN** it returns `true` (transient)

#### Scenario: Real transient 5xx unaffected

- **WHEN** the classifier receives `"503 Service Unavailable"`
- **THEN** it returns `true` (transient)

### Requirement: NATS pull consumers MUST set an explicit max_deliver cap

Every NATS JetStream pull consumer created by the platform — including the orchestrator's `bus.turn.request` consumer and any other `assistant-bus-nats` consumer — SHALL be created with an explicit `max_deliver` value. The default `max_deliver = -1` (infinite) of `pull::Config::default()` SHALL NOT be used in production code paths.

The cap value SHALL be a single shared constant. The default cap value SHALL be `10`.

#### Scenario: Consumer config carries max_deliver = 10

- **WHEN** `MessageBus` creates or updates a pull consumer for any topic
- **THEN** the resulting NATS consumer info reports `max_deliver: 10`

#### Scenario: Existing consumer config is reconciled on startup

- **WHEN** an `assistant-bus-nats` instance starts against a JetStream that already has a consumer with `max_deliver = -1` for the same name
- **THEN** the consumer is updated (or recreated) so its `max_deliver` matches the configured cap
- **AND** the bus does not silently keep using the old config

### Requirement: MessageBus MUST expose a terminate operation distinct from nack

The `MessageBus` trait SHALL provide a method to terminate a delivered message such that JetStream does not redeliver it. This method is distinct from `nack` (which schedules redelivery) and from `ack` (which marks success). The NATS implementation SHALL ack the message with `AckKind::Term`. The in-memory test implementation SHALL remove the message from the inflight map and never deliver it again.

#### Scenario: Terminating a message stops redelivery

- **WHEN** the worker calls `bus.fail(msg.id)` on a delivered message
- **THEN** JetStream does not redeliver the message
- **AND** subsequent calls to `claim_filtered` on the same topic do not return the same message

### Requirement: Orchestrator worker MUST cap transient redelivery and surface a terminal result

When the orchestrator worker handles a transient turn error, it SHALL inspect the JetStream `delivery_count` for the message:

- If `delivery_count` is less than `MAX_TRANSIENT_DELIVERIES` (default `10`), the worker SHALL nack with the existing backoff schedule (30/60/120/240 seconds for deliveries 1/2/3/4+).
- If `delivery_count` is greater than or equal to `MAX_TRANSIENT_DELIVERIES`, the worker SHALL:
  1. Log a single `error!`-level entry with the conversation id, delivery count, and the underlying error.
  2. Publish a terminal `TurnResult` for the conversation whose `error` field contains a user-visible message of the form `"exceeded retry cap: <last error>"`.
  3. Terminate the bus message via the bus's terminate operation so JetStream does not redeliver it.

The cap value `MAX_TRANSIENT_DELIVERIES` SHALL match the bus's `max_deliver` cap so the two layers reinforce each other.

#### Scenario: Transient error within cap is nacked normally

- **WHEN** the worker fails a turn with a transient error and `delivery_count = 3`
- **THEN** the worker calls `bus.nack_delayed(msg_id, 120s)`
- **AND** does not publish a terminal `TurnResult`

#### Scenario: Transient error at cap publishes terminal result and terminates

- **WHEN** the worker fails a turn with a transient error and `delivery_count = 10`
- **THEN** the worker publishes a `TurnResult` carrying `error = "exceeded retry cap: <last error>"`
- **AND** the worker calls `bus.fail(msg_id)` (terminate, not nack)
- **AND** an `error!` log entry records the conversation id, delivery count, and underlying error

#### Scenario: Permanent error short-circuits regardless of delivery_count

- **WHEN** the worker fails a turn with an error that the classifier reports as permanent
- **AND** `delivery_count = 1`
- **THEN** the worker publishes a terminal `TurnResult` with the underlying error
- **AND** terminates the message
- **AND** does not nack
