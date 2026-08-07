## ADDED Requirements

### Requirement: Element package carries no state-management dependency

The `assistant_ui` package SHALL declare no dependency on `flutter_riverpod` or `assistant_api`. Every widget it exports SHALL receive its data through constructor parameters or an inherited scope defined within the package, and SHALL emit user intent through callbacks. No exported widget may read application providers.

#### Scenario: Package manifest excludes state management

- **WHEN** `app/packages/assistant_ui/pubspec.yaml` is inspected
- **THEN** its `dependencies` SHALL NOT include `flutter_riverpod` or `assistant_api`

#### Scenario: Elements render without a ProviderScope

- **GIVEN** a widget test that pumps any exported element with literal constructor arguments
- **WHEN** the element is pumped without any `ProviderScope` ancestor
- **THEN** it SHALL build and render without error

#### Scenario: Turn progress card takes data by parameter

- **GIVEN** `TurnProgressCard`, which today reads `chatProvider` via `ref`
- **WHEN** it is exported from `assistant_ui`
- **THEN** it SHALL accept its turn status as a constructor parameter AND contain no `ref` usage

### Requirement: Timeline entry shell owns presentation state; bodies are stateless

`TimelineEntryShell` SHALL be the single owner of per-entry presentation state: expansion, user pinning, auto-collapse timing, reduced-motion handling, and `EntryState` reaction. Entry bodies SHALL be stateless widgets that render only the fields belonging to their own `TimelineEntryType`.

#### Scenario: Active entry auto-expands, complete entry auto-collapses

- **GIVEN** a `TimelineEntryShell` at `TimelineDensity.normal` in `EntryState.active`
- **WHEN** its state transitions to `EntryState.complete`
- **THEN** the shell SHALL collapse after its auto-collapse delay

#### Scenario: User pinning survives state transitions

- **GIVEN** a `TimelineEntryShell` the user has manually expanded
- **WHEN** its state transitions to `EntryState.complete` or `EntryState.stale`
- **THEN** the shell SHALL remain expanded

#### Scenario: Reduced motion collapses without delay

- **GIVEN** `MediaQuery.disableAnimations` is true
- **WHEN** an unpinned entry transitions from `active` to `complete`
- **THEN** the shell SHALL collapse immediately rather than scheduling a timer

#### Scenario: Bodies hold no presentation state

- **WHEN** any entry body widget is inspected
- **THEN** it SHALL be a `StatelessWidget` AND SHALL NOT reference expansion, pinning, timers, or `MediaQuery`

### Requirement: Density is supplied by an inherited scope

`TimelineDensity` SHALL be provided to elements by `ThreadDensityScope` rather than threaded as a constructor parameter through intermediate widgets. Elements SHALL resolve density from the nearest enclosing scope.

#### Scenario: Density resolves from the scope

- **GIVEN** a `ThreadDensityScope` providing `TimelineDensity.compact`
- **WHEN** a descendant element that varies by density is built
- **THEN** it SHALL render its compact presentation

#### Scenario: Density change rebuilds dependents

- **GIVEN** a mounted `ThreadDensityScope` providing `TimelineDensity.normal`
- **WHEN** the scope is rebuilt providing `TimelineDensity.expanded`
- **THEN** descendant elements that depend on density SHALL rebuild

### Requirement: Reasoning panel renders discrete steps with elapsed time

`ReasoningPanel` SHALL render streamed reasoning as a collapsible sequence of discrete steps, each showing elapsed time, rather than as a single undifferentiated text blob.

#### Scenario: Multiple reasoning steps render separately

- **GIVEN** a reasoning entry containing more than one step
- **WHEN** `ReasoningPanel` renders it expanded
- **THEN** each step SHALL be individually visible

#### Scenario: Elapsed time is shown while reasoning is active

- **GIVEN** a `ReasoningPanel` in `EntryState.active`
- **WHEN** it renders
- **THEN** it SHALL display an elapsed-time indication for the in-progress step

#### Scenario: Empty reasoning renders nothing

- **GIVEN** a reasoning entry with no steps
- **WHEN** `ReasoningPanel` renders it
- **THEN** it SHALL render no visible panel

### Requirement: Thread viewport anchors scrolling and offers recovery

`ThreadViewport` SHALL keep the newest content visible while the user is at the bottom of the thread, SHALL NOT move the viewport when the user has scrolled away, and SHALL surface a "jump to latest" affordance whenever the user is scrolled away while new content arrives.

#### Scenario: At bottom, new content stays visible

- **GIVEN** the viewport is scrolled to the bottom
- **WHEN** new content is appended
- **THEN** the viewport SHALL remain at the bottom

#### Scenario: Scrolled away, the viewport does not jump

- **GIVEN** the user has scrolled away from the bottom
- **WHEN** new content is appended
- **THEN** the scroll offset SHALL NOT change

#### Scenario: Recovery pill appears and returns the user

- **GIVEN** the user has scrolled away from the bottom AND new content has arrived
- **WHEN** the viewport rebuilds
- **THEN** a "jump to latest" affordance SHALL be visible AND activating it SHALL return the viewport to the bottom AND hide the affordance

### Requirement: Gallery exercises every element in every state

The `app/widgetbook/` application SHALL provide a runnable entry for every widget exported by `assistant_ui`. For elements that vary by density and entry state, it SHALL expose the full `EntryState` × `TimelineDensity` matrix.

#### Scenario: Every exported element has a gallery entry

- **WHEN** the exports of `assistant_ui` are compared against the gallery's registered entries
- **THEN** every exported widget SHALL have at least one entry

#### Scenario: Timeline entries expose the full state matrix

- **WHEN** the gallery entry for a timeline entry element is opened
- **THEN** each combination of `EntryState` (`active`, `complete`, `stale`) and `TimelineDensity` (`compact`, `normal`, `expanded`) SHALL be selectable

#### Scenario: Gallery is excluded from the shipped web build

- **WHEN** the embedded SPA is built for release
- **THEN** the widgetbook application SHALL NOT be included in the output

### Requirement: Chat screen is a thin adapter over the element package

`ChatScreen` SHALL retain application wiring — provider reads, routing, navigation chrome and layout — and SHALL delegate rendering of the conversation surface to `assistant_ui` elements.

#### Scenario: Screen keeps wiring, elements keep rendering

- **WHEN** `chat_screen.dart` is inspected after the extraction
- **THEN** conversation-surface rendering SHALL be delegated to exported elements rather than to private widgets declared in the screen file

#### Scenario: Existing chat behaviour is preserved

- **GIVEN** the chat widget test suite as it exists before this change
- **WHEN** the suite is run against the extracted elements with imports updated
- **THEN** all previously passing assertions SHALL still pass

### Requirement: Composer emits intent by callback and performs no I/O

`Composer` SHALL render the message input surface — text field, send and stop affordances, attachment tray, slash-command menu and voice affordance — receiving its state by constructor parameter and emitting every user intent as a callback. It SHALL NOT record audio, pick files, read the file system, or call any API.

#### Scenario: Send emits a callback rather than dispatching

- **GIVEN** a `Composer` with non-empty input text
- **WHEN** the send affordance is activated
- **THEN** its `onSend` callback SHALL fire AND no API call SHALL be made by the element

#### Scenario: Voice affordance is presentation only

- **GIVEN** `ComposerVoiceButton` rendered with `isRecording` false
- **WHEN** the mic affordance is activated
- **THEN** `onStart` SHALL fire AND the element SHALL NOT itself start a recorder

#### Scenario: Recording state is driven by the caller

- **GIVEN** `ComposerVoiceButton` rendered with `isRecording` true and an elapsed duration
- **WHEN** it builds
- **THEN** it SHALL display the recording countdown and a stop affordance

#### Scenario: Attachment removal is delegated

- **GIVEN** an `AttachmentTray` rendered with two pending attachments
- **WHEN** the remove affordance on the first is activated
- **THEN** `onRemove` SHALL fire with index 0 AND the tray SHALL NOT mutate the list itself

#### Scenario: Slash-command menu filters a supplied list

- **GIVEN** a `SlashCommandMenu` supplied with a list of commands
- **WHEN** a filter string is applied
- **THEN** only matching commands SHALL be shown AND selecting one SHALL fire `onSelected` without dispatching it

### Requirement: The element index records a verdict for every catalogue element

The package SHALL carry `ELEMENTS.md` recording, for each of the 59 assistant-ui catalogue elements, one of the verdicts Adopt, Adopt (have), Defer, or Decline. Every Defer SHALL name the prerequisite blocking it; every Decline SHALL give a reason.

#### Scenario: Every catalogue element has a verdict

- **WHEN** `ELEMENTS.md` is inspected
- **THEN** all 59 catalogue elements SHALL appear with exactly one verdict each

#### Scenario: Deferred elements name their blocker

- **WHEN** an element carries the Defer verdict
- **THEN** its row SHALL reference a named prerequisite

#### Scenario: Adopted elements are reflected in the index

- **GIVEN** a widget exported from `assistant_ui`
- **WHEN** the index is compared against the package exports
- **THEN** the corresponding element SHALL carry an Adopt or Adopt (have) verdict
