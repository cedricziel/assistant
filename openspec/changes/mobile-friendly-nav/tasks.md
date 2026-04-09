## 1. Refactor Destination Data Model in nav_shell.dart

- [x] 1.1 Split the single `_destinations` list into `_primaryDestinations` (Chat, Contexts, Skills, Workflows) and `_overflowDestinations` (Traces, Logs, Webhooks, Agents, Analytics) constants
- [x] 1.2 Create a helper method `_routeForIndex(int index)` that maps a combined index across primary + overflow lists to the correct route path
- [x] 1.3 Create a helper method `_isOverflowRouteActive(String currentPath)` that returns true when the current URI matches any overflow destination's route

## 2. Mobile Bottom Bar Overflow ("More")

- [x] 2.1 Add a "More" `NavigationDestination` as the 5th item in the mobile `NavigationBar`, using `more_horiz` icon and label "More"
- [x] 2.2 Implement `_showMoreSheet(BuildContext context)` that calls `showModalBottomSheet` displaying a grid/list of overflow destinations with their icons and labels
- [x] 2.3 Wire each overflow item in the sheet to navigate to its route and pop the sheet on tap
- [x] 2.4 Apply active/selected visual state to the "More" destination when `_isOverflowRouteActive` returns true for the current route
- [x] 2.5 Ensure the bottom sheet is keyboard-accessible and screen-reader labelled

## 3. Desktop Nav Rail Grouping

- [x] 3.1 Render primary destinations in the rail followed by a non-interactive divider row (disabled `NavigationRailDestination` that renders a `Divider`)
- [x] 3.2 Render overflow/developer destinations below the divider in the rail
- [x] 3.3 Verify selected index logic still maps correctly across the combined primary + divider + overflow list (divider must not consume an index slot)

## 4. Index-to-Route Mapping Correctness

- [x] 4.1 Audit `_onDestinationSelected` in `NavShell` to ensure tapping any primary destination navigates correctly after the list restructure
- [x] 4.2 Audit active-destination detection (URI comparison) for all 9 routes after the restructure
- [x] 4.3 Verify the PWA install button positioning in the rail trailing area is unaffected

## 5. Widget Tests

- [x] 5.1 Write a widget test that asserts the mobile `NavigationBar` contains exactly 5 items at width < 768px
- [x] 5.2 Write a widget test that asserts tapping "More" opens the bottom sheet containing all overflow destinations
- [x] 5.3 Write a widget test that asserts the "More" item shows as selected when the current route is `/traces`
- [x] 5.4 Write a widget test that asserts all 9 destinations are visible in the `NavigationRail` at width ≥ 768px with a divider between groups

## 6. Manual QA

- [ ] 6.1 Test on Chrome at 375px width (iPhone SE): confirm 5 bottom bar items, overflow sheet opens, all routes reachable
- [ ] 6.2 Test on Chrome at 1280px width: confirm rail shows all 9 items with visible divider, no regression
- [ ] 6.3 Test active state: navigate to /logs and confirm "More" appears selected on mobile; "Logs" appears selected on desktop
