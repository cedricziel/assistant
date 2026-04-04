# Feature Specification: Cross-Platform Native App Frontend

**Feature Branch**: `005-flutter-frontend`
**Created**: 2026-04-04
**Status**: Draft
**Input**: User description: "i want to replace our current web-frontend and future native app frontends with a flutter frontend that we can package"

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Real-Time Chat with the Assistant (Priority: P1)

A user opens the app, selects a persona, types a message, and receives a streaming response
from the assistant. Tool calls made by the assistant during the turn are shown as they execute.
The user can continue the conversation across multiple turns. Conversation history is accessible
from a side panel or list view.

**Why this priority**: Chat is the core value proposition. Without it, no other feature matters.
It is also the most direct validation that the app can consume the backend's streaming API.

**Independent Test**: Launch the app pointing at a running assistant server, start a new chat,
send a message, and verify that the response streams in word-by-word with visible tool call
progress indicators.

**Acceptance Scenarios**:

1. **Given** the app is connected to an assistant server, **When** the user sends a message,
   **Then** the assistant's response appears token-by-token without the user needing to wait
   for the full reply.
2. **Given** the assistant invokes a tool mid-turn, **When** the tool is executing,
   **Then** the user sees a visible in-progress indicator for that tool call before the
   final response text continues.
3. **Given** the user has prior conversations, **When** they open the conversation list,
   **Then** all previous conversations are displayed and any one can be reopened to read the
   full history.
4. **Given** the app loses its network connection mid-stream, **When** the connection
   is restored, **Then** the user is notified and can resend the last message without losing
   the conversation context.

---

### User Story 2 - Server Connection & Profile Setup (Priority: P2)

On first launch (or when no server is configured), the user is presented with a setup screen
where they enter the assistant server address and authentication token. The app validates
the connection and stores the profile for future launches without requiring re-entry.

**Why this priority**: Without a valid server connection, no other feature works. This story
gates all downstream functionality and is required before the app can be distributed to
any user.

**Independent Test**: Install the app fresh, enter a server URL and token, confirm the
connection succeeds (or fails with a clear error), then relaunch the app and verify
credentials are retained without re-entry.

**Acceptance Scenarios**:

1. **Given** the app has no saved server profile, **When** the user launches the app,
   **Then** they are taken to a connection setup screen before the main interface.
2. **Given** the user enters a valid server URL and token, **When** they tap "Connect",
   **Then** the app verifies the credentials, saves them, and navigates to the main
   chat interface.
3. **Given** the user enters an incorrect token or unreachable server, **When** they
   tap "Connect", **Then** the app shows a specific, actionable error message
   (e.g., "Server unreachable" or "Invalid token").
4. **Given** a valid profile is saved, **When** the user relaunches the app,
   **Then** they go directly to the chat interface without re-entering credentials.

---

### User Story 3 - Persona Selection & Switching (Priority: P3)

Users can view the available personas on the connected server, switch between them, and see
the currently active persona clearly indicated in the interface. Persona details (name,
description) are displayed so users understand what capabilities each persona offers.

**Why this priority**: Personas are the primary way users customise their experience.
A user who relies on a specific persona for a specific task cannot effectively use the
app without persona switching.

**Independent Test**: With the app connected to a server that has multiple personas,
navigate to the persona picker, switch to a different persona, send a message, and confirm
the assistant responds in the context of the newly selected persona.

**Acceptance Scenarios**:

1. **Given** the server has multiple personas, **When** the user opens the persona picker,
   **Then** all available personas are listed with their name and description.
2. **Given** the user selects a persona, **When** they return to the chat view,
   **Then** the active persona name is displayed and new messages are sent to that persona.
3. **Given** the server has only one persona, **When** the user opens the app,
   **Then** that persona is pre-selected and the picker is skipped or shows a single option.

---

### User Story 4 - Observability: Traces & Logs (Priority: P4)

A developer or operator can navigate to the traces screen and logs screen to inspect the
assistant's recent activity: which tools were called, how long each turn took, and what
log lines were emitted. Filters allow narrowing by time range or keyword.

**Why this priority**: Observability is essential for diagnosing issues in production
deployments but is not required for basic assistant usage. It reaches a secondary audience
(operators/developers) and depends on the chat and connection stories being stable first.

**Independent Test**: Navigate to the traces screen, locate the trace for the most recent
conversation turn, expand it, and verify that tool calls and timings are visible.

**Acceptance Scenarios**:

1. **Given** recent assistant activity exists, **When** the user opens the traces screen,
   **Then** they see a list of recent traces ordered by recency, each showing at minimum
   a timestamp, persona, and total duration.
2. **Given** a trace is selected, **When** the user expands it, **Then** individual
   span entries (tool calls, LLM turns, etc.) are visible with their durations.
3. **Given** the logs screen is open, **When** the user types a keyword filter,
   **Then** only log lines matching the keyword are displayed in real time.

---

### User Story 5 - Skill Discovery (Priority: P5)

Users can browse the skills available to the active persona, see which skills are enabled,
and read a short description of what each skill does.

**Why this priority**: Read-only skill discovery helps users understand what the assistant
can do. Skill toggling and management are deferred to a later spec.

**Independent Test**: Navigate to the skills screen, verify the skills for the active
persona are listed with names and descriptions.

**Acceptance Scenarios**:

1. **Given** the active persona has skills assigned, **When** the user opens the skills
   screen, **Then** the skill names, short descriptions, and enabled/disabled states
   are displayed.
2. **Given** the active persona has no skills, **When** the user opens the skills screen,
   **Then** an empty-state message explains that no skills are configured for this persona.

---

### Edge Cases

- What happens when the server token expires or is revoked mid-session? The app must
  detect authentication failure responses, clear stored credentials, and return the user
  to the connection setup screen with a clear explanation.
- What happens when the server is temporarily unreachable during a streaming response?
  The stream should fail gracefully with a visible error; partially received content
  must remain visible rather than disappear.
- What happens when a persona is deleted on the server while the user has an active chat
  with it? The next message send must detect the error and prompt persona re-selection.
- What happens on very slow network connections where streaming is delayed?
  A loading indicator must be shown; no blank state should persist for more than 2 seconds
  after a message is sent.
- How does the app behave when the server is reachable only over plain HTTP (not HTTPS),
  such as a local-network deployment? Connection profiles must support non-HTTPS URLs
  without blocking warnings.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: The app MUST display streaming assistant responses token-by-token as they
  arrive from the server, without buffering the full response before display.
- **FR-002**: The app MUST show visible progress indicators for tool calls executing
  during an assistant turn.
- **FR-003**: Users MUST be able to start new conversations, send messages, and view the
  full history of prior conversations.
- **FR-004**: The app MUST allow users to configure the server address and authentication
  token on first launch and persist this configuration across restarts.
- **FR-005**: The app MUST validate the server connection before saving credentials and
  display actionable error messages on failure (distinguishing unreachable server from
  invalid token).
- **FR-006**: Users MUST be able to switch between available personas, with the active
  persona clearly indicated in the chat interface.
- **FR-007**: The app MUST be packageable and distributable as a native application on
  two target platforms for the initial release: web (browser) and macOS desktop. Both
  must be buildable from the same codebase and produce distributable artefacts.
- **FR-008**: The app MUST display traces (recent assistant turns with timing and
  tool-call breakdowns) on a dedicated observability screen.
- **FR-009**: The app MUST display structured log lines on a dedicated logs screen with
  keyword filtering.
- **FR-010**: The app MUST display the skills available to the active persona with names
  and their enabled/disabled state.
- **FR-011**: The app MUST detect authentication errors (invalid or revoked token) and
  return the user to the connection setup screen with a clear message.
- **FR-012**: The app MUST function correctly when the server is accessed over plain HTTP,
  to support local and private-network deployments.

### Key Entities

- **Server Profile**: A named configuration of server URL and authentication token stored
  locally on the device. The active profile determines which backend the app communicates
  with.
- **Conversation**: An ordered sequence of user and assistant messages associated with a
  persona. Has an auto-generated title (derived from the first message) and a creation
  timestamp.
- **Message**: A single turn within a conversation — role (user/assistant), text content,
  and optionally a list of tool calls with their inputs, outputs, and status.
- **Persona**: A backend-defined configuration (name, description, system prompt, skills).
  The app displays and selects personas but does not create or edit them.
- **Skill**: A named capability available to a persona. Has a name, description, and
  enabled/disabled state within the active persona's context. Read-only from the app.
- **Trace**: A record of a single orchestration turn containing span entries for LLM
  calls, tool executions, and total duration.
- **Log Entry**: A structured log line with a timestamp, severity level, message text,
  and optional key-value metadata fields.

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: A new user can open the app, configure a server connection, and send their
  first message in under 2 minutes on first launch.
- **SC-002**: Streaming chat responses begin appearing on screen within 2 seconds of
  the user sending a message on a local-network connection.
- **SC-003**: The app is installable and fully functional as a native package on at least
  two distinct platforms without any user-facing server-side setup beyond the existing
  assistant backend.
- **SC-004**: All user-facing capabilities available in the current browser-based web UI
  are reachable from the new app (full feature parity verified by a capability checklist)
  before the current web UI is retired.
- **SC-005**: The app recovers gracefully from network interruption — the user sees a
  clear error message within 5 seconds of connectivity loss and can retry without
  restarting the app.
- **SC-006**: The traces screen loads and displays the 50 most recent traces in under
  3 seconds on a standard local-network connection.

## Assumptions

- The assistant backend already exposes or will expose (as co-deliverables of this
  feature) all required API endpoints: streaming chat, conversation history, personas,
  skills, traces, and logs. Missing API endpoints are in-scope deliverables of this
  feature, not pre-existing prerequisites.
- Authentication uses a static bearer token (`ASSISTANT_WEB_TOKEN`), matching the current
  web UI behaviour. No OAuth2, SSO, or user account system is in scope for v1.
- The existing server-rendered web UI (`assistant-web-ui`) will run in parallel with the
  new app during a transition period. Both surfaces are supported until SC-004 is met
  and the web UI is formally retired.
- Skill creation, editing, and deletion are out of scope. The app provides read-only
  skill visibility only (US5, FR-010).
- Workflow management, webhook configuration, and A2A (agent-to-agent) protocol management
  screens are out of scope for v1 and will be addressed in follow-on specs.
- The app connects to exactly one server profile at a time; simultaneous multi-server
  aggregation is not in scope.
- Users are assumed to be running the assistant backend themselves (self-hosted). There
  is no cloud-hosted backend or account registration flow in scope.
