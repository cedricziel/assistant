import 'package:material_ui/material_ui.dart';

import 'workflow_editor_model.dart';
import 'workflow_canvas_painters.dart';
import 'workflow_node_widget.dart';

// ---------------------------------------------------------------------------
// Desktop canvas editor

class DesktopCanvasEditor extends StatelessWidget {
  const DesktopCanvasEditor({
    super.key,
    required this.canvasController,
    required this.nodes,
    required this.edges,
    required this.connectingFrom,
    required this.pendingEdgeEnd,
    required this.onDragNode,
    required this.onEditNode,
    required this.onDeleteNode,
    required this.onStartConnect,
    required this.onCompleteEdge,
    required this.onCancelConnecting,
    required this.onUpdatePendingEdge,
    required this.onDeleteEdge,
  });

  final TransformationController canvasController;
  final List<EditorNode> nodes;
  final List<EditorEdge> edges;
  final String? connectingFrom;
  final Offset? pendingEdgeEnd;
  final void Function(EditorNode node, Offset delta) onDragNode;
  final void Function(EditorNode node) onEditNode;
  final void Function(String nodeId) onDeleteNode;
  final void Function(String nodeId) onStartConnect;
  final Future<void> Function(String nodeId) onCompleteEdge;
  final VoidCallback onCancelConnecting;
  final void Function(Offset offset) onUpdatePendingEdge;
  final void Function(EditorEdge edge) onDeleteEdge;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isConnecting = connectingFrom != null;

    return Column(
      children: [
        // Connecting mode banner
        if (isConnecting)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            color: theme.colorScheme.primaryContainer,
            child: Row(
              children: [
                Icon(
                  Icons.cable,
                  size: 16,
                  color: theme.colorScheme.onPrimaryContainer,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Tap a destination node to connect — or tap background to cancel',
                    style: TextStyle(
                      fontSize: 12,
                      color: theme.colorScheme.onPrimaryContainer,
                    ),
                  ),
                ),
                TextButton(
                  onPressed: onCancelConnecting,
                  child: const Text('Cancel'),
                ),
              ],
            ),
          ),

        // Canvas
        Expanded(
          child: InteractiveViewer(
            transformationController: canvasController,
            constrained: false,
            boundaryMargin: const EdgeInsets.all(100),
            minScale: 0.4,
            maxScale: 2.5,
            child: SizedBox(
              width: kEditorCanvasW,
              height: kEditorCanvasH,
              child: Stack(
                children: [
                  // Grid background
                  Positioned.fill(
                    child: CustomPaint(
                      painter: GridPainter(
                        gridColor: theme.colorScheme.onSurface.withValues(
                          alpha: 0.04,
                        ),
                      ),
                    ),
                  ),

                  // Edges layer
                  Positioned.fill(
                    child: GestureDetector(
                      onTapUp: (_) {
                        if (isConnecting) onCancelConnecting();
                      },
                      child: CustomPaint(
                        painter: EdgePainter(
                          nodes: nodes,
                          edges: edges,
                          nodeSize: kEditorNodeSize,
                          labelBgColor: theme.colorScheme.surface,
                          pendingFrom: connectingFrom,
                          pendingEnd: pendingEdgeEnd,
                        ),
                      ),
                    ),
                  ),

                  // Edge delete hit areas (midpoints)
                  ...edges.map(
                    (edge) => _EdgeDeleteHandle(
                      edge: edge,
                      nodes: nodes,
                      nodeSize: kEditorNodeSize,
                      onDelete: () => onDeleteEdge(edge),
                    ),
                  ),

                  // Nodes
                  ...nodes.map(
                    (node) => NodeWidget(
                      key: ValueKey(node.id),
                      node: node,
                      isConnectingFrom: connectingFrom == node.id,
                      isConnecting: isConnecting,
                      onDrag: (delta) => onDragNode(node, delta),
                      onEdit: () => onEditNode(node),
                      onDelete: () => onDeleteNode(node.id),
                      onStartConnect: () => onStartConnect(node.id),
                      onTap: isConnecting
                          ? () => onCompleteEdge(node.id)
                          : null,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// Edge delete handle (shown at midpoint of each edge)

class _EdgeDeleteHandle extends StatelessWidget {
  const _EdgeDeleteHandle({
    required this.edge,
    required this.nodes,
    required this.nodeSize,
    required this.onDelete,
  });

  final EditorEdge edge;
  final List<EditorNode> nodes;
  final Size nodeSize;
  final VoidCallback onDelete;

  Offset _midpoint() {
    final fromNode = nodes.firstWhere(
      (n) => n.id == edge.from,
      orElse: () => nodes.first,
    );
    final toNode = nodes.firstWhere(
      (n) => n.id == edge.to,
      orElse: () => nodes.last,
    );
    final start = Offset(
      fromNode.position.dx + nodeSize.width / 2,
      fromNode.position.dy + nodeSize.height,
    );
    final end = Offset(
      toNode.position.dx + nodeSize.width / 2,
      toNode.position.dy,
    );
    return Offset((start.dx + end.dx) / 2, (start.dy + end.dy) / 2);
  }

  @override
  Widget build(BuildContext context) {
    final mid = _midpoint();
    return Positioned(
      left: mid.dx - 18,
      top: mid.dy - 18,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onDelete,
        child: SizedBox(
          width: 36,
          height: 36,
          child: Center(
            child: Container(
              width: 20,
              height: 20,
              decoration: BoxDecoration(
                color: Theme.of(context).colorScheme.error,
                shape: BoxShape.circle,
              ),
              child: Icon(
                Icons.close,
                size: 12,
                color: Theme.of(context).colorScheme.surface,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
