# conversation-search Specification

## Purpose

The full-text message index in `space.db`, the title and message search endpoint, its highlight segments, and the sidebar search UI that consumes it.

## Requirements

### Requirement: User and assistant messages are full-text indexed

`space.db` SHALL maintain a contentful FTS5 table `messages_fts(content, message_id UNINDEXED, conversation_id UNINDEXED)`, tokenized with `unicode61 remove_diacritics 2`. Index rows SHALL be keyed by `message_id`, never by `messages.rowid`, so that `VACUUM` and `VACUUM INTO` backups cannot desynchronise the index. It SHALL index exactly the messages whose `role` is `user` or `assistant`, whose trimmed content is non-empty, and whose content does not start with `<think>`; thinking steps are not searchable. The insert trigger, the re-insert step of the `AFTER UPDATE OF content` trigger, and the migration backfill SHALL apply that same predicate. Updates to columns other than `content` SHALL NOT touch the index. Deleting a message or a conversation SHALL remove its rows from the index. The indexed copy SHALL replace the characters U+E000 and U+E001 with a space; the stored message is unchanged.

#### Scenario: New messages become searchable

- **WHEN** a user message "Book the Lisbon flight" is saved
- **THEN** an FTS query for `lisbon` matches that message's rowid

#### Scenario: Tool and system rows are not indexed

- **WHEN** a `tool` message containing "Lisbon" is saved
- **THEN** an FTS query for `lisbon` does not match it

#### Scenario: Thinking steps and tool-call rows are not indexed

- **WHEN** an assistant row `<think>Lisbon is cheaper in May</think>` and an assistant tool-call row with empty content are saved
- **THEN** an FTS query for `lisbon` matches neither
- **AND** search results never contain `<think>` markup

#### Scenario: Index survives a VACUUM INTO round trip

- **GIVEN** a space.db where some messages were deleted, leaving rowid gaps
- **WHEN** it is copied with `VACUUM INTO`, and in the copy a message is deleted and another is edited
- **THEN** `INSERT INTO messages_fts(messages_fts) VALUES('integrity-check')` succeeds on the copy
- **AND** a search on the copy returns the expected message ids

#### Scenario: Non-content updates do not touch the index

- **WHEN** an `UPDATE messages SET <another column> = …` runs over every row
- **THEN** the `messages_fts` row count and contents are unchanged

#### Scenario: Deleted conversation leaves no index rows

- **WHEN** a conversation with indexed messages is deleted
- **THEN** `messages_fts` has no rows for those messages
- **AND** `INSERT INTO messages_fts(messages_fts) VALUES('integrity-check')` succeeds

#### Scenario: Backfill indexes existing history

- **GIVEN** a space.db with user, assistant and tool messages created before the migration
- **WHEN** the migration runs
- **THEN** user and assistant messages match FTS queries, and tool messages and thinking steps do not

### Requirement: Conversations are searchable by title and message text

`GET /api/conversations/search?q={q}&limit={n}` SHALL return a bare JSON array of `ConversationSearchSummary` (operationId `search_conversations`). It SHALL require an authenticated caller with read access to the space, and SHALL be scoped to the caller's user and the active agent, exactly like `GET /api/conversations`. `q` is split on whitespace into at most 8 tokens. Each token matches as a prefix against indexed message text and as a substring against the title. Titles and message text SHALL be compared with the same folding: case-insensitive and with diacritics removed. Every required token must match somewhere in the conversation: a token is satisfied when the conversation's title contains it or when at least one of its indexed messages does, and different tokens MAY be satisfied by different messages, or by the title and a message. Tokens of one character SHALL match titles only and SHALL NOT be required when at least one token has two or more characters; when no token has two or more characters, every token is required and only titles are searched. Thinking steps SHALL NOT be searched. FTS5 operators in `q` SHALL be treated as literal text. A conversation SHALL appear at most once. A conversation whose title matches at least one token SHALL rank before message-only conversations, and message matches SHALL rank by the best bm25 of any of the conversation's messages. Archived conversations SHALL be included and carry `archived_at`. `limit` defaults to 20 and is capped at 50. A blank `q`, a `q` longer than 200 Unicode scalar values, or one with more than 8 tokens SHALL return `400` with the `{"error": ...}` envelope.

#### Scenario: Message text match

- **GIVEN** conversation A contains the assistant message "The Lisbon itinerary is ready"
- **WHEN** the caller searches `q=lisb`
- **THEN** the result contains A with one entry in `matches` for that message

#### Scenario: Title match ranks first

- **GIVEN** conversation T is titled "Lisbon trip" and conversation M only mentions Lisbon in a message
- **WHEN** the caller searches `q=lisbon`
- **THEN** T precedes M in the result

#### Scenario: Every token must match somewhere in the conversation

- **GIVEN** conversation A has one message mentioning "Lisbon" and a later, different message mentioning "hotel"
- **AND** conversation B only mentions "Lisbon"
- **WHEN** the caller searches `q=lisbon hotel`
- **THEN** the result contains A and not B
- **AND** A's `matches` include a snippet highlighting "Lisbon" and a snippet highlighting "hotel"

#### Scenario: Tokens may be split between title and messages

- **GIVEN** conversation T is titled "Lisbon trip" and has a message mentioning "hotel"
- **WHEN** the caller searches `q=lisbon hotel`
- **THEN** the result contains T with "Lisbon" highlighted in `title_segments` and one match for the "hotel" message

#### Scenario: Thinking text is not searched

- **GIVEN** conversation A has an assistant row `<think>Lisbon is cheaper in May</think>` and no other mention of Lisbon
- **WHEN** the caller searches `q=lisbon`
- **THEN** the result does not contain A

#### Scenario: Non-ASCII titles fold like message text

- **GIVEN** conversation D is titled "Ärztetermin" and conversation P is titled "Lisbôa"
- **WHEN** the caller searches `q=ärzte`, and then `q=lisboa`
- **THEN** the first result contains D with "Ärzte" highlighted
- **AND** the second result contains P with "Lisbôa" highlighted

#### Scenario: One-character tokens do not scan messages

- **WHEN** the caller searches `q=t`
- **THEN** only conversations whose title contains "t" are returned
- **AND** no `matches` are returned

#### Scenario: A one-character token is not required alongside a longer one

- **GIVEN** conversation A has a message mentioning "Lisbon" and a title without the letter "q"
- **WHEN** the caller searches `q=lisbon q`
- **THEN** the result contains A

#### Scenario: Query syntax is literal

- **WHEN** the caller searches `q="foo" OR bar*`
- **THEN** the response is `200` and no FTS5 syntax error occurs

#### Scenario: Other users' conversations are invisible

- **GIVEN** user A has a conversation mentioning "Lisbon"
- **WHEN** user B searches `q=lisbon`
- **THEN** the result does not contain user A's conversation

#### Scenario: Blank query is rejected

- **WHEN** the caller searches `q=%20`
- **THEN** the response is `400` with an `error` field

#### Scenario: Unauthenticated search is rejected

- **WHEN** a request without credentials calls `GET /api/conversations/search?q=lisbon`
- **THEN** the response is `401` with an `error` field

### Requirement: Search results carry highlight segments

Each `ConversationSearchSummary` SHALL carry the `ConversationSummary` fields, `title_segments`, and `matches`. `matches` holds up to 3 entries `{message_id, role, created_at, segments}` ordered by rank. A segment SHALL be `{text: string, highlighted: boolean}`, and concatenating a list's `text` values SHALL yield the displayed string. Message segments SHALL come from an FTS5 snippet of at most 12 tokens around the hits, with `…` marking elided text. Highlighted segments SHALL cover exactly the matched tokens. Title segments SHALL cover the matched substrings of the original title, mapped back through the folding so that case or diacritic folding never shifts a highlight. A stored message containing U+E000 or U+E001 SHALL NOT produce spurious highlights, and no segment SHALL contain either character.

#### Scenario: Segments reconstruct the snippet

- **WHEN** a match for `lisbon` is returned in "The Lisbon itinerary is ready"
- **THEN** `segments` is `[{"text":"The ","highlighted":false},{"text":"Lisbon","highlighted":true},{"text":" itinerary is ready","highlighted":false}]`

#### Scenario: Stored sentinel characters do not split snippets

- **GIVEN** a user message "Lisbon\u{E000}plan for May" was saved
- **WHEN** the caller searches `q=lisbon`
- **THEN** exactly one segment is highlighted, with text "Lisbon"
- **AND** no segment contains U+E000 or U+E001

#### Scenario: Title-only match has no message matches

- **WHEN** a conversation matches only by title
- **THEN** its `matches` is an empty array
- **AND** `title_segments` highlights the matched substring

### Requirement: The sidebar offers conversation search

The conversation list header SHALL contain a search field. While its trimmed value is non-empty, the list SHALL be replaced by search results. `conversationSearchProvider` SHALL debounce input by 250 ms, cancel superseded requests, and expose loading, empty ("No conversations match"), error (with Retry) and data states. Each result row SHALL render the title and up to 3 message snippets, with highlighted segments in a bold, accent-colored style. Tapping a result SHALL open the conversation and clear the search. Esc or the clear button SHALL clear the query and restore the list. Visible results SHALL follow the conversation stream: a `deleted` event SHALL remove that conversation's result at once, and an `upserted` event for a visible result SHALL re-run the current query. The pure rendering (`ConversationSearchResultsView`, `HighlightedSegments`) SHALL live in `assistant_ui` with widgetbook use-cases and goldens.

#### Scenario: Typing debounces requests

- **WHEN** the user types "lis" and then "b" within 250 ms
- **THEN** exactly one request is sent, with `q=lisb`

#### Scenario: Superseded request is cancelled

- **GIVEN** a request for `q=lis` is in flight
- **WHEN** the debounced query becomes `lisb`
- **THEN** the `lis` request is cancelled and its result is never shown

#### Scenario: Empty results

- **WHEN** the search returns `[]`
- **THEN** the list area shows "No conversations match"

#### Scenario: Highlighted snippet rendering

- **WHEN** a result carries a highlighted segment "Lisbon"
- **THEN** `HighlightedSegments` renders that span with a bold weight and the accent color, and the other spans in the body style

#### Scenario: Clearing restores the list

- **WHEN** the user presses Esc in the search field
- **THEN** the query is empty and the grouped conversation list is shown again

#### Scenario: Deleted conversation leaves the results

- **GIVEN** results for `q=lisbon` show conversation A
- **WHEN** a `deleted` event for A arrives
- **THEN** A's result disappears without a new request

#### Scenario: Renamed conversation refreshes the results

- **GIVEN** results for `q=lisbon` show conversation A
- **WHEN** an `upserted` event for A arrives with a new title
- **THEN** the provider re-runs the `lisbon` query and shows the new title

#### Scenario: Selecting a result navigates

- **WHEN** the user taps a result for conversation A
- **THEN** the app navigates to `/chat/A` and the search field is cleared
