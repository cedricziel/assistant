import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'nav_shell.dart';

/// Left edge zone (in logical pixels) where a touch-drag is recognised as
/// a sidebar toggle gesture.
const double kEdgeSwipeZoneWidth = 20.0;

/// Minimum horizontal travel (in logical pixels) before a touch-drag triggers
/// a sidebar toggle.
const double kEdgeSwipeThreshold = 40.0;

/// Wraps a child and intercepts horizontal touch-drags starting in the
/// left [kEdgeSwipeZoneWidth] of the wrapped region. A drag exceeding
/// [kEdgeSwipeThreshold] toggles [sidebarCollapsedProvider]. Mouse drags
/// and drags starting outside the edge zone are ignored.
class EdgeSwipeSidebarToggle extends ConsumerStatefulWidget {
  const EdgeSwipeSidebarToggle({super.key, required this.child});

  final Widget child;

  @override
  ConsumerState<EdgeSwipeSidebarToggle> createState() =>
      _EdgeSwipeSidebarToggleState();
}

class _EdgeSwipeSidebarToggleState
    extends ConsumerState<EdgeSwipeSidebarToggle> {
  Offset? _downPosition;
  PointerDeviceKind? _downKind;
  bool _toggled = false;

  void _onPointerDown(PointerDownEvent event) {
    _downPosition = event.localPosition;
    _downKind = event.kind;
    _toggled = false;
  }

  void _onPointerMove(PointerMoveEvent event) {
    if (_toggled) return;
    final start = _downPosition;
    final kind = _downKind;
    if (start == null || kind == null) return;
    if (kind != PointerDeviceKind.touch) return;
    if (start.dx > kEdgeSwipeZoneWidth) return;

    final dx = event.localPosition.dx - start.dx;
    if (dx.abs() < kEdgeSwipeThreshold) return;

    _toggled = true;
    final current = ref.read(sidebarCollapsedProvider).value ?? false;
    final wantCollapsed = dx < 0;
    if (current != wantCollapsed) {
      ref.read(sidebarCollapsedProvider.notifier).toggle();
    }
  }

  void _onPointerUp(PointerUpEvent event) {
    _downPosition = null;
    _downKind = null;
    _toggled = false;
  }

  @override
  Widget build(BuildContext context) {
    return Listener(
      behavior: HitTestBehavior.translucent,
      onPointerDown: _onPointerDown,
      onPointerMove: _onPointerMove,
      onPointerUp: _onPointerUp,
      child: widget.child,
    );
  }
}
