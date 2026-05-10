## ADDED Requirements

### Requirement: Space selector has a discoverable close affordance

`SpaceSelectorScreen` SHALL render a "Close" affordance (icon button or equivalent) that returns the user to `/chat` without modifying `spaceSelectionProvider`. The affordance MUST be visible on every entry to `/spaces`, regardless of how many orgs/spaces the user has access to.

#### Scenario: Close button is visible on every entry

- **WHEN** the user navigates to `/spaces`
- **THEN** a clearly identifiable close affordance SHALL be visible in the screen header

#### Scenario: Close returns to /chat without changing selection

- **GIVEN** `spaceSelectionProvider` has `(orgId: o, spaceId: s)`
- **WHEN** the user taps the close affordance
- **THEN** the router SHALL navigate to `/chat` AND `spaceSelectionProvider` SHALL still be `(orgId: o, spaceId: s)`

#### Scenario: Close from mid-org-change abandons the change

- **GIVEN** the user clicked "Change organization" (clearing the selection) AND is now viewing `_OrgList`
- **WHEN** they tap the close affordance
- **THEN** the router SHALL navigate to `/chat` AND `spaceSelectionProvider` SHALL be reset to its prior `(orgId, spaceId)` if the prior selection still references a valid org/space — otherwise reset to empty

### Requirement: "Change organization" hides for single-org users

`_SpaceList` SHALL only render the "Change organization" button when `orgsProvider` data indicates more than one org is available. Single-org users (`orgs.length == 1`) MUST NOT see the button.

#### Scenario: Single-org — button hidden

- **GIVEN** the API returns exactly one org
- **WHEN** `_SpaceList` renders for that user
- **THEN** the "Change organization" button SHALL NOT appear

#### Scenario: Multi-org — button visible

- **GIVEN** the API returns two or more orgs
- **WHEN** `_SpaceList` renders
- **THEN** the "Change organization" button SHALL appear AND clicking it SHALL clear the current selection and return the user to `_OrgList`

#### Scenario: Org count changes mid-session

- **GIVEN** the user is on `_SpaceList` with one org AND a second org becomes available (e.g., admin invitation accepted, providers refresh)
- **WHEN** the screen rebuilds with the new orgs list
- **THEN** the "Change organization" button SHALL render

### Requirement: SpaceSwitcher in the nav remains fully clickable

The `SpaceSwitcher` widget in the navigation shell SHALL remain a fully-clickable entry point to `/spaces` regardless of the user's space count. This change MUST NOT introduce any conditional rendering, disabling, or hiding of the switcher itself.

#### Scenario: Single-space user can still click the switcher

- **GIVEN** the user has exactly one org and one space
- **WHEN** they tap the SpaceSwitcher
- **THEN** the router SHALL navigate to `/spaces` (and the destination screen handles the dead-end via the new close affordance)

#### Scenario: Multi-space user — unchanged

- **GIVEN** the user has multiple spaces
- **WHEN** they tap the SpaceSwitcher
- **THEN** behavior SHALL be identical to before this change
