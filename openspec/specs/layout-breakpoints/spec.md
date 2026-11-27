# layout-breakpoints Specification

## Purpose

The shared 768 px breakpoint token and the rule that a region splits only when its own width is wide.

## Requirements

### Requirement: Layout breakpoints are shared tokens

`app/lib/shared/layout/breakpoints.dart` SHALL declare `Breakpoints.wide = 768`, which is the single width at which a region switches from a single-pane to a split layout. `NavShell` and `ChatScreen` SHALL read this token.

No file under `app/lib` SHALL compare a width against a numeric literal or a private `k…` constant to choose a layout, and none SHALL declare a private `…Breakpoint` constant. The only exceptions are `shared/layout/breakpoints.dart` itself and `features/workflows/workflow_editor_screen.dart`, whose 600 px mobile editor layout is a separate surface outside this change. `TimelineDensity` thresholds SHALL be the named package constants `kTimelineCompactMaxWidth` (400) and `kTimelineExpandedMinWidth` (700) in `assistant_ui`, which describe content density, not layout.

#### Scenario: No literal layout widths remain

- **WHEN** a unit test scans every Dart file under `app/lib` for the patterns `\b(width|maxWidth)\s*[<>]=?\s*([0-9]|_?k[A-Z]\w*)` and `const double _?k\w*Breakpoint`
- **THEN** it SHALL find matches only in the files on its file-scoped allowlist, which SHALL be exactly `shared/layout/breakpoints.dart` and `features/workflows/workflow_editor_screen.dart`

#### Scenario: Lint catches the forms in use today

- **GIVEN** fixture lines `size.width > 700`, `MediaQuery.sizeOf(context).width >= _kNavRailBreakpoint`, `constraints.maxWidth < 600` and `const double _kNavRailBreakpoint = 768;`
- **WHEN** the lint's matcher runs on them
- **THEN** each SHALL match

#### Scenario: Nav shell uses the token

- **WHEN** `NavShell` renders at a window width of `Breakpoints.wide - 1` and at `Breakpoints.wide`
- **THEN** it SHALL show the bottom navigation and the rail or sidebar respectively

### Requirement: A region splits only when the region itself is wide

`ChatScreen` SHALL decide between the inline conversation list and the drawer from the width available to the chat pane, measured by a `LayoutBuilder` above its `Scaffold` (the decision drives the body, the app bar's leading menu button and the drawer), not from the window width. The list SHALL be inline when the pane is at least `Breakpoints.wide` wide. Otherwise the drawer and its menu button SHALL be used. The thread's `ThreadDensityScope` SHALL be fed from the width available to the thread, after the inline list is subtracted.

#### Scenario: No bottom-nav and inline-list overlap

- **WHEN** the app renders the chat at a window width of 740 px
- **THEN** the conversation list SHALL be in the drawer, not inline

#### Scenario: Expanded nav sidebar narrows the pane

- **GIVEN** a 900 px window with the expanded 240 px nav sidebar and its 1 px divider, so the chat pane is 659 px
- **WHEN** the chat renders
- **THEN** the conversation list SHALL be in the drawer AND the menu button labelled "Open conversations" SHALL be shown

#### Scenario: Collapsed nav sidebar narrows the pane less

- **GIVEN** an 840 px window with the collapsed 72 px nav sidebar, so the chat pane is 767 px
- **THEN** the conversation list SHALL be in the drawer
- **GIVEN** an 841 px window with the collapsed sidebar, so the pane is 768 px
- **THEN** the conversation list SHALL be inline

#### Scenario: Wide window shows the list inline

- **GIVEN** a 1280 px window with the expanded nav sidebar
- **WHEN** the chat renders
- **THEN** the 240 px conversation list SHALL be inline AND no drawer SHALL be attached

#### Scenario: Toggling the sidebar flips the layout once

- **GIVEN** a 900 px window with the collapsed sidebar, so the list is inline
- **WHEN** the user expands the sidebar and its 200 ms width animation runs to completion frame by frame
- **THEN** the chat SHALL switch from inline list to drawer exactly once AND no exception or overflow SHALL be reported

#### Scenario: Density follows thread width

- **GIVEN** a 1280 px window with the expanded nav sidebar and the inline list, so the thread is about 798 px wide
- **WHEN** a timeline entry renders
- **THEN** it SHALL resolve `TimelineDensity.expanded`
- **GIVEN** a 1000 px window with the same chrome, so the pane is 759 px and the list moves to the drawer
- **THEN** density SHALL be computed from 759 px

### Requirement: Breakpoint documentation matches the code

The `ux-principles` skill's responsive table SHALL list `Breakpoints.wide` (768) and the pane-width rule. It SHALL NOT list widths that no code uses.

#### Scenario: Skill and code agree

- **WHEN** `.claude/skills/ux-principles/SKILL.md` is read after this change
- **THEN** its breakpoint table SHALL name 768 and `Breakpoints.wide` AND SHALL NOT name 640 or 900
