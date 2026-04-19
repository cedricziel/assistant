## Platform Abstraction

### Summary

A single `platform.dart` file that provides the canonical platform detection for the entire app. All UI branching decisions flow from this file — no `defaultTargetPlatform` checks scattered across feature code.

### Requirements

- **REQ-1**: Define `AppPlatformStyle` enum with three values: `cupertino`, `material`, `macos`.
- **REQ-2**: Provide a top-level `platformStyle` getter that maps runtime platform to the enum:
  - `kIsWeb` → `material`
  - `TargetPlatform.macOS` → `macos`
  - `TargetPlatform.iOS` → `cupertino` (covers iPhone, iPad, Mac Catalyst)
  - All other → `material`
- **REQ-3**: Provide convenience getters: `isAppleTouch`, `isMacOS`, `isMaterial`.
- **REQ-4**: File lives at `app/lib/shared/platform/platform.dart`.
- **REQ-5**: No other file in the codebase should directly check `defaultTargetPlatform` for UI rendering decisions after this is introduced — all should use the platform abstraction. Existing platform checks for feature gating (embedded server, tray) may remain as-is since they gate functionality, not rendering.

### Acceptance Criteria

- `platformStyle` returns `cupertino` on iOS Simulator.
- `platformStyle` returns `material` in Chrome (web).
- `platformStyle` returns `macos` on macOS native build.
- Convenience getters are consistent with enum value.
