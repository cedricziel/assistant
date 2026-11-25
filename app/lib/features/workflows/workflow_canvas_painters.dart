import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';

import 'workflow_editor_model.dart';

// ---------------------------------------------------------------------------
// Edge painter

class EdgePainter extends CustomPainter {
  EdgePainter({
    required this.nodes,
    required this.edges,
    required this.nodeSize,
    required this.labelBgColor,
    this.pendingFrom,
    this.pendingEnd,
  });

  final List<EditorNode> nodes;
  final List<EditorEdge> edges;
  final Size nodeSize;
  final Color labelBgColor;
  final String? pendingFrom;
  final Offset? pendingEnd;

  Offset _nodeBottomCenter(EditorNode n) {
    return Offset(
      n.position.dx + nodeSize.width / 2,
      n.position.dy + nodeSize.height,
    );
  }

  Offset _nodeTopCenter(EditorNode n) {
    return Offset(n.position.dx + nodeSize.width / 2, n.position.dy);
  }

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..strokeWidth = 2
      ..style = PaintingStyle.stroke;

    final arrowPaint = Paint()..style = PaintingStyle.fill;

    final labelBg = Paint()
      ..color = labelBgColor
      ..style = PaintingStyle.fill;

    for (final edge in edges) {
      final fromNode = nodes.firstWhere(
        (n) => n.id == edge.from,
        orElse: () => nodes.first,
      );
      final toNode = nodes.firstWhere(
        (n) => n.id == edge.to,
        orElse: () => nodes.last,
      );

      final start = _nodeBottomCenter(fromNode);
      final end = _nodeTopCenter(toNode);
      final color = fromNode.color;

      paint.color = color.withValues(alpha: 0.7);
      arrowPaint.color = color.withValues(alpha: 0.7);

      // Draw curved bezier edge
      final cp1 = Offset(start.dx, start.dy + 40);
      final cp2 = Offset(end.dx, end.dy - 40);
      final path = Path()
        ..moveTo(start.dx, start.dy)
        ..cubicTo(cp1.dx, cp1.dy, cp2.dx, cp2.dy, end.dx, end.dy);
      canvas.drawPath(path, paint);

      // Arrowhead at destination
      _drawArrow(canvas, arrowPaint, end, const Offset(0, -1));

      // Edge label
      if (edge.on != null) {
        final mid = _cubicPoint(start, cp1, cp2, end, 0.5);
        final labelRect = Rect.fromCenter(center: mid, width: 56, height: 18);
        canvas.drawRRect(
          RRect.fromRectAndRadius(labelRect, const Radius.circular(9)),
          labelBg,
        );
        final tp = TextPainter(
          text: TextSpan(
            text: edge.on,
            style: TextStyle(
              fontSize: 10,
              fontWeight: FontWeight.w600,
              color: color,
            ),
          ),
          textDirection: TextDirection.ltr,
        )..layout(maxWidth: 52);
        tp.paint(canvas, Offset(mid.dx - tp.width / 2, mid.dy - tp.height / 2));
      }
    }

    // Pending edge (drag-in-progress)
    if (pendingFrom != null && pendingEnd != null) {
      final fromNode = nodes.firstWhere(
        (n) => n.id == pendingFrom,
        orElse: () => nodes.first,
      );
      final start = _nodeBottomCenter(fromNode);
      paint.color = Colors.grey.withValues(alpha: 0.5);
      paint.strokeWidth = 1.5;
      canvas.drawLine(start, pendingEnd!, paint);
    }
  }

  Offset _cubicPoint(Offset p0, Offset p1, Offset p2, Offset p3, double t) {
    final mt = 1 - t;
    return Offset(
      mt * mt * mt * p0.dx +
          3 * mt * mt * t * p1.dx +
          3 * mt * t * t * p2.dx +
          t * t * t * p3.dx,
      mt * mt * mt * p0.dy +
          3 * mt * mt * t * p1.dy +
          3 * mt * t * t * p2.dy +
          t * t * t * p3.dy,
    );
  }

  void _drawArrow(Canvas canvas, Paint paint, Offset tip, Offset direction) {
    final angle = math.atan2(direction.dy, direction.dx);
    const arrowSize = 8.0;
    final p1 = Offset(
      tip.dx - arrowSize * math.cos(angle - 0.4),
      tip.dy - arrowSize * math.sin(angle - 0.4),
    );
    final p2 = Offset(
      tip.dx - arrowSize * math.cos(angle + 0.4),
      tip.dy - arrowSize * math.sin(angle + 0.4),
    );
    final path = Path()
      ..moveTo(tip.dx, tip.dy)
      ..lineTo(p1.dx, p1.dy)
      ..lineTo(p2.dx, p2.dy)
      ..close();
    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(EdgePainter old) => true;
}

// ---------------------------------------------------------------------------
// Grid background painter

class GridPainter extends CustomPainter {
  GridPainter({required this.gridColor});

  final Color gridColor;

  @override
  void paint(Canvas canvas, Size size) {
    const spacing = 30.0;
    final paint = Paint()
      ..color = gridColor
      ..strokeWidth = 1;

    for (double x = 0; x <= size.width; x += spacing) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), paint);
    }
    for (double y = 0; y <= size.height; y += spacing) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), paint);
    }
  }

  @override
  bool shouldRepaint(GridPainter old) => gridColor != old.gridColor;
}
