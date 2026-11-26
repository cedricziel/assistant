# assistant_ui

Pure, state-free chat elements for the assistant app. Every piece a conversation
is made of (message bubble, timeline entry, composer, thread layout) is built
here from constructor data and shown in isolation, in every state. The app owns
state, I/O and navigation and plugs them in.

The catalogue of elements, with a verdict for each, is in
[ELEMENTS.md](ELEMENTS.md). Decisions are recorded in
[ADR-0021](../../../docs/adr/adr-0021-assistant-ui-element-package.md).

## Purity rule

`app/test/unit/assistant_ui_boundary_test.dart` enforces it.

- `dependencies` are exactly `flutter`, `flutter_smooth_markdown` and
  `material_ui`.
- Package source never imports `flutter_riverpod`, `assistant_api`,
  `cupertino_ui`, `package:flutter/cupertino.dart`, `dart:io`, or an I/O or
  platform-channel package (`record`, `file_picker`, `desktop_drop`,
  `url_launcher`, `audioplayers`, `http`, `cached_network_image`, `flutter_svg`,
  `share_plus`, `path_provider`).
- `package:flutter/material.dart` appears only in `lib/src/theme/legacy_theme.dart`,
  for `MarkdownStyleSheet.fromTheme`.
- Source never reads `defaultTargetPlatform`, `.platform ==` or
  `TargetPlatform.`. `material_ui`'s `.adaptive` constructors are allowed: they
  branch inside `material_ui`, and replacing them would change Apple rendering.
- `flutter_smooth_markdown` fetches `http(s)` images by default. Every markdown
  widget the package builds passes an `imageBuilder`: the one from
  `ChatMarkdownScope` if the app supplies it, otherwise a placeholder showing the
  alt text.
- Colours and spacing are tokens in `lib/src/theme/`, equal to what the code
  rendered before the move. No raw `Color(0x...)` or `Colors.X` elsewhere
  (`app/scripts/check_no_raw_colors.sh`; `Colors.transparent` is allowed).

## Injection rule

When an element needs something adaptive or I/O-backed, it exposes a slot or
builder and ships a neutral Material default, so the gallery and goldens run
standalone. The app passes the real thing, for example `AdaptiveTextField` as
the composer's `inputBuilder`.

## Composition rule

Constructor slots (`header`, `body`, `footer`, ...) are the default. An inherited
scope is allowed only for configuration that is ambient and read at more than one
nesting level. There are three: `ThreadDensityScope`, `ChatMarkdownScope` and
`ToolResultRendererScope`. Riverpod inside an element is banned.

## Amendment rule

A change that needs another package dependency must show that it only renders
(no I/O, no platform channel, no state management), add a MODIFIED delta of the
"Element package is pure" requirement listing the new allowlist, and update the
boundary test in the same PR. Otherwise the adapter stays in `app/lib`.

## Checklist for every element PR

- Add the element to `kAssistantUiElements`.
- Add a widgetbook use-case.
- Add at least one golden test tagged `golden`, with a fixed clock where time is
  shown. Advance with `pump(Duration)`; `pumpAndSettle` is banned in goldens.
- For an extracted widget: render the characterization inputs through the new
  element and match those goldens byte for byte, then point the characterization
  test at the element, then delete the app widget.
- Update the element's row in `ELEMENTS.md`.
- Move the widget's existing tests alongside it and keep every widget `Key`.
- Run `flutter analyze --fatal-infos` and `flutter test` in `app/` and here.
- Run the Playwright visual suite against the current baselines, with no
  baseline update.

## Gallery

`app/widgetbook/` is a Widgetbook gallery of every element in
`kAssistantUiElements` (`lib/src/registry.dart`). Run it with
`cd app/widgetbook && flutter run -d chrome`. Add a `WidgetbookComponent` named
after the element to `kComponents` in `app/widgetbook/lib/components.dart`; use
`enumKnob` from `lib/knobs.dart` for state and density knobs. The gallery is its
own entry point, so `flutter build web` in `app/` never compiles it.
`app/widgetbook/test/coverage_test.dart` fails when a registered element lacks a
component or a `golden`-tagged test.

## Goldens

Baselines are generated on Linux. `make test-flutter` skips the `golden` tag
unless `CI` is set or the host is Linux. `make update-goldens` regenerates the
baselines in the Flutter Docker image pinned to `app/pubspec.yaml`. Use
`pumpGolden` from `test/support/golden.dart`.
