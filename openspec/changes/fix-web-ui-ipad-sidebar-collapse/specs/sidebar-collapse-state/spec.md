## ADDED Requirements

### Requirement: Sidebar collapse state persists across reloads

`sidebarCollapsedProvider` SHALL read its initial value from `SharedPreferences` under the key `assistant.sidebarCollapsed` and SHALL write back on every state change. A missing key SHALL default to `false` (expanded). Read or write failures SHALL be non-fatal: the provider continues with in-memory state.

#### Scenario: Cold start with no stored value

- **GIVEN** `assistant.sidebarCollapsed` is absent from `SharedPreferences`
- **WHEN** `sidebarCollapsedProvider.build()` runs
- **THEN** the resolved state SHALL be `false` (expanded)

#### Scenario: Hard reload after collapse

- **GIVEN** the user has toggled the sidebar to collapsed AND `assistant.sidebarCollapsed == true` is persisted
- **WHEN** the app is reloaded
- **THEN** `sidebarCollapsedProvider` SHALL resolve to `true` AND the sidebar SHALL render in its collapsed form on first paint

#### Scenario: Toggle writes through to storage

- **WHEN** any code calls `sidebarCollapsedProvider.notifier.toggle()`
- **THEN** the new boolean value SHALL be written to `SharedPreferences` under `assistant.sidebarCollapsed`

#### Scenario: Storage write failure is non-fatal

- **WHEN** the platform rejects the SharedPreferences write
- **THEN** the provider SHALL continue with the updated in-memory state AND SHALL NOT throw

### Requirement: Sidebar exposes a top-leading toggle on Material wide layouts

On the Material wide branch of `NavShell` (web/macOS, viewport >= 768 dp), the shell SHALL render an unconditional toggle affordance in the **main content area's top-leading corner**, in addition to the existing toggle inside the sidebar. The button SHALL show `Icons.menu` when collapsed and `Icons.menu_open` when expanded with a tooltip reading `"Show navigation"` / `"Hide navigation"` so it can be uniquely targeted in widget tests without colliding with the in-sidebar `"Expand sidebar"` / `"Collapse sidebar"` tooltip.

#### Scenario: iPad-landscape viewport shows the top-leading toggle

- **GIVEN** the app is rendered at 1180×820 (iPad landscape) on the web platform
- **THEN** an `IconButton` with tooltip `"Hide navigation"` SHALL be visible at the top-leading corner of the main content area (outside the sidebar)

#### Scenario: Tapping the top-leading toggle collapses the sidebar

- **WHEN** the user taps the top-leading toggle while the sidebar is expanded
- **THEN** `sidebarCollapsedProvider` SHALL transition to `true` AND the sidebar SHALL animate to its 72 dp collapsed width

#### Scenario: Toggle remains visible while collapsed

- **GIVEN** the sidebar is collapsed
- **THEN** the top-leading toggle SHALL still be visible AND its tooltip SHALL be `"Show navigation"`

### Requirement: Swipe-from-left-edge toggles the sidebar on touch input

On touch input devices (`PointerDeviceKind.touch`), a horizontal drag originating in the left 20 logical pixels of the viewport SHALL toggle `sidebarCollapsedProvider` when the drag exceeds 40 logical pixels in either direction within a single continuous gesture (no time cap — slow drags also qualify; the `within 250 ms` reading in the scenarios below is illustrative of a typical swipe, not a normative constraint). The toggle SHALL fire at most once per gesture so a long drag does not flip the state repeatedly.

#### Scenario: Drag right from left edge expands the sidebar

- **GIVEN** the sidebar is collapsed AND the user touches at x=10
- **WHEN** the touch drags to x=80 within 250 ms
- **THEN** `sidebarCollapsedProvider` SHALL transition to `false`

#### Scenario: Drag left from left edge collapses the sidebar

- **GIVEN** the sidebar is expanded AND the user touches at x=18
- **WHEN** the touch drags to x=-40 (off-screen) within 250 ms
- **THEN** `sidebarCollapsedProvider` SHALL transition to `true`

#### Scenario: Drag starting outside the edge zone is ignored

- **GIVEN** the user touches at x=200 (well past the edge zone)
- **WHEN** the touch drags horizontally
- **THEN** `sidebarCollapsedProvider` SHALL NOT change AND the underlying scrollable SHALL handle the gesture normally

#### Scenario: Mouse drag is ignored on non-touch platforms

- **GIVEN** the user is on web with a mouse-only pointer
- **WHEN** the cursor performs a horizontal drag from x=5
- **THEN** the sidebar state SHALL NOT change (mouse users have the top-leading toggle)
