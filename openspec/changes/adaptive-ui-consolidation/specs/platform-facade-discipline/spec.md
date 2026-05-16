## ADDED Requirements

### Requirement: Façade is the sole entry point for adaptive rendering

All platform-conditional rendering in the Flutter app SHALL flow through `app/lib/shared/platform/`. Feature code (anything under `app/lib/features/**` or `app/lib/shared/**` other than `lib/shared/platform/`) SHALL NOT import `package:flutter/cupertino.dart`, and SHALL NOT branch on `defaultTargetPlatform`, `Theme.of(context).platform`, or `Platform.is*` for rendering purposes. Non-rendering platform checks in service-layer code (subprocess spawning, file-system APIs, etc.) remain permitted.

#### Scenario: New widget needs iOS-specific rendering

- **WHEN** a developer needs a widget that looks different on iOS vs Material
- **THEN** they SHALL add or extend a wrapper under `app/lib/shared/platform/`
- **THEN** the feature code SHALL consume that wrapper without conditional imports

#### Scenario: Service-layer platform check is permitted

- **WHEN** a non-widget file (e.g. `installer_launcher.dart`) checks `Platform.isMacOS` to choose a subprocess
- **THEN** this SHALL be permitted and SHALL NOT trigger a lint failure

### Requirement: Lint enforcement via custom_lint

The repository SHALL include a `custom_lint` rule that fires on any `import 'package:flutter/cupertino.dart'` or `import 'package:flutter/material.dart'` statement in a file outside the allowlist. The allowlist SHALL contain `app/lib/shared/platform/**` and `app/lib/main.dart`. The rule SHALL run as part of `make lint-flutter` and the Flutter CI workflow.

#### Scenario: Disallowed Cupertino import is flagged

- **WHEN** a developer commits a file under `app/lib/features/` that imports `package:flutter/cupertino.dart`
- **THEN** `make lint-flutter` SHALL exit non-zero with a diagnostic naming the file, the offending import, and a hint pointing to `app/lib/shared/platform/`
- **THEN** the CI Flutter workflow SHALL block merge of the PR

#### Scenario: Disallowed Material import is flagged

- **WHEN** the same situation occurs with `package:flutter/material.dart`
- **THEN** the same diagnostic and CI behaviour SHALL occur

#### Scenario: Allowlisted file is not flagged

- **WHEN** a file under `app/lib/shared/platform/` or the root `app/lib/main.dart` imports either Cupertino or Material
- **THEN** the lint SHALL accept the import without error

#### Scenario: Defensive sweep at Phase 4 landing

- **WHEN** the lint rule is enabled in the repo (Phase 4)
- **THEN** running `make lint-flutter` against the codebase SHALL pass with zero violations
- **THEN** no file outside the allowlist SHALL import `package:flutter/cupertino.dart` or `package:flutter/material.dart`

### Requirement: adaptive_platform_ui dependency is confined to the façade

The `adaptive_platform_ui` package SHALL be imported only from files under `app/lib/shared/platform/`. No feature screen or shared widget outside the façade SHALL import `package:adaptive_platform_ui/...`.

#### Scenario: Stray package import is rejected

- **WHEN** a developer imports `package:adaptive_platform_ui/...` from a file outside `app/lib/shared/platform/`
- **THEN** `make lint-flutter` SHALL fail with a diagnostic instructing the developer to add the needed widget to the façade

#### Scenario: Package version is pinned exact

- **WHEN** `app/pubspec.yaml` declares `adaptive_platform_ui`
- **THEN** the version constraint SHALL be an exact pin (e.g. `0.1.107`) and SHALL NOT use a caret (`^`) or range operator

### Requirement: AppPlatformStyle three-bucket model is preserved

The `AppPlatformStyle` enum in `app/lib/shared/platform/platform.dart` SHALL keep its three values: `cupertino`, `material`, and `macos`. Wrappers SHALL branch on this enum (or its convenience getters `isAppleTouch`, `isMaterial`, `isMacOS`) rather than on `Platform.is*` or `defaultTargetPlatform` directly. The `macos` bucket SHALL remain a distinct branch in every new wrapper even when its current rendering aliases to the Material output, so a future macOS-native rendering pass is a local change.

#### Scenario: New wrapper branches on AppPlatformStyle

- **WHEN** a developer adds a new wrapper under `app/lib/shared/platform/`
- **THEN** the wrapper SHALL branch on `platformStyle` (or `isAppleTouch`/`isMaterial`/`isMacOS`) and SHALL NOT call `Platform.is*` or `defaultTargetPlatform` directly

#### Scenario: macos bucket remains addressable

- **WHEN** a future change introduces a macOS-native widget for a wrapper
- **THEN** the change SHALL only need to edit the wrapper's `isMacOS` branch
- **THEN** no feature code SHALL need to change
