## ADDED Requirements

### Requirement: Conversation metadata is edited through one partial PATCH

`PATCH /api/conversations/{id}` SHALL accept `UpdateConversationRequest { title?: string, pinned?: boolean, archived?: boolean }`, apply all supplied fields in one transaction, emit exactly one `ConversationUpserted` event, and return `200` with the full `ConversationSummary`. A body with none of the fields SHALL return `400`. A body that does not deserialize (malformed JSON or a field of the wrong type) SHALL return `400`. A supplied `title` SHALL be trimmed. An empty trimmed title, or one longer than 200 Unicode scalar values, SHALL return `400`. A supplied `title` SHALL set `title_locked = 1`, whether or not the title was already locked. The route SHALL require write access to the space; a caller with read-only access SHALL receive `403`. All `400`, `403`, `404` and `409` responses from this route SHALL use the `{"error": "..."}` envelope, and so SHALL the `404` of every route that loads a conversation through the shared ownership check. A conversation owned by another user SHALL return `404`.

#### Scenario: Rename trims and locks

- **WHEN** a client sends `PATCH /api/conversations/{id}` with `{"title": "  Trip plan  "}`
- **THEN** the response is `200` with `"title": "Trip plan"`
- **AND** the row has `title_locked = 1`

#### Scenario: Empty title is rejected

- **WHEN** a client sends `{"title": "   "}`
- **THEN** the response is `400` with an `error` field
- **AND** the stored title is unchanged

#### Scenario: Empty body is rejected

- **WHEN** a client sends `{}`
- **THEN** the response is `400` with an `error` field

#### Scenario: Wrong field type is rejected with the envelope

- **WHEN** a client sends `{"pinned": "yes"}`
- **THEN** the response is `400` with an `error` field

#### Scenario: Title length counts Unicode scalar values

- **WHEN** a client sends a title of 200 `é` characters (400 UTF-8 bytes)
- **THEN** the response is `200`
- **AND** a title of 201 `é` characters returns `400`

#### Scenario: Manual rename overwrites a locked title

- **GIVEN** a conversation whose title was auto-generated and locked
- **WHEN** the user renames it to "Trip plan"
- **THEN** the stored title is "Trip plan"

#### Scenario: Read-only member cannot patch

- **GIVEN** a space member with read-only access
- **WHEN** the member sends `PATCH /api/conversations/{id}`
- **THEN** the response is `403` with an `error` field

#### Scenario: Several fields apply atomically

- **WHEN** a client sends `{"title": "Ideas", "pinned": true}`
- **THEN** the response carries `"title": "Ideas"` and a `pinned_at` timestamp
- **AND** subscribers of the conversation stream receive one `upserted` event carrying both

#### Scenario: Another user's conversation is not found

- **WHEN** user B patches a conversation owned by user A
- **THEN** the response is `404` with an `error` field

### Requirement: Metadata edits do not change recency

Renaming, pinning, unpinning, archiving, unarchiving and auto-titling SHALL NOT change a conversation's `updated_at`. Only persisting a message SHALL advance `updated_at`.

#### Scenario: Rename keeps list position

- **GIVEN** conversations A (updated 1 h ago) and B (updated 5 min ago)
- **WHEN** the user renames A
- **THEN** A's `updated_at` is unchanged
- **AND** `GET /api/conversations` still lists B before A

### Requirement: Conversations can be pinned

The system SHALL store a nullable `pinned_at` timestamp per conversation. `{"pinned": true}` SHALL set it to the current time if unset, and leave it unchanged if already set. `{"pinned": false}` SHALL clear it. `ConversationSummary` SHALL include `pinned_at` (RFC 3339) when set and omit it otherwise. `GET /api/conversations` and the stream `snapshot` SHALL order pinned conversations first by `pinned_at` descending, then the rest by `updated_at` descending.

#### Scenario: Pinned conversation lists first

- **GIVEN** conversations A (updated 1 h ago) and B (updated 5 min ago)
- **WHEN** the user pins A
- **THEN** `GET /api/conversations` returns A before B
- **AND** A carries `pinned_at`, and B has no `pinned_at` key

#### Scenario: Most recently pinned first

- **WHEN** the user pins A and then pins B
- **THEN** the list returns B, then A, then unpinned conversations

#### Scenario: Re-pinning keeps the original pin time

- **WHEN** a client sends `{"pinned": true}` for a conversation that is already pinned
- **THEN** its `pinned_at` is unchanged

#### Scenario: Unpin returns the conversation to recency order

- **WHEN** the user unpins A
- **THEN** A has no `pinned_at` and is ordered by `updated_at` among unpinned conversations

### Requirement: Conversations can be archived

The system SHALL store a nullable `archived_at` timestamp per conversation. `{"archived": true}` SHALL set it and clear `pinned_at`. `{"archived": false}` SHALL clear it. `GET /api/conversations` and the stream `snapshot` SHALL exclude archived conversations. `GET /api/conversations?archived=true` SHALL return only archived conversations, ordered by `archived_at` descending. Persisting a message with role `user`, through `save_message` or through a branch insert (edit and resend), SHALL clear `archived_at`. Messages with other roles SHALL NOT.

A conversation SHALL never be pinned and archived at once, and the store SHALL enforce this inside the update statement rather than by the order in which fields are applied. `{"pinned": true, "archived": true}` SHALL return `400`. `{"pinned": true}` for an archived conversation SHALL return `409` and change nothing. `{"pinned": true, "archived": false}` SHALL unarchive and pin.

#### Scenario: Archived conversation leaves the main list

- **WHEN** the user archives A
- **THEN** `GET /api/conversations` does not include A
- **AND** `GET /api/conversations?archived=true` includes A with `archived_at`

#### Scenario: Archiving unpins

- **GIVEN** A is pinned
- **WHEN** the user archives A
- **THEN** A has no `pinned_at`

#### Scenario: New user message unarchives

- **GIVEN** A is archived
- **WHEN** the user sends a message in A
- **THEN** A has no `archived_at` and appears in `GET /api/conversations`

#### Scenario: Assistant-only activity keeps it archived

- **GIVEN** the user archives A while a turn in A is still streaming
- **WHEN** the turn persists its assistant reply
- **THEN** A keeps its `archived_at`

#### Scenario: Edit and resend unarchives

- **GIVEN** A is archived
- **WHEN** the user edits and resends an earlier message in A
- **THEN** A has no `archived_at`

#### Scenario: Pin and archive in one request are rejected

- **WHEN** a client sends `{"pinned": true, "archived": true}`
- **THEN** the response is `400` with an `error` field
- **AND** the conversation is unchanged

#### Scenario: Pinning an archived conversation conflicts

- **GIVEN** A is archived
- **WHEN** a client sends `{"pinned": true}` for A
- **THEN** the response is `409` with an `error` field
- **AND** A keeps its `archived_at` and has no `pinned_at`

#### Scenario: Unarchive and pin together

- **GIVEN** A is archived
- **WHEN** a client sends `{"pinned": true, "archived": false}`
- **THEN** A has a `pinned_at` and no `archived_at`

#### Scenario: Archive racing a pin from another tab

- **GIVEN** tab 1 and tab 2 both show A as unpinned and unarchived
- **WHEN** tab 1 archives A and then tab 2 pins A
- **THEN** tab 2 receives `409`
- **AND** A stays archived and unpinned

### Requirement: The conversation stream carries list metadata

The `snapshot` and `upserted` events of `GET /api/conversations/stream` SHALL carry the same `ConversationSummary` shape as the REST endpoints, including `pinned_at` and `archived_at` when set. Archiving or unarchiving SHALL emit `upserted`, not `deleted`. The snapshot SHALL exclude archived conversations. The stream SHALL forward `deleted` for every conversation the caller owns, archived or not, including conversations archived before the stream was opened. Summaries for REST and stream SHALL be built by a single projection from `ConversationRecord`.

#### Scenario: Pin reaches other tabs

- **GIVEN** two clients subscribed to the stream for the same user
- **WHEN** client 1 pins A
- **THEN** client 2 receives `upserted` for A carrying `pinned_at`

#### Scenario: Archive is an upsert

- **WHEN** the user archives A
- **THEN** subscribers receive `upserted` for A carrying `archived_at`
- **AND** no `deleted` event is sent for A

#### Scenario: Reconnect heals missed events

- **GIVEN** a client missed the `upserted` for a pin while disconnected
- **WHEN** it reconnects
- **THEN** the new `snapshot` lists A first with `pinned_at`

#### Scenario: Deleting an archived conversation reaches a later stream

- **GIVEN** conversation A was archived
- **AND** client 1 opened the stream after that, so its snapshot does not list A
- **WHEN** client 2 deletes A
- **THEN** client 1 receives `deleted` for A

#### Scenario: Older clients ignore new fields

- **WHEN** a client that predates this change parses an `upserted` carrying `pinned_at`
- **THEN** it parses the entry without error and ignores the unknown key

### Requirement: The conversation list groups pinned and archived conversations

The Flutter conversation list SHALL render a "Pinned" group above the remaining conversations when any are pinned, and a collapsed "Archived" section at the bottom. Expanding it loads `GET /api/conversations?archived=true`. When an `upserted` event arrives, the client SHALL place the conversation in the group its `pinned_at` / `archived_at` indicate, and sort it with the server's ordering. A `deleted` event SHALL remove the conversation from whichever list holds it. While the "Archived" section is expanded, the client SHALL refetch `?archived=true` on every new `snapshot`. The package `ConversationListView` SHALL render the groups from `ConversationListItem.pinnedAt` and SHALL take the archived section through a slot. It SHALL remain free of platform branching and I/O.

#### Scenario: Pinned group header

- **GIVEN** the list has one pinned and two unpinned conversations
- **WHEN** the list renders
- **THEN** a "Pinned" header precedes the pinned row
- **AND** the unpinned rows follow in `updatedAt` order

#### Scenario: Upsert moves a row between groups

- **WHEN** an `upserted` event sets `pinned_at` on an unpinned row
- **THEN** the row moves into the "Pinned" group without a full reload

#### Scenario: Upsert with archived_at removes the row

- **WHEN** an `upserted` event carries `archived_at` for a row in the main list
- **THEN** the row disappears from the main list
- **AND** it appears in the "Archived" section if that section is expanded

#### Scenario: Upsert without archived_at moves a row back

- **GIVEN** the "Archived" section is expanded and lists A
- **WHEN** an `upserted` event for A carries no `archived_at` (an explicit unarchive or a new user message)
- **THEN** A leaves the "Archived" section
- **AND** A appears in the main list in server order

#### Scenario: Deleted removes an archived row

- **GIVEN** the "Archived" section is expanded and lists A
- **WHEN** a `deleted` event for A arrives
- **THEN** A disappears from the "Archived" section

#### Scenario: Reconnect refreshes the expanded Archived section

- **GIVEN** the "Archived" section is expanded
- **WHEN** the stream reconnects and a new `snapshot` arrives
- **THEN** the client requests `GET /api/conversations?archived=true` again and replaces the section's rows

#### Scenario: Archived section loads lazily

- **WHEN** the list first renders
- **THEN** no `?archived=true` request is made until the user expands "Archived"

### Requirement: Conversation rows offer platform-appropriate actions

Each conversation row SHALL offer Rename, Pin/Unpin, Archive/Unarchive and Delete. The package `ConversationListView` SHALL emit `onRequestActions(ConversationActionRequest)` on long-press, on secondary tap, and from a trailing overflow button. `ConversationActionRequest` SHALL carry the `item`, the `PointerDeviceKind` of the pointer that started the gesture (`null` for keyboard activation), the global pointer `position` (`null` for the overflow button), and the overflow button's global `anchorRect` (`null` for row gestures). The button is shown while the row is hovered, focused or selected. The view SHALL emit `onTogglePin(item)` on a start-to-end swipe and keep the row in place, and SHALL keep end-to-start swipe as delete with caller-supplied confirmation.

The app SHALL choose the presentation from the request:

- mouse or trackpad, on any platform: a popup menu at `position`, or anchored to `anchorRect` for the overflow button;
- touch or stylus, on any platform including mobile web: `showAdaptiveActionSheet` (Cupertino on Apple touch, Material elsewhere);
- no pointer (keyboard) on web, macOS, Windows or Linux: a popup menu anchored to `anchorRect`;
- no pointer on iOS or Android: `showAdaptiveActionSheet`.

Rename SHALL open `showAdaptiveTextInputDialog`, prefilled with the current title, with the text selected. Rename SHALL NOT submit an empty title or one longer than 200 Unicode scalar values.

#### Scenario: Right-click opens a menu on desktop

- **GIVEN** the web app with a mouse
- **WHEN** the user right-clicks a row
- **THEN** a menu with Rename, Pin, Archive and Delete opens at the pointer

#### Scenario: Long-press with a finger on mobile web opens a sheet

- **GIVEN** the web app on a touch device
- **WHEN** the user long-presses a row with a touch pointer
- **THEN** `onRequestActions` carries `kind == touch`
- **AND** an action sheet opens, not a popup menu

#### Scenario: Overflow button anchors the menu on desktop

- **GIVEN** the web app with a mouse
- **WHEN** the user clicks a row's overflow button
- **THEN** `onRequestActions` carries the button's `anchorRect` and a `null` position
- **AND** the popup menu opens anchored to that rect

#### Scenario: Long-press opens an action sheet on iOS

- **GIVEN** the app on iOS
- **WHEN** the user long-presses a row
- **THEN** a Cupertino action sheet with the same actions opens

#### Scenario: Overflow button is keyboard reachable

- **WHEN** a row has keyboard focus
- **THEN** its overflow button is visible and focusable
- **AND** activating it emits `onRequestActions` with a `null` kind, a `null` position and the button's `anchorRect`

#### Scenario: Swipe right pins without removing the row

- **WHEN** the user swipes a row start-to-end
- **THEN** `onTogglePin` fires once
- **AND** the row stays in the list

#### Scenario: Rename dialog submits a trimmed title

- **WHEN** the user picks Rename, types " Ideas ", and confirms
- **THEN** the app sends `PATCH {"title": "Ideas"}`
- **AND** the confirm button is disabled while the field is blank

#### Scenario: Failed action shows an error

- **WHEN** a PATCH from a row action fails
- **THEN** the app shows an adaptive snack bar with the error
- **AND** the row keeps its previous state

### Requirement: Rename dialog is a façade helper

`app/lib/shared/platform/adaptive_dialog.dart` SHALL provide `showAdaptiveTextInputDialog({title, initialValue, confirmLabel, maxLength})`. It returns the entered text, or `null` on cancel. It renders a `CupertinoAlertDialog` with a `CupertinoTextField` on Apple touch platforms and an `AlertDialog` with a `TextField` elsewhere. Pressing Enter confirms and Esc cancels. `maxLength` SHALL count Unicode scalar values (`String.runes`), matching the server, and Confirm SHALL be disabled while the trimmed text is blank or over the limit.

#### Scenario: Cancel returns null

- **WHEN** the user cancels the dialog
- **THEN** the future completes with `null`

#### Scenario: Length limit counts scalar values

- **GIVEN** `maxLength: 200`
- **WHEN** the field holds 200 emoji that are each one scalar value
- **THEN** Confirm is enabled
- **AND** one more character disables it

#### Scenario: Platform rendering

- **WHEN** the dialog is shown on iOS
- **THEN** it renders a `CupertinoAlertDialog`
- **AND** on web it renders a Material `AlertDialog`
