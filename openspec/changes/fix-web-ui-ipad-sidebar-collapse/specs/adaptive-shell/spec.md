## MODIFIED Requirements

### Requirement: REQ-2 sidebar on Apple touch wide is collapsible with a persistent state

On Apple touch platforms at regular width (>= 768 dp), the shell SHALL render a sidebar with all destinations using `CupertinoSidebarCollapsible`. Collapse / expand SHALL be driven by `sidebarCollapsedProvider`, which SHALL persist its state across app launches as defined in the `sidebar-collapse-state` capability. The Cupertino toggle button at the top-leading corner SHALL be rendered for every Apple touch wide configuration (native iPad app, Mac Catalyst).

#### Scenario: Native iPad-landscape sidebar starts collapsed when previously collapsed

- **GIVEN** `assistant.sidebarCollapsed == true` is persisted
- **WHEN** the native iPad app launches in landscape
- **THEN** `CupertinoSidebarCollapsible.isExpanded` SHALL be `false` on first frame

#### Scenario: Cupertino toggle button uses the persisted notifier

- **WHEN** the user taps the Cupertino top-leading toggle
- **THEN** `sidebarCollapsedProvider.notifier.toggle()` SHALL run AND the new value SHALL be written to `SharedPreferences`
