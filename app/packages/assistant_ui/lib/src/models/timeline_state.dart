/// Display density for timeline entries, derived from viewport width.
enum TimelineDensity {
  /// < 400px: minimal detail, icon + name only for most entries.
  compact,

  /// 400–700px: standard detail level.
  normal,

  /// > 700px: full detail including arguments preview.
  expanded;

  /// Derive density from the current viewport width.
  static TimelineDensity fromWidth(double width) {
    if (width < 400) return TimelineDensity.compact;
    if (width <= 700) return TimelineDensity.normal;
    return TimelineDensity.expanded;
  }
}

/// Lifecycle state of a timeline entry during streaming.
enum EntryState {
  /// Currently being produced (streaming thinking, tool running, etc).
  active,

  /// Finished normally — will auto-collapse after a short delay.
  complete,

  /// A newer entry superseded this one — stays collapsed, reduced opacity.
  stale,

  /// Paused until the user answers (a tool approval or a question). The entry
  /// stays expanded, never auto-collapses and shows an attention accent.
  awaitingInput,
}
