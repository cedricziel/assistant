## ADDED Requirements

### Requirement: iPhone uses bottom navigation bar

On iPhone (screen width < 600 dp) the app SHALL display a `BottomNavigationBar` for primary navigation instead of a `NavigationRail`.

#### Scenario: User runs the app on an iPhone

- **WHEN** the app is running on an iPhone or any device with a screen width below 600 dp
- **THEN** a bottom navigation bar is shown at the bottom of the screen
- **THEN** the navigation rail is NOT shown
- **THEN** all primary destinations (Chat, Personas, Skills, Traces, Logs, Settings) are reachable from the bottom bar

#### Scenario: User taps a destination in the bottom bar

- **WHEN** the user taps a destination icon in the bottom navigation bar
- **THEN** the corresponding screen is shown
- **THEN** the selected icon is highlighted

### Requirement: iPad uses navigation rail or sidebar

On iPadOS (screen width >= 600 dp) the app SHALL display the existing `NavigationRail` for primary navigation, consistent with the macOS layout.

#### Scenario: User runs the app on an iPad

- **WHEN** the app is running on an iPad or any device with a screen width of 600 dp or more
- **THEN** the navigation rail is shown on the leading edge of the screen
- **THEN** no bottom navigation bar is shown

#### Scenario: iPad rotated to portrait

- **WHEN** the user rotates an iPad to portrait orientation and the width drops below 600 dp
- **THEN** the navigation switches to the bottom bar layout
- **THEN** all destinations remain accessible

#### Scenario: iPad rotated to landscape

- **WHEN** the user rotates an iPad to landscape orientation and the width is 600 dp or more
- **THEN** the navigation rail is shown

### Requirement: Navigation shell does not overflow on small screens

The navigation shell SHALL NOT overflow or clip on any supported iOS device, including iPhone SE (375 dp wide, 667 dp tall).

#### Scenario: App displayed on the smallest supported device

- **WHEN** the app is run on a device with a screen of 375 × 667 dp
- **THEN** no `RenderFlex overflowed` error is reported
- **THEN** all navigation destinations are visible and tappable
