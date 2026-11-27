## MODIFIED Requirements

<!-- Copied verbatim from openspec/changes/chat-element-library/specs/chat-element-library/spec.md; only the highlight allowlist, the Clipboard/SemanticsService rule and ChatMarkdownScope.onCopyCode differ. Re-diff against that file (or the archived openspec/specs/chat-element-library/spec.md) before archiving this change. -->

### Requirement: Element package is pure (no state management, no I/O, no platform branching)

The `assistant_ui` package at `app/packages/assistant_ui/` SHALL declare only `flutter`, `flutter_smooth_markdown`, `material_ui` and `highlight` under `dependencies`. `highlight` is pinned to the version already resolved transitively through `flutter_smooth_markdown` (0.7.0); it is pure Dart and only tokenises source text for `CodeBlock`. Package source SHALL NOT import `flutter_riverpod`, `assistant_api`, `cupertino_ui`, `package:flutter/cupertino.dart`, `dart:io`, or any I/O or platform-channel package (`record`, `file_picker`, `desktop_drop`, `url_launcher`, `audioplayers`, `http`, `cached_network_image`, `flutter_svg`, `share_plus`, `path_provider`). Package source SHALL NOT reference `Clipboard` or `SemanticsService`. Exported widgets SHALL receive data through constructor parameters or a package-defined inherited scope, and SHALL emit user intent through callbacks.

Package source SHALL NOT branch on `defaultTargetPlatform` or `Theme.of(context).platform` itself. The `.adaptive` constructors of `material_ui` widgets (for example `CircularProgressIndicator.adaptive`, used today at `streaming_timeline_entry.dart:221,304,420` and `turn_progress_card.dart:101`) are permitted. They branch inside the allowlisted `material_ui` dependency, and keeping them preserves today's Cupertino spinner on Apple platforms.

`flutter_smooth_markdown` 0.8.1 depends transitively on `cached_network_image` and `flutter_svg`, and its default `ImageBuilder` fetches `http(s)` images. The package SHALL NOT reach that path on its own: every `SmoothMarkdown` and `StreamMarkdown` it builds SHALL pass an `imageBuilder`, taken from `ChatMarkdownScope` when one is supplied and otherwise a package placeholder that does no I/O.

To add a dependency, a change SHALL carry a MODIFIED delta of this requirement naming the new allowlist and showing that the dependency only renders. Otherwise its adapter SHALL stay in `app/lib`.

The deprecated `package:flutter/material.dart` import that `flutter_smooth_markdown` needs (`MarkdownStyleSheet.fromTheme`) SHALL be confined to the single file `lib/src/theme/legacy_theme.dart`. Only files under `lib/src/highlight/` SHALL import `package:highlight/`.

#### Scenario: Manifest matches the closed allowlist

- **WHEN** `app/packages/assistant_ui/pubspec.yaml` is inspected
- **THEN** its `dependencies` SHALL contain exactly `flutter`, `flutter_smooth_markdown`, `material_ui` and `highlight`

#### Scenario: Source imports no forbidden library

- **WHEN** every Dart file under `app/packages/assistant_ui/lib/` is scanned for `import` and `export` directives and for `Clipboard.` and `SemanticsService.` references
- **THEN** none SHALL reference a forbidden library or API listed in this requirement
- **AND** `package:flutter/material.dart` SHALL appear only in `lib/src/theme/legacy_theme.dart`
- **AND** `package:highlight/` SHALL appear only under `lib/src/highlight/`

#### Scenario: Platform branching stays inside material_ui

- **WHEN** package source is scanned
- **THEN** it SHALL contain no `defaultTargetPlatform`, `.platform ==` or `TargetPlatform.` reference
- **AND** `.adaptive` constructors of `material_ui` widgets SHALL be allowed

#### Scenario: Markdown image without a scope fetches nothing

- **GIVEN** a `MessageBody` with content `![logo](https://example.com/logo.png)` and no `ChatMarkdownScope` ancestor
- **WHEN** it renders
- **THEN** a placeholder showing the alt text `logo` SHALL render
- **AND** no `CachedNetworkImage`, `Image.network` or `SvgPicture.network` widget SHALL be in the tree

#### Scenario: Elements render without a ProviderScope

- **GIVEN** a widget test that pumps any element in `kAssistantUiElements` with literal arguments
- **WHEN** no `ProviderScope` ancestor exists
- **THEN** the element SHALL build without error

#### Scenario: Turn progress card takes its status by parameter

- **GIVEN** `TurnProgressCard` exported from `assistant_ui`
- **WHEN** it is pumped with `status: null`
- **THEN** it SHALL render nothing
- **WHEN** it is pumped with a status older than `kTurnStallThreshold` relative to an injected `now`, and `showSkip: true`
- **THEN** activating the skip affordance SHALL invoke `onSkip` and SHALL NOT call any API

### Requirement: Ambient configuration is supplied by inherited scopes

`ThreadDensityScope` (a `TimelineDensity`), `ChatMarkdownScope` (markdown parser plugins, builder registry, `imageBuilder`, style overrides, an `onTapLink` callback and an `onCopyCode` callback) and `ToolResultRendererScope` (a single `ToolResultBuilder` resolver function) SHALL be the only inherited scopes in the package. Each SHALL have a default when no ancestor exists: `TimelineDensity.normal`; no plugins, the no-I/O image placeholder, links ignored and no code-copy callback (so `CodeBlock` hides its copy button); and no resolver, respectively.

#### Scenario: Density resolves from the scope

- **GIVEN** a `ThreadDensityScope` providing `compact`
- **WHEN** a density-dependent descendant builds
- **THEN** it SHALL render its compact presentation
- **WHEN** the scope is rebuilt providing `expanded`
- **THEN** that descendant SHALL rebuild with its expanded presentation

#### Scenario: Markdown plugins come from the app

- **GIVEN** a `ChatMarkdownScope` whose builder registry maps the `mermaid` block to a test builder
- **WHEN** a `MessageBody` renders content containing a mermaid block
- **THEN** the test builder SHALL render that block

#### Scenario: Image rendering comes from the app

- **GIVEN** a `ChatMarkdownScope` with an `imageBuilder` returning a test widget
- **WHEN** a `MessageBody` renders a markdown image
- **THEN** the test widget SHALL render in its place

#### Scenario: Link taps are delegated

- **GIVEN** a `ChatMarkdownScope` with an `onTapLink` callback
- **WHEN** a link in a `MessageBody`, or in an active streaming `ThinkingBody`, is tapped
- **THEN** the callback SHALL receive the URL AND the package SHALL NOT launch it

#### Scenario: Code copy is delegated

- **GIVEN** a `ChatMarkdownScope` with an `onCopyCode` callback
- **WHEN** the copy button of a `CodeBlock` inside a `MessageBody`, a `ThinkingBody` or a tool result renderer is activated
- **THEN** the callback SHALL receive the code AND the package SHALL NOT write the clipboard
