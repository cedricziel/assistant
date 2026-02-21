# assistant-interface-signal

Signal messenger interface for the AI assistant.

This crate provides a **second full interface** (parallel to the CLI REPL) that
receives messages via Signal and routes them through the `ReactOrchestrator`,
replying in the same Signal thread.

---

## Prerequisites

1. **A registered Signal account** — a phone number already linked to Signal.
2. **Device linking** — you must run `assistant-signal link` once before starting
   the listener (see below).
3. **Rust toolchain** — the full feature build requires `presage` from git (see
   [Building](#building)).

---

## Building

### Stub only (compiles for all targets, no Signal dependency)

```sh
cargo build -p assistant-interface-signal
```

The binary will compile but `link` and `run` will return an informative error
message asking you to enable the feature.

### Full Signal integration

The presage integration is gated behind `--features signal`. The presage crates
are fetched directly from git and are **not** published to crates.io.

```sh
cargo build -p assistant-interface-signal --features signal
```

> **Note** The presage crates require additional system dependencies (OpenSSL,
> libsqlite3). Ensure they are installed before building.

---

## Device linking

Before starting the listener, link this machine as a Signal secondary device.

```sh
./target/debug/assistant-signal link --device-name "AssistantBot"
```

1. A QR code is printed to the terminal.
2. Open the Signal app on your phone.
3. Navigate to **Settings → Linked Devices → Link a device**.
4. Scan the QR code.
5. The command exits once the handshake completes.

The linked device state is stored in the path configured by `store_path` in
`~/.assistant/config.toml` (default: `~/.assistant/signal-store`).

---

## Running the listener

Once the device is linked:

```sh
./target/debug/assistant-signal run
```

Any Signal message sent to the linked number is dispatched to the
`ReactOrchestrator` and the reply is sent back to the sender.

---

## Configuration

Add a `[signal]` section to `~/.assistant/config.toml`:

```toml
[signal]
# Phone number of the Signal account (informational; not used for auth).
phone_number = "+14155550123"

# Optional allowlist — only these sender identifiers may interact with the bot.
# Use UUID strings (e.g. "550e8400-e29b-41d4-a716-446655440000").
# Leave empty to accept all contacts.
allowed_senders = []

# Custom path for the presage SQLite store.
# Defaults to ~/.assistant/signal-store if omitted.
# store_path = "/var/lib/assistant/signal-store"
```

---

## Safety

- The `shell-exec` skill is **blocked** on the Signal interface by
  `SafetyGate` — it cannot be invoked via incoming messages.
- Any skill marked `confirmation_required` is **auto-denied** (the
  `AutoDenyConfirmation` callback always returns `false`).
- `allowed_senders` provides an additional layer of access control.

---

## Architecture

```
Signal message
     │
     ▼
assistant-signal (binary)
     │
     ├── link subcommand ──► presage::Manager::link_secondary_device
     │                              │
     │                        QR code printed to terminal
     │
     └── run subcommand ──► SignalInterface::run()
                                    │
                         presage::Manager::receive_messages()
                                    │  (streaming)
                                    ▼
                         ReactOrchestrator::run_turn_streaming()
                                    │
                                    ▼
                         presage::Manager::send_message()
```

---

## Verification steps

```sh
# Stub build (no presage deps needed):
cargo build -p assistant-interface-signal

# Full build:
cargo build -p assistant-interface-signal --features signal

# Lint (full feature set):
cargo clippy -p assistant-interface-signal --features signal -- -D warnings

# Link device (requires Signal app):
./target/debug/assistant-signal link --device-name "AssistantBot"

# Start listener:
./target/debug/assistant-signal run
# → send a Signal message to the linked number → bot replies
```
