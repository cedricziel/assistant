## ADDED Requirements

### Requirement: App redirects to Context Switcher when no active context exists

The router SHALL redirect any navigation attempt to the Context Switcher screen (`/contexts`) when `activeContextProvider` is `null`. The redirect SHALL NOT occur when the user is already on the `/contexts` route.

#### Scenario: Launch with no contexts defined

- **WHEN** the app launches and no contexts exist
- **THEN** the Context Switcher screen is shown immediately
- **THEN** the chat and other main screens are not accessible

#### Scenario: Launch with contexts but none active

- **WHEN** the app launches and contexts exist but none is marked active
- **THEN** the Context Switcher screen is shown
- **THEN** the existing contexts are listed

#### Scenario: Launch with an active context

- **WHEN** the app launches and an active context is set
- **THEN** the app navigates directly to the chat screen
- **THEN** the Context Switcher screen is not shown automatically

---

### Requirement: Context Switcher lists all saved contexts

The Context Switcher screen SHALL display a scrollable list of all saved contexts. Each list item SHALL show the context name and server URL. The currently active context SHALL be visually indicated (e.g., a checkmark or highlight).

#### Scenario: Empty state shown when no contexts exist

- **WHEN** the Context Switcher screen is shown and no contexts have been created
- **THEN** an empty-state message is shown: "No contexts yet. Tap + to add one."
- **THEN** a floating action button (FAB) with a + icon is visible

#### Scenario: List shows saved contexts

- **WHEN** the user has saved one or more contexts
- **THEN** each context appears as a list tile with its name and URL
- **THEN** the active context tile displays a visual indicator

---

### Requirement: User can activate a context from the switcher

Tapping a context in the list SHALL set it as the active context and navigate the user to the main chat screen.

#### Scenario: Tapping an inactive context activates it

- **WHEN** the user taps a context that is not currently active
- **THEN** that context becomes the active context
- **THEN** the app navigates to the chat screen (`/chat`)

#### Scenario: Tapping the already-active context

- **WHEN** the user taps the context that is already active
- **THEN** the app navigates to the chat screen without changing state

---

### Requirement: User can create a context from the switcher screen

The Context Switcher screen SHALL provide a floating action button that opens a creation form. The form SHALL include fields for name, server URL, and optional auth token. Submitting the form SHALL save the context and return to the switcher.

#### Scenario: Opening the create form

- **WHEN** the user taps the FAB on the Context Switcher screen
- **THEN** a create-context dialog or bottom sheet appears
- **THEN** it contains a name field, a URL field, and an optional auth token field

#### Scenario: Successful creation from the switcher

- **WHEN** the user fills in a valid name and URL and submits
- **THEN** the new context appears in the list
- **THEN** the dialog closes

---

### Requirement: Context Switcher is accessible from the navigation rail

The navigation rail SHALL expose a "Contexts" affordance in the **trailing slot** of the `NavigationRail` (not as a regular `NavigationRailDestination`). Tapping it SHALL navigate to the `/contexts` route regardless of the current active context state. On mobile the affordance SHALL appear in the "More" overflow sheet.

#### Scenario: Contexts entry in nav rail trailing slot

- **WHEN** the user is on any main screen (desktop/tablet layout)
- **THEN** the trailing area of the navigation rail displays a "Contexts" icon button with an appropriate icon

#### Scenario: Navigating to Contexts from nav rail trailing slot

- **WHEN** the user taps the trailing "Contexts" icon button
- **THEN** the app navigates to the Context Switcher screen
