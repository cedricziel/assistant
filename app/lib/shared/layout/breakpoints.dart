/// Shared layout breakpoint tokens.
///
/// A region splits into a list and a detail pane only when the region itself
/// is at least [wide] logical pixels wide. `NavShell` compares the window;
/// `ChatScreen` compares its own pane (the window minus the navigation
/// sidebar). See `adaptive-shell` REQ-5.
abstract final class Breakpoints {
  /// The single width at which a region switches from one pane to a split
  /// layout.
  static const double wide = 768;
}
