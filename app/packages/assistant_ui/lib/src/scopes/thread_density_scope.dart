import 'package:material_ui/material_ui.dart';

import '../models/timeline_state.dart';

/// Supplies the [TimelineDensity] of a thread to every element below it.
///
/// Elements read it with [ThreadDensityScope.of] instead of taking a
/// `density` parameter. Without an ancestor scope, [of] returns
/// [TimelineDensity.normal].
class ThreadDensityScope extends InheritedWidget {
  const ThreadDensityScope({
    super.key,
    required this.density,
    required super.child,
  });

  /// Derives the density from a viewport [width] via
  /// [TimelineDensity.fromWidth].
  ThreadDensityScope.fromWidth({
    super.key,
    required double width,
    required super.child,
  }) : density = TimelineDensity.fromWidth(width);

  final TimelineDensity density;

  /// The density of the nearest scope, or [TimelineDensity.normal] when there
  /// is none. Rebuilds the caller when the density changes.
  static TimelineDensity of(BuildContext context) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<ThreadDensityScope>();
    return scope?.density ?? TimelineDensity.normal;
  }

  @override
  bool updateShouldNotify(ThreadDensityScope oldWidget) =>
      density != oldWidget.density;
}
