## Purpose

How people see, edit and share artifacts in the Flutter app, and how
artifacts appear in chat and on messenger platforms.

## ADDED Requirements

### Requirement: The app renders every artifact kind

The app SHALL render `markdown`, `code` and `data` natively, and `html` and
`slides` in a sandboxed frame loading a signed render URL: an `<iframe>` whose
`sandbox` attribute grants only `allow-scripts` on web, and a webview on macOS
and iOS. It SHALL show loading, error and empty states.

#### Scenario: Slides in the app

- **WHEN** a user opens a `slides` artifact
- **THEN** the deck appears in the frame and arrow keys move between slides

#### Scenario: Render URL expires while open

- **WHEN** the frame reloads after its URL has expired
- **THEN** the app fetches a new render URL and reloads without an error

### Requirement: People with edit can change content and resolve conflicts

The app SHALL offer a source editor to users with `edit`. Saving SHALL send the
version the editor started from as `base_version`. On `409` the app SHALL show
the user's text next to the latest version and offer "Reapply on latest" or
"Discard mine".

#### Scenario: Conflict during a Persona edit

- **GIVEN** a user is editing version 3 and the Persona creates version 4
- **WHEN** the user saves
- **THEN** the conflict view opens with version 4 and the user's text

### Requirement: Versions, comments and sharing are available in the viewer

The viewer SHALL provide a version picker with a diff against the previous
version, a comment sidebar with anchored threads, and, for `manage` users, a
share dialog for grants and links.

#### Scenario: Editor opens sharing

- **WHEN** a user with `edit` but not `manage` opens the viewer
- **THEN** no share dialog is offered

### Requirement: Chat shows artifacts the turn changed

When the chat stream delivers an `artifact` event, the message list SHALL show
an artifact card with title, kind and version. Opening the card SHALL show the
artifact beside the chat at widths of 900 px or more, and as a full-screen route
below that. Routes `/artifacts` and `/artifacts/:id` SHALL exist.

#### Scenario: Narrow window

- **WHEN** a user on a 600 px window opens an artifact card
- **THEN** the artifact opens full screen with back navigation to the chat

### Requirement: Messenger platforms receive a link

Slack, Matrix, Mattermost, Nextcloud, Signal and the CLI SHALL show each
artifact a turn changed as one line, `<title> (v<n>): <public_url>/artifacts/<id>`,
appended to the reply. Artifact content SHALL NOT be posted inline. Without a
configured public URL the line SHALL name the artifact without a link.

#### Scenario: Slack reply

- **GIVEN** a Slack turn creates artifact "Q4 plan"
- **WHEN** the reply is posted
- **THEN** it ends with `Q4 plan (v1): https://assistant.example.com/artifacts/<id>`
