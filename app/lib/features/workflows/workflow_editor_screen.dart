import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:uuid/uuid.dart';

import '../../shared/platform/adaptive_dialog.dart';
import 'workflows_provider.dart';

// ---------------------------------------------------------------------------
// Canvas layout constants (shared across canvas widgets)

const _kNodeW = 160.0;
const _kNodeH = 72.0;
const _kNodeSize = Size(_kNodeW, _kNodeH);
const _kCanvasW = 3000.0;
const _kCanvasH = 3000.0;

// ---------------------------------------------------------------------------
// Local graph model (not the API model — nodes carry canvas positions)

class EditorNode {
  EditorNode({
    required this.id,
    required this.kind,
    required this.config,
    required this.position,
  });

  final String id;
  final String kind; // trigger | action | condition
  Map<String, dynamic> config;
  Offset position;

  EditorNode copyWith({Map<String, dynamic>? config, Offset? position}) {
    return EditorNode(
      id: id,
      kind: kind,
      config: config ?? Map.from(this.config),
      position: position ?? this.position,
    );
  }

  String get label {
    final type = config['type'] as String? ?? kind;
    switch (type) {
      case 'manual':
        return 'Manual Trigger';
      case 'webhook':
        return 'Webhook Trigger';
      case 'schedule':
        return 'Schedule Trigger';
      case 'event':
        return 'Event Trigger';
      case 'assistant_turn':
        return 'Assistant Turn';
      case 'http_request':
        return 'HTTP Request';
      case 'always_true':
        return 'Always True';
      case 'always_false':
        return 'Always False';
      case 'has_trigger_payload_key':
        return 'Payload Key Check';
      default:
        return type;
    }
  }

  Color get color {
    switch (kind) {
      case 'trigger':
        return Colors.blue;
      case 'action':
        return Colors.green;
      case 'condition':
        return Colors.orange;
      default:
        return Colors.grey;
    }
  }

  IconData get icon {
    switch (kind) {
      case 'trigger':
        return Icons.play_circle_outline;
      case 'action':
        return Icons.bolt;
      case 'condition':
        return Icons.call_split;
      default:
        return Icons.circle_outlined;
    }
  }

  /// Valid outcome labels for edges leaving this node.
  List<String> get outcomesLabels {
    switch (kind) {
      case 'trigger':
        return ['trigger'];
      case 'action':
        return ['success', 'failure'];
      case 'condition':
        return ['true', 'false'];
      default:
        return ['next'];
    }
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'kind': kind,
    'config': config,
    'position': {'x': position.dx, 'y': position.dy},
  };
}

class EditorEdge {
  EditorEdge({required this.from, required this.to, this.on});

  final String from;
  final String to;
  String? on;

  Map<String, dynamic> toJson() => {
    'from': from,
    'to': to,
    if (on != null) 'on': on,
  };
}

// ---------------------------------------------------------------------------
// Mobile edge inference

/// Builds a linear edge chain from the ordered [nodes] list.
/// Each node connects to the next via its primary outcome.
List<EditorEdge> buildMobileEdges(List<EditorNode> nodes) {
  final edges = <EditorEdge>[];
  for (int i = 0; i < nodes.length - 1; i++) {
    final from = nodes[i];
    final to = nodes[i + 1];
    final String outcome;
    switch (from.kind) {
      case 'trigger':
        outcome = 'trigger';
        break;
      case 'condition':
        outcome = 'true';
        break;
      default:
        outcome = 'success';
    }
    edges.add(EditorEdge(from: from.id, to: to.id, on: outcome));
  }
  return edges;
}

/// Returns true when [edges] form a non-linear graph that cannot be
/// represented as a simple chain — i.e. any node has more than one
/// incoming edge, or nodes exist that are disconnected from the chain.
bool isComplexDag(List<EditorNode> nodes, List<EditorEdge> edges) {
  if (nodes.length <= 1 || edges.isEmpty) return false;

  // Any node with multiple incoming edges → complex
  final incomingCount = <String, int>{};
  for (final edge in edges) {
    incomingCount[edge.to] = (incomingCount[edge.to] ?? 0) + 1;
  }
  if (incomingCount.values.any((c) => c > 1)) return true;

  // Any non-first node not referenced by any edge → disconnected
  final referenced = <String>{};
  for (final edge in edges) {
    referenced.add(edge.from);
    referenced.add(edge.to);
  }
  for (int i = 1; i < nodes.length; i++) {
    if (!referenced.contains(nodes[i].id)) return true;
  }

  return false;
}

// ---------------------------------------------------------------------------
// Edge painter

class _EdgePainter extends CustomPainter {
  _EdgePainter({
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
  bool shouldRepaint(_EdgePainter old) => true;
}

// ---------------------------------------------------------------------------
// Workflow editor screen

class WorkflowEditorScreen extends ConsumerStatefulWidget {
  /// Pass [workflowId] to edit an existing workflow; null to create a new one.
  const WorkflowEditorScreen({super.key, this.workflowId});

  final String? workflowId;

  bool get isEdit => workflowId != null;

  @override
  ConsumerState<WorkflowEditorScreen> createState() =>
      _WorkflowEditorScreenState();
}

class _WorkflowEditorScreenState extends ConsumerState<WorkflowEditorScreen> {
  final _uuid = const Uuid();
  final _nameController = TextEditingController();
  final _descController = TextEditingController();

  List<EditorNode> _nodes = [];
  List<EditorEdge> _edges = [];
  int _maxSteps = 200;
  int _maxVisitsPerNode = 25;
  bool _active = false;

  // Connecting mode: id of the node we're drawing an edge from
  String? _connectingFrom;
  Offset? _pendingEdgeEnd;

  bool _submitting = false;
  String? _error;
  bool _loaded = false;

  // Tracks which layout branch is active; set during build by LayoutBuilder.
  bool _isMobileLayout = false;

  @override
  void initState() {
    super.initState();
    if (!widget.isEdit) {
      // Pre-populate with a manual trigger to get started
      _nodes = [
        EditorNode(
          id: _uuid.v4(),
          kind: 'trigger',
          config: {'type': 'manual'},
          position: const Offset((_kCanvasW / 2) - (_kNodeW / 2), 200),
        ),
      ];
    }
  }

  @override
  void dispose() {
    _nameController.dispose();
    _descController.dispose();
    super.dispose();
  }

  void _populateFromDetail(WorkflowDetailState state) {
    if (_loaded) return;
    _loaded = true;
    final detail = state.detail;
    _nameController.text = detail.name;
    _descController.text = detail.description;
    _active = detail.active;

    // Parse graph JSON
    final graphRaw = detail.graph?.value;
    if (graphRaw is Map) {
      final nodesRaw = graphRaw['nodes'] as List? ?? [];
      final edgesRaw = graphRaw['edges'] as List? ?? [];
      final execRaw = graphRaw['execution'] as Map? ?? {};
      _maxSteps = (execRaw['max_steps'] as int?) ?? 200;
      _maxVisitsPerNode = (execRaw['max_visits_per_node'] as int?) ?? 25;

      // Restore stored positions if available, otherwise cascade vertically
      double fallbackX = (_kCanvasW / 2) - (_kNodeW / 2);
      double fallbackY = 150;
      _nodes = nodesRaw.asMap().entries.map((entry) {
        final raw = entry.value as Map;
        final posRaw = raw['position'] as Map?;
        final Offset pos;
        if (posRaw != null) {
          pos = Offset(
            (posRaw['x'] as num).toDouble(),
            (posRaw['y'] as num).toDouble(),
          );
        } else {
          pos = Offset(fallbackX, fallbackY);
          fallbackY += _kNodeH + 60;
        }
        return EditorNode(
          id: raw['id'] as String,
          kind: raw['kind'] as String,
          config: Map<String, dynamic>.from(raw['config'] as Map? ?? {}),
          position: pos,
        );
      }).toList();

      _edges = edgesRaw.map((raw) {
        final r = raw as Map;
        return EditorEdge(
          from: r['from'] as String,
          to: r['to'] as String,
          on: r['on'] as String?,
        );
      }).toList();
    }
  }

  Map<String, dynamic> _buildGraph() {
    final edges = _isMobileLayout ? buildMobileEdges(_nodes) : _edges;
    return {
      'version': 1,
      'nodes': _nodes.map((n) => n.toJson()).toList(),
      'edges': edges.map((e) => e.toJson()).toList(),
      'execution': {
        'max_steps': _maxSteps,
        'max_visits_per_node': _maxVisitsPerNode,
      },
    };
  }

  String? _validateGraph() {
    if (_nameController.text.trim().isEmpty) return 'Workflow name is required';
    if (_nodes.isEmpty) return 'Add at least one node';
    final triggers = _nodes.where((n) => n.kind == 'trigger').length;
    if (triggers == 0) return 'At least one trigger node is required';
    if (triggers > 1) return 'Only one trigger node is allowed';
    return null;
  }

  Future<void> _save() async {
    final validationError = _validateGraph();
    if (validationError != null) {
      setState(() => _error = validationError);
      return;
    }

    setState(() {
      _submitting = true;
      _error = null;
    });

    final graph = _buildGraph();
    final notifier = ref.read(workflowsProvider.notifier);
    String? error;

    if (widget.isEdit) {
      error = await notifier.updateWorkflow(
        id: widget.workflowId!,
        name: _nameController.text.trim(),
        description: _descController.text.trim().isEmpty
            ? null
            : _descController.text.trim(),
        graph: graph,
        active: _active,
      );
    } else {
      error = await notifier.createWorkflow(
        name: _nameController.text.trim(),
        description: _descController.text.trim().isEmpty
            ? null
            : _descController.text.trim(),
        graph: graph,
        active: _active,
      );
    }

    if (!mounted) return;
    if (error != null) {
      setState(() {
        _submitting = false;
        _error = error;
      });
      return;
    }

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(widget.isEdit ? 'Workflow updated' : 'Workflow created'),
        behavior: SnackBarBehavior.floating,
      ),
    );
    context.go('/workflows');
  }

  void _startConnecting(String nodeId) {
    setState(() {
      _connectingFrom = nodeId;
      _pendingEdgeEnd = null;
    });
  }

  void _cancelConnecting() {
    setState(() {
      _connectingFrom = null;
      _pendingEdgeEnd = null;
    });
  }

  Future<void> _completeEdge(String toNodeId) async {
    if (_connectingFrom == null || _connectingFrom == toNodeId) {
      _cancelConnecting();
      return;
    }

    final fromNode = _nodes.firstWhere(
      (n) => n.id == _connectingFrom,
      orElse: () => _nodes.first,
    );
    final outcomes = fromNode.outcomesLabels;

    String? chosenOn;
    if (outcomes.length == 1) {
      chosenOn = outcomes.first;
    } else {
      chosenOn = await showDialog<String>(
        context: context,
        builder: (ctx) => SimpleDialog(
          title: const Text('Select edge outcome'),
          children: outcomes
              .map(
                (o) => SimpleDialogOption(
                  onPressed: () => Navigator.of(ctx).pop(o),
                  child: Text(o),
                ),
              )
              .toList(),
        ),
      );
    }

    if (chosenOn == null) {
      _cancelConnecting();
      return;
    }

    setState(() {
      _edges.add(
        EditorEdge(from: _connectingFrom!, to: toNodeId, on: chosenOn),
      );
      _connectingFrom = null;
      _pendingEdgeEnd = null;
    });
  }

  Future<void> _addNode() async {
    final result = await showModalBottomSheet<EditorNode>(
      context: context,
      isScrollControlled: true,
      builder: (ctx) => _NodePalette(
        onSelected: (kind, config) {
          Navigator.of(ctx).pop(
            EditorNode(
              id: _uuid.v4(),
              kind: kind,
              config: config,
              position: Offset(
                _kCanvasW / 2 - _kNodeW / 2,
                (_nodes.isEmpty ? 200 : _nodes.last.position.dy + _kNodeH + 80),
              ),
            ),
          );
        },
      ),
    );
    if (result != null) {
      setState(() => _nodes.add(result));
    }
  }

  Future<void> _editNode(EditorNode node) async {
    final updated = await showModalBottomSheet<EditorNode>(
      context: context,
      isScrollControlled: true,
      builder: (ctx) => _NodeConfigSheet(node: node),
    );
    if (updated != null) {
      setState(() {
        final idx = _nodes.indexWhere((n) => n.id == node.id);
        if (idx >= 0) _nodes[idx] = updated;
      });
    }
  }

  Future<void> _deleteNode(String nodeId) async {
    final confirmed = await showAdaptiveConfirmDialog(
      context: context,
      title: 'Delete node?',
      content: 'This will also remove all edges connected to this node.',
      confirmLabel: 'Delete',
      isDestructive: true,
    );
    if (!confirmed || !mounted) return;
    setState(() {
      _nodes.removeWhere((n) => n.id == nodeId);
      _edges.removeWhere((e) => e.from == nodeId || e.to == nodeId);
    });
  }

  void _deleteEdge(EditorEdge edge) {
    setState(() {
      _edges.remove(edge);
    });
  }

  Future<void> _showSettings() async {
    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      builder: (ctx) => _WorkflowSettingsSheet(
        nameController: _nameController,
        descController: _descController,
        active: _active,
        maxSteps: _maxSteps,
        maxVisitsPerNode: _maxVisitsPerNode,
        onChanged: (active, maxSteps, maxVisitsPerNode) {
          setState(() {
            _active = active;
            _maxSteps = maxSteps;
            _maxVisitsPerNode = maxVisitsPerNode;
          });
          Navigator.of(ctx).pop();
        },
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    // Load existing workflow if editing
    if (widget.isEdit) {
      final detailAsync = ref.watch(workflowDetailProvider(widget.workflowId!));
      detailAsync.whenData(_populateFromDetail);

      // Show loading/error until data is available
      if (!_loaded) {
        return Scaffold(
          appBar: AppBar(
            title: const Text('Edit Workflow'),
            leading: IconButton(
              icon: const Icon(Icons.arrow_back),
              onPressed: () => context.go('/workflows/${widget.workflowId}'),
            ),
          ),
          body: detailAsync.when(
            loading: () =>
                const Center(child: CircularProgressIndicator.adaptive()),
            error: (err, _) => Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    Icons.error_outline,
                    color: Theme.of(context).colorScheme.error,
                    size: 48,
                  ),
                  const SizedBox(height: 12),
                  Text(err.toString(), textAlign: TextAlign.center),
                  const SizedBox(height: 12),
                  FilledButton(
                    onPressed: () => ref
                        .read(
                          workflowDetailProvider(widget.workflowId!).notifier,
                        )
                        .refresh(),
                    child: const Text('Retry'),
                  ),
                ],
              ),
            ),
            data: (_) => const SizedBox.shrink(),
          ),
        );
      }
    }

    return LayoutBuilder(
      builder: (context, outerConstraints) {
        _isMobileLayout = outerConstraints.maxWidth < 600;
        final theme = Theme.of(context);

        return Scaffold(
          appBar: AppBar(
            title: Text(widget.isEdit ? 'Edit Workflow' : 'New Workflow'),
            leading: IconButton(
              icon: const Icon(Icons.arrow_back),
              onPressed: () => widget.isEdit
                  ? context.go('/workflows/${widget.workflowId}')
                  : context.go('/workflows'),
            ),
            actions: [
              IconButton(
                icon: const Icon(Icons.settings_outlined),
                tooltip: 'Workflow settings',
                onPressed: _showSettings,
              ),
              if (_submitting)
                const Padding(
                  padding: EdgeInsets.symmetric(horizontal: 16),
                  child: SizedBox(
                    width: 20,
                    height: 20,
                    child: CircularProgressIndicator.adaptive(strokeWidth: 2),
                  ),
                )
              else
                TextButton(
                  onPressed: _save,
                  child: Text(
                    widget.isEdit ? 'Save' : 'Create',
                    style: TextStyle(
                      fontWeight: FontWeight.w700,
                      color: theme.colorScheme.primary,
                    ),
                  ),
                ),
            ],
          ),
          body: Column(
            children: [
              // Error banner (shared between both layouts)
              if (_error != null)
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 8,
                  ),
                  color: theme.colorScheme.errorContainer,
                  child: Row(
                    children: [
                      Icon(
                        Icons.error_outline,
                        size: 16,
                        color: theme.colorScheme.onErrorContainer,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          _error!,
                          style: TextStyle(
                            fontSize: 12,
                            color: theme.colorScheme.onErrorContainer,
                          ),
                        ),
                      ),
                      IconButton(
                        icon: const Icon(Icons.close, size: 14),
                        onPressed: () => setState(() => _error = null),
                        padding: EdgeInsets.zero,
                        constraints: const BoxConstraints(),
                      ),
                    ],
                  ),
                ),

              // Layout branch
              Expanded(
                child: _isMobileLayout
                    ? _MobileWorkflowEditor(
                        nodes: _nodes,
                        edges: _edges,
                        onNodesChanged: (nodes) =>
                            setState(() => _nodes = nodes),
                        onAddNode: _addNode,
                        onEditNode: _editNode,
                        onDeleteNode: _deleteNode,
                      )
                    : _DesktopCanvasEditor(
                        nodes: _nodes,
                        edges: _edges,
                        connectingFrom: _connectingFrom,
                        pendingEdgeEnd: _pendingEdgeEnd,
                        onDragNode: (node, delta) {
                          setState(() {
                            node.position = Offset(
                              (node.position.dx + delta.dx).clamp(
                                0,
                                _kCanvasW - _kNodeW,
                              ),
                              (node.position.dy + delta.dy).clamp(
                                0,
                                _kCanvasH - _kNodeH,
                              ),
                            );
                          });
                        },
                        onEditNode: _editNode,
                        onDeleteNode: _deleteNode,
                        onStartConnect: _startConnecting,
                        onCompleteEdge: _completeEdge,
                        onCancelConnecting: _cancelConnecting,
                        onUpdatePendingEdge: (offset) =>
                            setState(() => _pendingEdgeEnd = offset),
                        onDeleteEdge: _deleteEdge,
                      ),
              ),
            ],
          ),
          floatingActionButton: _isMobileLayout
              ? null
              : FloatingActionButton(
                  onPressed: _addNode,
                  tooltip: 'Add node',
                  child: const Icon(Icons.add),
                ),
        );
      },
    );
  }
}

// ---------------------------------------------------------------------------
// Desktop canvas editor (extracted from _WorkflowEditorScreenState.build)

class _DesktopCanvasEditor extends StatelessWidget {
  const _DesktopCanvasEditor({
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
            constrained: false,
            boundaryMargin: const EdgeInsets.all(100),
            minScale: 0.4,
            maxScale: 2.5,
            child: SizedBox(
              width: _kCanvasW,
              height: _kCanvasH,
              child: Stack(
                children: [
                  // Grid background
                  Positioned.fill(
                    child: CustomPaint(
                      painter: _GridPainter(
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
                        painter: _EdgePainter(
                          nodes: nodes,
                          edges: edges,
                          nodeSize: _kNodeSize,
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
                      nodeSize: _kNodeSize,
                      onDelete: () => onDeleteEdge(edge),
                    ),
                  ),

                  // Nodes
                  ...nodes.map(
                    (node) => _NodeWidget(
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
// Mobile workflow editor

class _MobileWorkflowEditor extends StatelessWidget {
  const _MobileWorkflowEditor({
    required this.nodes,
    required this.edges,
    required this.onNodesChanged,
    required this.onAddNode,
    required this.onEditNode,
    required this.onDeleteNode,
  });

  final List<EditorNode> nodes;
  final List<EditorEdge> edges;
  final void Function(List<EditorNode> nodes) onNodesChanged;
  final VoidCallback onAddNode;
  final void Function(EditorNode node) onEditNode;
  final void Function(String nodeId) onDeleteNode;

  @override
  Widget build(BuildContext context) {
    final complex = isComplexDag(nodes, edges);

    return Column(
      children: [
        // Complex-graph banner
        if (complex)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
            color: Colors.orange.withAlpha(20),
            child: Row(
              children: [
                const Icon(Icons.info_outline, size: 16, color: Colors.orange),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Complex graph — edit branching on a wider screen',
                    style: TextStyle(fontSize: 12, color: Colors.orange),
                  ),
                ),
              ],
            ),
          ),

        // Reorderable card list
        Expanded(
          child: ReorderableListView.builder(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
            itemCount: nodes.length,
            onReorder: (oldIndex, newIndex) {
              if (newIndex > oldIndex) newIndex--;
              final updated = List<EditorNode>.from(nodes);
              final item = updated.removeAt(oldIndex);
              updated.insert(newIndex, item);
              onNodesChanged(updated);
            },
            itemBuilder: (context, index) {
              final node = nodes[index];
              return _MobileNodeCard(
                key: ValueKey(node.id),
                node: node,
                index: index,
                onEdit: () => onEditNode(node),
                onDelete: () => onDeleteNode(node.id),
              );
            },
          ),
        ),

        // Add step button
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
          child: FilledButton.icon(
            onPressed: onAddNode,
            icon: const Icon(Icons.add),
            label: const Text('Add step'),
            style: FilledButton.styleFrom(
              minimumSize: const Size.fromHeight(48),
            ),
          ),
        ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// Mobile node card

class _MobileNodeCard extends StatelessWidget {
  const _MobileNodeCard({
    super.key,
    required this.node,
    required this.index,
    required this.onEdit,
    required this.onDelete,
  });

  final EditorNode node;
  final int index;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final color = node.color;

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: color.withValues(alpha: 0.4), width: 1.5),
      ),
      child: ListTile(
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
        leading: Container(
          width: 36,
          height: 36,
          decoration: BoxDecoration(
            color: color.withValues(alpha: 0.12),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Icon(node.icon, color: color, size: 20),
        ),
        title: Text(
          node.label,
          style: TextStyle(
            fontWeight: FontWeight.w600,
            color: color.withValues(alpha: 0.9),
          ),
        ),
        subtitle: Text(node.kind, style: const TextStyle(fontSize: 11)),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            IconButton(
              icon: Icon(Icons.edit_outlined, size: 20, color: color),
              onPressed: onEdit,
              tooltip: 'Edit',
              visualDensity: VisualDensity.compact,
            ),
            IconButton(
              icon: Icon(
                Icons.delete_outline,
                size: 20,
                color: Theme.of(context).colorScheme.error,
              ),
              onPressed: onDelete,
              tooltip: 'Delete',
              visualDensity: VisualDensity.compact,
            ),
            ReorderableDelayedDragStartListener(
              index: index,
              child: Icon(
                Icons.drag_handle,
                size: 24,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Grid background painter

class _GridPainter extends CustomPainter {
  _GridPainter({required this.gridColor});

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
  bool shouldRepaint(_GridPainter old) => gridColor != old.gridColor;
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
      left: mid.dx - 10,
      top: mid.dy - 10,
      child: GestureDetector(
        onTap: onDelete,
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
    );
  }
}

// ---------------------------------------------------------------------------
// Node widget (canvas)

class _NodeWidget extends StatelessWidget {
  const _NodeWidget({
    super.key,
    required this.node,
    required this.isConnectingFrom,
    required this.isConnecting,
    required this.onDrag,
    required this.onEdit,
    required this.onDelete,
    required this.onStartConnect,
    this.onTap,
  });

  final EditorNode node;
  final bool isConnectingFrom;
  final bool isConnecting;
  final void Function(Offset delta) onDrag;
  final VoidCallback onEdit;
  final VoidCallback onDelete;
  final VoidCallback onStartConnect;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final color = node.color;

    return Positioned(
      left: node.position.dx,
      top: node.position.dy,
      child: GestureDetector(
        onPanUpdate: (details) => onDrag(details.delta),
        onTap: onTap,
        child: SizedBox(
          width: _kNodeW,
          height: _kNodeH,
          child: Stack(
            clipBehavior: Clip.none,
            children: [
              // Main card
              Material(
                elevation: isConnectingFrom ? 6 : 2,
                borderRadius: BorderRadius.circular(12),
                color: isConnecting && !isConnectingFrom
                    ? Theme.of(context).colorScheme.surface
                    : color.withValues(alpha: 0.08),
                child: Container(
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(
                      color: isConnectingFrom
                          ? color
                          : color.withValues(alpha: 0.4),
                      width: isConnectingFrom ? 2.5 : 1.5,
                    ),
                  ),
                  child: Padding(
                    padding: const EdgeInsets.all(10),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Icon(node.icon, size: 16, color: color),
                            const SizedBox(width: 6),
                            Expanded(
                              child: Text(
                                node.label,
                                style: TextStyle(
                                  fontSize: 12,
                                  fontWeight: FontWeight.w700,
                                  color: color.withValues(alpha: 0.9),
                                ),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 4),
                        Text(
                          node.id.length > 8
                              ? node.id.substring(0, 8)
                              : node.id,
                          style: TextStyle(
                            fontSize: 10,
                            fontFamily: 'monospace',
                            color: Theme.of(
                              context,
                            ).colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),

              // Action buttons (shown when NOT in connecting mode)
              if (!isConnecting) ...[
                // Edit button (top-right)
                Positioned(
                  top: -8,
                  right: 24,
                  child: GestureDetector(
                    onTap: onEdit,
                    child: Container(
                      width: 22,
                      height: 22,
                      decoration: BoxDecoration(
                        color: color,
                        shape: BoxShape.circle,
                        boxShadow: [
                          BoxShadow(
                            color: color.withValues(alpha: 0.3),
                            blurRadius: 4,
                          ),
                        ],
                      ),
                      child: Icon(
                        Icons.edit,
                        size: 12,
                        color: Theme.of(context).colorScheme.surface,
                      ),
                    ),
                  ),
                ),

                // Delete button (top-right, offset)
                Positioned(
                  top: -8,
                  right: -2,
                  child: GestureDetector(
                    onTap: onDelete,
                    child: Container(
                      width: 22,
                      height: 22,
                      decoration: BoxDecoration(
                        color: Theme.of(context).colorScheme.error,
                        shape: BoxShape.circle,
                        boxShadow: [
                          BoxShadow(
                            color: Theme.of(
                              context,
                            ).colorScheme.error.withValues(alpha: 0.3),
                            blurRadius: 4,
                          ),
                        ],
                      ),
                      child: Icon(
                        Icons.close,
                        size: 12,
                        color: Theme.of(context).colorScheme.surface,
                      ),
                    ),
                  ),
                ),

                // Connect button (bottom-center)
                Positioned(
                  bottom: -10,
                  left: _kNodeW / 2 - 11,
                  child: GestureDetector(
                    onTap: onStartConnect,
                    child: Container(
                      width: 22,
                      height: 22,
                      decoration: BoxDecoration(
                        color: color,
                        shape: BoxShape.circle,
                        boxShadow: [
                          BoxShadow(
                            color: color.withValues(alpha: 0.3),
                            blurRadius: 4,
                          ),
                        ],
                      ),
                      child: Icon(
                        Icons.arrow_downward,
                        size: 12,
                        color: Theme.of(context).colorScheme.surface,
                      ),
                    ),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Node palette bottom sheet

class _NodePalette extends StatelessWidget {
  const _NodePalette({required this.onSelected});

  final void Function(String kind, Map<String, dynamic> config) onSelected;

  @override
  Widget build(BuildContext context) {
    return DraggableScrollableSheet(
      initialChildSize: 0.6,
      maxChildSize: 0.9,
      minChildSize: 0.4,
      expand: false,
      builder: (ctx, sc) => Column(
        children: [
          const _SheetHandle(),
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 4),
            child: Row(
              children: [
                Text(
                  'Add Node',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const Spacer(),
                IconButton(
                  icon: const Icon(Icons.close),
                  onPressed: () => Navigator.of(ctx).pop(),
                ),
              ],
            ),
          ),
          Expanded(
            child: ListView(
              controller: sc,
              padding: const EdgeInsets.symmetric(horizontal: 16),
              children: [
                _PaletteSection(
                  label: 'Triggers',
                  color: Colors.blue,
                  items: [
                    _PaletteItem(
                      title: 'Manual',
                      subtitle: 'Triggered manually or by test run',
                      icon: Icons.touch_app,
                      onTap: () => onSelected('trigger', {'type': 'manual'}),
                    ),
                    _PaletteItem(
                      title: 'Webhook',
                      subtitle: 'Triggered via inbound HTTP webhook',
                      icon: Icons.webhook,
                      onTap: () => onSelected('trigger', {'type': 'webhook'}),
                    ),
                    _PaletteItem(
                      title: 'Schedule',
                      subtitle: 'Triggered on a cron schedule',
                      icon: Icons.schedule,
                      onTap: () => onSelected('trigger', {
                        'type': 'schedule',
                        'cron': '0 * * * *',
                      }),
                    ),
                    _PaletteItem(
                      title: 'Event',
                      subtitle: 'Triggered by an internal bus event',
                      icon: Icons.notifications_active_outlined,
                      onTap: () => onSelected('trigger', {
                        'type': 'event',
                        'event': 'message.completed',
                      }),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                _PaletteSection(
                  label: 'Actions',
                  color: Colors.green,
                  items: [
                    _PaletteItem(
                      title: 'Assistant Turn',
                      subtitle: 'Run a prompt through the assistant',
                      icon: Icons.smart_toy_outlined,
                      onTap: () => onSelected('action', {
                        'type': 'assistant_turn',
                        'prompt': '',
                      }),
                    ),
                    _PaletteItem(
                      title: 'HTTP Request',
                      subtitle: 'Make an outbound HTTP call',
                      icon: Icons.http,
                      onTap: () => onSelected('action', {
                        'type': 'http_request',
                        'url': 'https://',
                        'method': 'GET',
                      }),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                _PaletteSection(
                  label: 'Conditions',
                  color: Colors.orange,
                  items: [
                    _PaletteItem(
                      title: 'Always True',
                      subtitle: 'Always routes to the "true" branch',
                      icon: Icons.check_circle_outline,
                      onTap: () =>
                          onSelected('condition', {'type': 'always_true'}),
                    ),
                    _PaletteItem(
                      title: 'Always False',
                      subtitle: 'Always routes to the "false" branch',
                      icon: Icons.cancel_outlined,
                      onTap: () =>
                          onSelected('condition', {'type': 'always_false'}),
                    ),
                    _PaletteItem(
                      title: 'Payload Key Check',
                      subtitle: 'Check if trigger payload has a key',
                      icon: Icons.vpn_key_outlined,
                      onTap: () => onSelected('condition', {
                        'type': 'has_trigger_payload_key',
                        'key': '',
                      }),
                    ),
                  ],
                ),
                const SizedBox(height: 24),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _PaletteSection extends StatelessWidget {
  const _PaletteSection({
    required this.label,
    required this.color,
    required this.items,
  });

  final String label;
  final Color color;
  final List<Widget> items;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Container(
              width: 12,
              height: 12,
              decoration: BoxDecoration(color: color, shape: BoxShape.circle),
            ),
            const SizedBox(width: 6),
            Text(
              label.toUpperCase(),
              style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w700,
                letterSpacing: 0.5,
                color: color,
              ),
            ),
          ],
        ),
        const SizedBox(height: 6),
        ...items,
      ],
    );
  }
}

class _PaletteItem extends StatelessWidget {
  const _PaletteItem({
    required this.title,
    required this.subtitle,
    required this.icon,
    required this.onTap,
  });

  final String title;
  final String subtitle;
  final IconData icon;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      dense: true,
      leading: Icon(icon, size: 20),
      title: Text(title, style: const TextStyle(fontWeight: FontWeight.w600)),
      subtitle: Text(subtitle, style: const TextStyle(fontSize: 11)),
      onTap: onTap,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
    );
  }
}

// ---------------------------------------------------------------------------
// Node config bottom sheet

class _NodeConfigSheet extends StatefulWidget {
  const _NodeConfigSheet({required this.node});

  final EditorNode node;

  @override
  State<_NodeConfigSheet> createState() => _NodeConfigSheetState();
}

class _NodeConfigSheetState extends State<_NodeConfigSheet> {
  late Map<String, dynamic> _config;
  late TextEditingController _cronCtrl;
  late TextEditingController _eventCtrl;
  late TextEditingController _promptCtrl;
  late TextEditingController _urlCtrl;
  late TextEditingController _keyCtrl;
  String _method = 'GET';

  @override
  void initState() {
    super.initState();
    _config = Map<String, dynamic>.from(widget.node.config);
    _cronCtrl = TextEditingController(text: _config['cron'] as String? ?? '');
    _eventCtrl = TextEditingController(text: _config['event'] as String? ?? '');
    _promptCtrl = TextEditingController(
      text: _config['prompt'] as String? ?? '',
    );
    _urlCtrl = TextEditingController(text: _config['url'] as String? ?? '');
    _keyCtrl = TextEditingController(text: _config['key'] as String? ?? '');
    _method = _config['method'] as String? ?? 'GET';
  }

  @override
  void dispose() {
    _cronCtrl.dispose();
    _eventCtrl.dispose();
    _promptCtrl.dispose();
    _urlCtrl.dispose();
    _keyCtrl.dispose();
    super.dispose();
  }

  void _save() {
    final nodeType = _config['type'] as String? ?? '';
    switch (nodeType) {
      case 'schedule':
        _config['cron'] = _cronCtrl.text.trim();
        break;
      case 'event':
        _config['event'] = _eventCtrl.text.trim();
        break;
      case 'assistant_turn':
        _config['prompt'] = _promptCtrl.text;
        break;
      case 'http_request':
        _config['url'] = _urlCtrl.text.trim();
        _config['method'] = _method;
        break;
      case 'has_trigger_payload_key':
        _config['key'] = _keyCtrl.text.trim();
        break;
    }

    final updated = widget.node.copyWith(config: _config);
    Navigator.of(context).pop(updated);
  }

  @override
  Widget build(BuildContext context) {
    final nodeType = _config['type'] as String? ?? widget.node.kind;
    final color = widget.node.color;

    return Padding(
      padding: EdgeInsets.only(
        bottom: MediaQuery.of(context).viewInsets.bottom,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const _SheetHandle(),
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            child: Row(
              children: [
                Icon(widget.node.icon, color: color, size: 20),
                const SizedBox(width: 8),
                Text(
                  'Configure: ${widget.node.label}',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: _buildConfigFields(nodeType),
          ),
          const SizedBox(height: 16),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: FilledButton(onPressed: _save, child: const Text('Apply')),
          ),
          const SizedBox(height: 24),
        ],
      ),
    );
  }

  Widget _buildConfigFields(String nodeType) {
    switch (nodeType) {
      case 'manual':
      case 'webhook':
      case 'always_true':
      case 'always_false':
        return Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: Theme.of(
              context,
            ).colorScheme.onSurface.withValues(alpha: 0.04),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Text(
            'No configuration required for this node type.',
            style: TextStyle(
              fontSize: 13,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        );

      case 'schedule':
        return TextField(
          controller: _cronCtrl,
          decoration: const InputDecoration(
            labelText: 'Cron expression',
            hintText: '0 * * * *',
            helperText: 'Standard 5-field cron: min hour day month weekday',
            border: OutlineInputBorder(),
          ),
          style: const TextStyle(fontFamily: 'monospace'),
        );

      case 'event':
        return TextField(
          controller: _eventCtrl,
          decoration: const InputDecoration(
            labelText: 'Event topic',
            hintText: 'message.completed',
            border: OutlineInputBorder(),
          ),
          style: const TextStyle(fontFamily: 'monospace'),
        );

      case 'assistant_turn':
        return TextField(
          controller: _promptCtrl,
          minLines: 4,
          maxLines: null,
          decoration: const InputDecoration(
            labelText: 'Prompt',
            hintText: 'Enter the assistant prompt…',
            border: OutlineInputBorder(),
            alignLabelWithHint: true,
          ),
        );

      case 'http_request':
        return Column(
          children: [
            DropdownButtonFormField<String>(
              initialValue: _method,
              decoration: const InputDecoration(
                labelText: 'HTTP method',
                border: OutlineInputBorder(),
              ),
              items: [
                'GET',
                'POST',
                'PUT',
                'PATCH',
                'DELETE',
              ].map((m) => DropdownMenuItem(value: m, child: Text(m))).toList(),
              onChanged: (v) => setState(() => _method = v ?? 'GET'),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _urlCtrl,
              decoration: const InputDecoration(
                labelText: 'URL',
                hintText: 'https://example.com/webhook',
                border: OutlineInputBorder(),
              ),
              keyboardType: TextInputType.url,
            ),
          ],
        );

      case 'has_trigger_payload_key':
        return TextField(
          controller: _keyCtrl,
          decoration: const InputDecoration(
            labelText: 'Payload key',
            hintText: 'e.g. user_id',
            border: OutlineInputBorder(),
          ),
          style: const TextStyle(fontFamily: 'monospace'),
        );

      default:
        return Text('Unknown node type: $nodeType');
    }
  }
}

// ---------------------------------------------------------------------------
// Workflow settings sheet

class _WorkflowSettingsSheet extends StatefulWidget {
  const _WorkflowSettingsSheet({
    required this.nameController,
    required this.descController,
    required this.active,
    required this.maxSteps,
    required this.maxVisitsPerNode,
    required this.onChanged,
  });

  final TextEditingController nameController;
  final TextEditingController descController;
  final bool active;
  final int maxSteps;
  final int maxVisitsPerNode;
  final void Function(bool active, int maxSteps, int maxVisitsPerNode)
  onChanged;

  @override
  State<_WorkflowSettingsSheet> createState() => _WorkflowSettingsSheetState();
}

class _WorkflowSettingsSheetState extends State<_WorkflowSettingsSheet> {
  late bool _active;
  late int _maxSteps;
  late int _maxVisitsPerNode;

  @override
  void initState() {
    super.initState();
    _active = widget.active;
    _maxSteps = widget.maxSteps;
    _maxVisitsPerNode = widget.maxVisitsPerNode;
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.only(
        bottom: MediaQuery.of(context).viewInsets.bottom,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const _SheetHandle(),
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            child: Text(
              'Workflow Settings',
              style: Theme.of(context).textTheme.titleMedium,
            ),
          ),
          const SizedBox(height: 12),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Column(
              children: [
                TextField(
                  controller: widget.nameController,
                  decoration: const InputDecoration(
                    labelText: 'Name',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: widget.descController,
                  maxLines: 2,
                  decoration: const InputDecoration(
                    labelText: 'Description (optional)',
                    border: OutlineInputBorder(),
                    alignLabelWithHint: true,
                  ),
                ),
                const SizedBox(height: 12),
                SwitchListTile(
                  title: const Text('Active'),
                  subtitle: const Text('Enable automatic triggering'),
                  value: _active,
                  onChanged: (v) => setState(() => _active = v),
                  contentPadding: EdgeInsets.zero,
                ),
                Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Max steps: $_maxSteps',
                            style: const TextStyle(fontSize: 13),
                          ),
                          Slider(
                            value: _maxSteps.toDouble(),
                            min: 10,
                            max: 500,
                            divisions: 49,
                            onChanged: (v) =>
                                setState(() => _maxSteps = v.round()),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Max visits/node: $_maxVisitsPerNode',
                            style: const TextStyle(fontSize: 13),
                          ),
                          Slider(
                            value: _maxVisitsPerNode.toDouble(),
                            min: 1,
                            max: 100,
                            divisions: 99,
                            onChanged: (v) =>
                                setState(() => _maxVisitsPerNode = v.round()),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: FilledButton(
              onPressed: () =>
                  widget.onChanged(_active, _maxSteps, _maxVisitsPerNode),
              child: const Text('Apply Settings'),
            ),
          ),
          const SizedBox(height: 24),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Shared sheet handle

class _SheetHandle extends StatelessWidget {
  const _SheetHandle();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.only(top: 12, bottom: 4),
        child: Container(
          width: 40,
          height: 4,
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.outlineVariant,
            borderRadius: BorderRadius.circular(2),
          ),
        ),
      ),
    );
  }
}
