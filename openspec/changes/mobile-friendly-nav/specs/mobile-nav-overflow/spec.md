## ADDED Requirements

### Requirement: Mobile bottom bar shows at most 5 destinations

On screens narrower than 768px the bottom `NavigationBar` SHALL display exactly 4 primary destinations (Chat, Personas, Skills, Workflows) plus one "More" overflow destination. It SHALL NOT display more than 5 items.

#### Scenario: Primary destinations visible on mobile

- **WHEN** the app is rendered at a viewport width less than 768px
- **THEN** the bottom navigation bar shows Chat, Personas, Skills, Workflows, and More — and no other destinations

#### Scenario: Overflow destinations not directly in bottom bar

- **WHEN** the app is rendered at a viewport width less than 768px
- **THEN** Traces, Logs, Webhooks, Agents, and Analytics are NOT visible as direct bottom bar items

### Requirement: More destination opens overflow bottom sheet

Tapping the "More" destination SHALL open a modal bottom sheet listing all overflow destinations.

#### Scenario: Tapping More opens sheet

- **WHEN** the user taps the "More" item in the bottom navigation bar
- **THEN** a modal bottom sheet appears containing tappable items for Traces, Logs, Webhooks, Agents, and Analytics

#### Scenario: Tapping overflow item navigates and closes sheet

- **WHEN** the user taps an overflow destination in the bottom sheet
- **THEN** the app navigates to that destination and the bottom sheet is dismissed

### Requirement: More destination shows active state when an overflow route is current

When the current route belongs to an overflow destination, the "More" item SHALL appear selected in the bottom navigation bar.

#### Scenario: Active overflow route highlights More

- **WHEN** the current route is /traces, /logs, /webhooks, /agents, or /analytics
- **THEN** the "More" bottom bar item is rendered in the selected/active visual state
