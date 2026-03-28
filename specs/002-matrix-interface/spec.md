# Feature Specification: Matrix Interface

**Feature Branch**: `002-matrix-interface`
**Created**: 2026-03-28
**Status**: Draft
**Input**: User description: "we want to create a new interface for matrix"

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Chat with Assistant in Matrix Room (Priority: P1)

A user in a Matrix room sends a message mentioning or directly addressing the assistant bot. The assistant processes the message, runs the appropriate tools or reasoning, and replies in the same room with a useful response.

**Why this priority**: This is the core value of the interface — without basic message exchange, nothing else matters.

**Independent Test**: Can be fully tested by sending a question to the bot in a Matrix room and verifying a relevant answer is returned in the same room.

**Acceptance Scenarios**:

1. **Given** the assistant bot is present in a Matrix room, **When** a user sends a message addressing the bot, **Then** the bot replies with a response in the same room within a reasonable time.
2. **Given** the bot is addressed, **When** the message requires tool usage (e.g., web search, file lookup), **Then** the bot completes the tool execution and returns the result in the conversation.
3. **Given** the bot encounters an error processing a request, **When** the failure occurs, **Then** the bot notifies the user with a clear, human-readable error message rather than silently failing.

---

### User Story 2 - Private Direct Message Conversations (Priority: P2)

A user can send a direct message (1:1) to the assistant bot in Matrix. The bot responds in the direct message channel, keeping the conversation private and isolated from group rooms.

**Why this priority**: Many users prefer private conversations with an assistant, especially when sharing sensitive context or personal queries.

**Independent Test**: Can be tested by opening a direct message conversation with the bot and verifying responses stay in that private channel.

**Acceptance Scenarios**:

1. **Given** a user starts a direct message conversation with the bot, **When** the user sends a message, **Then** the bot replies in the same direct message thread.
2. **Given** a user has ongoing conversations in both a group room and a DM, **When** messages arrive in each, **Then** the bot maintains separate conversation contexts for each channel.

---

### User Story 3 - Multi-Room Deployment (Priority: P3)

An administrator can configure the assistant bot to be active in multiple Matrix rooms simultaneously, allowing different teams or use cases to share one bot deployment.

**Why this priority**: Operational efficiency — running a single bot across many rooms reduces maintenance overhead and deployment cost.

**Independent Test**: Can be tested by inviting the bot to two separate rooms and verifying it responds correctly and independently in each.

**Acceptance Scenarios**:

1. **Given** the bot is invited to two different Matrix rooms, **When** users in each room send messages, **Then** the bot responds in each room independently with correct context.
2. **Given** the bot is active in multiple rooms, **When** a conversation occurs in room A, **Then** context from room A does not leak into responses in room B.

---

### Edge Cases

- What happens when the bot is removed from a room mid-conversation?
- How does the system handle duplicate message delivery due to network retries?
- What happens if the Matrix homeserver is temporarily unreachable?
- What happens when a message contains only an attachment with no text?
- How does the bot avoid responding to its own messages (preventing infinite loops)?

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: The assistant MUST be able to connect to a Matrix homeserver using configurable credentials.
- **FR-002**: The assistant MUST receive messages sent in Matrix rooms where it is a member and respond to them.
- **FR-003**: The assistant MUST maintain separate conversation contexts for each Matrix room.
- **FR-004**: The assistant MUST support direct message (1:1) conversations with individual users.
- **FR-005**: The assistant MUST reply in the same room or channel where the original message was sent.
- **FR-006**: The assistant MUST handle disconnections from the Matrix homeserver gracefully and reconnect automatically.
- **FR-007**: The assistant MUST log connection events and message processing errors for operator visibility.
- **FR-008**: Operators MUST be able to configure the homeserver URL, bot credentials, and room restrictions via configuration file or environment variables.
- **FR-009**: The assistant MUST ignore messages sent by other bots and its own messages to prevent response loops.
- **FR-010**: The assistant MUST support being invited to new rooms at runtime without requiring a restart.

### Key Entities

- **Matrix Room**: A communication channel on a Matrix homeserver where the bot participates; identified by a room ID; has members and message history.
- **Matrix User**: A participant on the Matrix network identified by a Matrix user ID; can be a human or another bot.
- **Conversation Session**: The contextual state maintained per room or per DM thread, mapping Matrix room identifiers to ongoing assistant conversation history.
- **Bot Configuration**: The set of settings (homeserver URL, access token, allowed rooms, bot user ID) required to connect and operate the interface.

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: Users receive a response from the assistant within 30 seconds of sending a message under normal load conditions.
- **SC-002**: The bot automatically reconnects and resumes operation within 60 seconds of a homeserver connectivity interruption.
- **SC-003**: 100% of messages sent to the bot in configured rooms result in either a response or a visible error notification — no silent message drops.
- **SC-004**: Conversation context is correctly isolated across rooms — responses in one room never reference content from a different room.
- **SC-005**: The interface can be deployed and configured by an operator in under 15 minutes using only documentation and configuration files.

## Assumptions

- Users already have a Matrix homeserver available (self-hosted or a hosted service).
- The bot will be registered as a dedicated Matrix account on the homeserver before deployment.
- Matrix rooms where the bot operates are configured by administrators; end users do not need special permissions beyond room membership to interact with the bot.
- The existing assistant runtime and tool execution infrastructure will be reused without modification.
- Authentication to the Matrix homeserver uses a long-lived access token; interactive login flows are out of scope for v1.
- End-to-end encrypted rooms are out of scope for v1; the bot will only operate in unencrypted rooms.
- Voice/audio message transcription in Matrix is out of scope for v1.
- The interface follows the same structural patterns as existing interfaces (Slack, Mattermost) in this project.
