## ADDED Requirements

### Requirement: Markdown anchor taps open the target URL

Both the streaming and finalised chat message bubbles SHALL pass an `onTapLink` callback to every `SmoothMarkdown` (and `StreamMarkdown`) instance they render. The callback SHALL open the target URL using `url_launcher` with `LaunchMode.externalApplication` so links escape the chat surface on every platform (new tab on web, system browser on native).

#### Scenario: Tapping a markdown link opens it externally

- **GIVEN** an assistant message containing `[Example](https://example.com)`
- **WHEN** the user taps the rendered anchor
- **THEN** `launchUrl(Uri.parse('https://example.com'), mode: LaunchMode.externalApplication)` SHALL be invoked exactly once

#### Scenario: Streaming bubble also handles taps

- **GIVEN** an in-progress assistant message that contains a markdown link
- **WHEN** the user taps the link before the stream completes
- **THEN** the same `MarkdownLinkHandler.onTap` SHALL run AND the launch result SHALL match the finalised-bubble behaviour

### Requirement: Only safe URL schemes are launched

The link handler SHALL only launch URLs whose scheme is `http`, `https`, or `mailto`. Any other scheme (including `javascript`, `data`, `file`, and unknown custom schemes) SHALL be rejected without calling the launcher AND a snackbar with text matching `"Cannot open link"` SHALL be shown.

#### Scenario: javascript: is rejected

- **WHEN** the handler receives `"javascript:alert(1)"`
- **THEN** `launchUrl` SHALL NOT be called AND a snackbar `"Cannot open link: javascript:alert(1)"` SHALL be displayed

#### Scenario: mailto is allowed

- **WHEN** the handler receives `"mailto:hi@example.com"`
- **THEN** `launchUrl` SHALL be called once with that URI AND no snackbar SHALL be shown

### Requirement: Launch failure surfaces to the user

When `launchUrl` returns `false` (or throws), the handler SHALL show a snackbar with text matching `"Could not open link"` so the user understands the click was not silently dropped.

#### Scenario: Launcher returns false

- **GIVEN** the injected launcher returns `false` for an `https://` URL
- **WHEN** `onTap` runs
- **THEN** a snackbar `"Could not open link: https://example.com"` SHALL be shown

#### Scenario: Launcher throws

- **GIVEN** the injected launcher throws a `PlatformException`
- **WHEN** `onTap` runs
- **THEN** the exception SHALL be caught AND a snackbar `"Could not open link"` SHALL be shown AND the error SHALL be logged via `debugPrint`

### Requirement: Scheme/launch logic is unit-testable without plugins

The `MarkdownLinkHandler` SHALL accept an injectable launcher function so the scheme allow-list and launch invocation can be verified on the Dart VM without binding `url_launcher` platform channels. The snackbar feedback requirements above are a separate concern, exercised via `WidgetTester` against a wrapping `MaterialApp + Scaffold` rather than via VM unit tests.

#### Scenario: Test injects a fake launcher

- **GIVEN** a `MarkdownLinkHandler` constructed with a fake launcher recording its arguments
- **WHEN** `onTap("https://example.com")` is invoked
- **THEN** the fake SHALL record one call to `Uri.parse('https://example.com')` with `LaunchMode.externalApplication`

#### Scenario: Handler is robust to a missing ScaffoldMessenger

- **GIVEN** the handler is invoked inside a tree without a `ScaffoldMessenger` ancestor (e.g. a bare-VM test)
- **WHEN** a snackbar would otherwise be shown
- **THEN** the handler SHALL skip the snackbar without throwing AND the launch/decision behaviour SHALL be unchanged
