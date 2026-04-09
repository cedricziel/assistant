## ADDED Requirements

### Requirement: Desktop nav rail visually groups primary and developer destinations

On screens 768px wide or wider, the `NavigationRail` SHALL render a visual divider between the primary destinations (Chat, Contexts, Skills, Workflows) and the developer/observability destinations (Traces, Logs, Webhooks, Agents, Analytics).

#### Scenario: Divider present between groups on wide screen

- **WHEN** the app is rendered at a viewport width of 768px or greater
- **THEN** a horizontal divider line is visible in the navigation rail separating the top group (Chat, Contexts, Skills, Workflows) from the bottom group (Traces, Logs, Webhooks, Agents, Analytics)

#### Scenario: All 9 destinations remain accessible on desktop

- **WHEN** the app is rendered at a viewport width of 768px or greater
- **THEN** all 9 navigation destinations are visible and tappable in the rail (no overflow or hidden items)

### Requirement: Navigation rail destination order matches grouped structure

The `NavigationRail` SHALL list primary destinations first, then the divider, then developer destinations — in a fixed, predictable order.

#### Scenario: Primary destinations appear above divider

- **WHEN** the navigation rail is visible
- **THEN** Chat, Contexts, Skills, and Workflows appear above the divider in that order

#### Scenario: Developer destinations appear below divider

- **WHEN** the navigation rail is visible
- **THEN** Traces, Logs, Webhooks, Agents, and Analytics appear below the divider
