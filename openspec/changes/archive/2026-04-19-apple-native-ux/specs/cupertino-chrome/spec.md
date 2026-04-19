## Cupertino Page Chrome

### Summary

Page-level structural widgets (scaffold, navigation bar) use Cupertino equivalents on Apple touch platforms, giving each screen a native iOS look.

### Requirements

- **REQ-1**: Provide an `AdaptiveScaffold` widget that renders `CupertinoPageScaffold` on Apple touch and `Scaffold` on Material/macOS.
- **REQ-2**: Provide an `AdaptiveNavBar` widget that renders `CupertinoNavigationBar` on Apple touch and `AppBar` on Material/macOS. Must support: title, leading widget, trailing actions.
- **REQ-3**: List-style screens (Settings, Skills, Personas, Traces, Logs, Webhooks, Agents, Analytics, Workflows, Contexts) use `CupertinoSliverNavigationBar` with `largeTitle` on Apple touch platforms. The large title collapses on scroll.
- **REQ-4**: The Chat screen uses a regular (non-large-title) `CupertinoNavigationBar` on Apple touch platforms, since it is a conversation view, not a list.
- **REQ-5**: `CupertinoNavigationBar` should use translucent background (the default blur effect).
- **REQ-6**: On Material platforms, all screens continue to use `Scaffold` + `AppBar` unchanged.
- **REQ-7**: Back navigation buttons are handled automatically by `CupertinoNavigationBar` (no manual back button needed on Apple touch).
- **REQ-8**: Screens that use `CupertinoSliverNavigationBar` must convert their body from `ListView` to `CustomScrollView` with `SliverList` (or equivalent sliver widgets).

### Acceptance Criteria

- List screens on iPhone Simulator show large title that collapses on scroll.
- Chat screen on iPhone Simulator shows compact (non-large-title) nav bar with blur.
- Back button appears automatically when navigating to detail screens.
- Web browser shows existing AppBar / Scaffold unchanged.
- No visual regression on macOS native build.
