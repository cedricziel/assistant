# web-sse-eventsource Specification

## Purpose

TBD - created by archiving change web-sse-eventsource. Update Purpose after archive.

## Requirements

### Requirement: Web SSE consumption uses the browser's `EventSource` API

On the web platform, `streamConversations()` SHALL use the browser's native `EventSource` API to consume `/api/conversations/stream`. Native platforms (mac, iOS) SHALL keep the existing `dio` stream path. Selection MUST be at compile time via conditional imports — no runtime `kIsWeb` checks in the public API surface.

#### Scenario: Web — conversation snapshot delivered via EventSource

- **GIVEN** the user is on web AND has a usable active context
- **WHEN** `streamConversations()` is called
- **THEN** the underlying transport SHALL be `EventSource(url)` AND the first emitted `ConversationListEvent` SHALL be `ConversationSnapshotEvent` with the server's full conversation list within 500ms

#### Scenario: Native — no behavior change

- **WHEN** running on macOS or iOS
- **THEN** `streamConversations()` SHALL use the existing dio stream path AND `package:web` SHALL NOT be loaded

#### Scenario: Same `ConversationListEvent` types regardless of transport

- **WHEN** any caller (chat provider, tests) consumes the stream
- **THEN** the events SHALL be `ConversationSnapshotEvent`, `ConversationUpsertedEvent`, or `ConversationDeletedEvent` — identical types/shape on web and native

### Requirement: SSE authentication uses `?access_token=<jwt>` on web

`EventSource` cannot send custom headers. The web SSE adapter SHALL append the active JWT as a `?access_token=<jwt>` query parameter to the SSE URL. The server's auth middleware SHALL accept this parameter as equivalent to `Authorization: Bearer <jwt>` for SSE endpoints.

#### Scenario: Web SSE includes the access token in the URL

- **GIVEN** the active context has `oauthCredentials.bearerToken == "abc.def"`
- **WHEN** the EventSource adapter constructs the SSE URL
- **THEN** the URL SHALL include `access_token=abc.def` as a query parameter

#### Scenario: Server accepts query-param auth on SSE

- **WHEN** a client requests `/api/conversations/stream?access_token=<valid_jwt>` with no `Authorization` header
- **THEN** the server SHALL authenticate the request equivalently to a request with `Authorization: Bearer <valid_jwt>` AND populate the same `AuthContext`

#### Scenario: Invalid token in query param

- **WHEN** the request includes `?access_token=<invalid>` AND no other auth
- **THEN** the server SHALL respond `401 Unauthorized`

### Requirement: EventSource close + dispose cleans up

When the chat provider disposes its subscription, the underlying `EventSource` connection SHALL be closed and SHALL NOT continue to receive events. Subsequent reconnects SHALL open a fresh `EventSource`.

#### Scenario: Subscription cancel closes EventSource

- **GIVEN** the chat provider is subscribed to the stream
- **WHEN** `subscription.cancel()` is invoked
- **THEN** the underlying `EventSource.close()` SHALL be called AND the network connection in DevTools SHALL show as closed

### Requirement: Reconnection delegated to the browser

The web SSE adapter SHALL NOT implement custom reconnection logic. `EventSource`'s built-in reconnect SHALL handle transport interruptions. The wrapper MAY surface terminal failures (e.g., 401) via the existing `_onStreamError` path so the UI banner behavior matches native.

#### Scenario: Transient disconnect is auto-resumed

- **WHEN** the network drops briefly and the EventSource fires a `error` event with `readyState == CONNECTING`
- **THEN** the adapter SHALL NOT call `onError` on the consuming stream — `EventSource` will reconnect automatically

#### Scenario: Permanent failure surfaces

- **WHEN** the EventSource fires `error` with `readyState == CLOSED` AND the server returned a 401 or 403
- **THEN** the adapter SHALL emit an `ApiAuthException` so `_handleAuthExpired` (from #685) can deactivate the session
