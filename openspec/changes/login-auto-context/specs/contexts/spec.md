## MODIFIED Requirements

### Requirement: Contexts navigation hidden on web platform

On the web platform, the contexts nav destination and switcher screen SHALL NOT be accessible via the navigation rail or any in-app navigation. On native platforms (macOS, iOS, Android) the contexts nav destination SHALL remain visible and functional as before.

#### Scenario: Contexts button hidden on web

- **WHEN** the app runs on the web platform
- **THEN** the nav rail trailing section SHALL NOT display the contexts switcher button (`Icons.swap_horiz_outlined`)

#### Scenario: Contexts button visible on native

- **WHEN** the app runs on macOS or mobile
- **THEN** the nav rail trailing section SHALL display the contexts switcher button as before

#### Scenario: Direct navigation to /contexts blocked on web

- **WHEN** a web user navigates directly to `/contexts`
- **THEN** the router SHALL redirect them to `/login` if no active context exists, or `/chat` if one does
