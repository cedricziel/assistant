import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:uuid/uuid.dart';

import '../../shared/platform/adaptive_dialog.dart';
import '../personas/personas_provider.dart';
import 'workflows_provider.dart';
import 'workflow_editor_model.dart';
import 'workflow_desktop_canvas.dart';
import 'workflow_mobile_editor.dart';
import 'workflow_node_palette.dart';
import 'workflow_node_config_sheet.dart';
import 'workflow_settings_sheet.dart';

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
  final _canvasController = TransformationController();

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
  bool _dirty = false;
  bool _initialViewportSet = false;

  // Tracks which layout branch is active; set during build by LayoutBuilder.
  bool _isMobileLayout = false;

  void _markDirty() {
    if (!_dirty) setState(() => _dirty = true);
  }

  @override
  void initState() {
    super.initState();
    _nameController.addListener(_markDirty);
    _descController.addListener(_markDirty);
    if (!widget.isEdit) {
      // Pre-populate with a manual trigger to get started
      _nodes = [
        EditorNode(
          id: _uuid.v4(),
          kind: 'trigger',
          config: {'type': 'manual'},
          position: const Offset(
            (kEditorCanvasW / 2) - (kEditorNodeW / 2),
            200,
          ),
        ),
      ];
    }
  }

  @override
  void dispose() {
    _nameController.dispose();
    _descController.dispose();
    _canvasController.dispose();
    super.dispose();
  }

  /// Returns the bounding rect around all nodes (in canvas coords).
  Rect _nodeBounds() {
    if (_nodes.isEmpty) return Rect.zero;
    double minX = double.infinity, minY = double.infinity;
    double maxX = double.negativeInfinity, maxY = double.negativeInfinity;
    for (final n in _nodes) {
      if (n.position.dx < minX) minX = n.position.dx;
      if (n.position.dy < minY) minY = n.position.dy;
      if (n.position.dx + kEditorNodeW > maxX) {
        maxX = n.position.dx + kEditorNodeW;
      }
      if (n.position.dy + kEditorNodeH > maxY) {
        maxY = n.position.dy + kEditorNodeH;
      }
    }
    return Rect.fromLTRB(minX, minY, maxX, maxY);
  }

  /// Pans and scales so all nodes fit in the viewport with padding.
  void _fitToView(Size viewportSize) {
    if (_nodes.isEmpty) return;
    final bounds = _nodeBounds();
    const padding = 80.0;
    final padded = bounds.inflate(padding);
    final scaleX = viewportSize.width / padded.width;
    final scaleY = viewportSize.height / padded.height;
    final scale = math.min(scaleX, scaleY).clamp(0.4, 2.5);
    final tx =
        (viewportSize.width - padded.width * scale) / 2 - padded.left * scale;
    final ty =
        (viewportSize.height - padded.height * scale) / 2 - padded.top * scale;
    _canvasController.value = Matrix4.identity()
      ..setTranslationRaw(tx, ty, 0)
      ..scaleByDouble(scale, scale, 1.0, 1.0);
  }

  /// Auto-layout: arrange nodes top-down using topological order.
  void _autoLayout() {
    if (_nodes.isEmpty) return;
    // Build adjacency from edges.
    final order = <String>[];
    final visited = <String>{};
    final adj = <String, List<String>>{};
    for (final e in _edges) {
      adj.putIfAbsent(e.from, () => []).add(e.to);
    }

    void visit(String id) {
      if (visited.contains(id)) return;
      visited.add(id);
      for (final child in adj[id] ?? []) {
        visit(child);
      }
      order.insert(0, id);
    }

    // Start from triggers, then any remaining.
    for (final n in _nodes.where((n) => n.kind == 'trigger')) {
      visit(n.id);
    }
    for (final n in _nodes) {
      visit(n.id);
    }

    // Assign positions: center horizontally, cascade vertically.
    const startY = 150.0;
    const spacingY = kEditorNodeH + 80;

    // Assign depth per node for multi-column layout.
    final depth = <String, int>{};
    for (final id in order) {
      var d = 0;
      // Find max depth of parents + 1.
      for (final e in _edges.where((e) => e.to == id)) {
        final pd = depth[e.from] ?? 0;
        if (pd + 1 > d) d = pd + 1;
      }
      depth[id] = d;
    }

    // Group by depth.
    final byDepth = <int, List<String>>{};
    for (final entry in depth.entries) {
      byDepth.putIfAbsent(entry.value, () => []).add(entry.key);
    }

    setState(() {
      for (final entry in byDepth.entries) {
        final d = entry.key;
        final ids = entry.value;
        final totalW = ids.length * kEditorNodeW + (ids.length - 1) * 40;
        var x = (kEditorCanvasW - totalW) / 2;
        for (final id in ids) {
          final idx = _nodes.indexWhere((n) => n.id == id);
          if (idx >= 0) {
            _nodes[idx].position = Offset(x, startY + d * spacingY);
          }
          x += kEditorNodeW + 40;
        }
      }
      _dirty = true;
    });
  }

  Future<bool> _maybeDiscardChanges() async {
    if (!_dirty) return true;
    final discard = await showAdaptiveConfirmDialog(
      context: context,
      title: 'Discard changes?',
      content: 'You have unsaved changes that will be lost.',
      confirmLabel: 'Discard',
      isDestructive: true,
    );
    return discard;
  }

  Future<void> _navigateBack() async {
    if (!await _maybeDiscardChanges()) return;
    if (!mounted) return;
    if (widget.isEdit) {
      context.go('/workflows/${widget.workflowId}');
    } else {
      context.go('/workflows');
    }
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
      double fallbackX = (kEditorCanvasW / 2) - (kEditorNodeW / 2);
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
          fallbackY += kEditorNodeH + 60;
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
    // Loading from server is not a user edit — reset dirty flag.
    _dirty = false;
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

    // Block saving complex branching graphs from mobile layout — mobile uses
    // a linear edge chain which would silently destroy branching edges.
    if (_isMobileLayout && isComplexDag(_nodes, _edges)) {
      setState(
        () => _error =
            'This workflow has a complex graph that cannot be saved from mobile layout. '
            'Use a wider screen to preserve branching edges.',
      );
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

    _dirty = false; // Prevent discard prompt on navigation.
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
      _dirty = true;
    });
  }

  Future<void> _addNode() async {
    final result = await showModalBottomSheet<EditorNode>(
      context: context,
      isScrollControlled: true,
      builder: (ctx) => NodePalette(
        onSelected: (kind, config) {
          Navigator.of(ctx).pop(
            EditorNode(
              id: _uuid.v4(),
              kind: kind,
              config: config,
              position: Offset(
                kEditorCanvasW / 2 - kEditorNodeW / 2,
                (_nodes.isEmpty
                    ? 200
                    : _nodes.last.position.dy + kEditorNodeH + 80),
              ),
            ),
          );
        },
      ),
    );
    if (result != null) {
      setState(() {
        _nodes.add(result);
        _dirty = true;
      });
    }
  }

  Future<void> _editNode(EditorNode node) async {
    final personas = ref.read(personasProvider).value?.personas ?? [];
    final personaNames = personas.map((p) => (p.id, p.name)).toList();
    final updated = await showModalBottomSheet<EditorNode>(
      context: context,
      isScrollControlled: true,
      builder: (ctx) => NodeConfigSheet(node: node, personaNames: personaNames),
    );
    if (updated != null) {
      setState(() {
        final idx = _nodes.indexWhere((n) => n.id == node.id);
        if (idx >= 0) _nodes[idx] = updated;
        _dirty = true;
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
      _dirty = true;
    });
  }

  void _deleteEdge(EditorEdge edge) {
    setState(() {
      _edges.remove(edge);
      _dirty = true;
    });
  }

  Future<void> _showSettings() async {
    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      builder: (ctx) => WorkflowSettingsSheet(
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
            _dirty = true;
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

        // Center canvas on nodes once after first layout.
        if (!_initialViewportSet && !_isMobileLayout && _nodes.isNotEmpty) {
          _initialViewportSet = true;
          WidgetsBinding.instance.addPostFrameCallback((_) {
            _fitToView(
              Size(outerConstraints.maxWidth, outerConstraints.maxHeight),
            );
          });
        }

        return PopScope(
          canPop: !_dirty,
          onPopInvokedWithResult: (didPop, _) async {
            if (didPop) return;
            await _navigateBack();
          },
          child: Scaffold(
            appBar: AppBar(
              title: Text(widget.isEdit ? 'Edit Workflow' : 'New Workflow'),
              leading: IconButton(
                icon: const Icon(Icons.arrow_back),
                onPressed: _navigateBack,
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
                        ),
                      ],
                    ),
                  ),

                // Layout branch
                Expanded(
                  child: _isMobileLayout
                      ? MobileWorkflowEditor(
                          nodes: _nodes,
                          edges: _edges,
                          onNodesChanged: (nodes) => setState(() {
                            _nodes = nodes;
                            _dirty = true;
                          }),
                          onAddNode: _addNode,
                          onEditNode: _editNode,
                          onDeleteNode: _deleteNode,
                        )
                      : DesktopCanvasEditor(
                          canvasController: _canvasController,
                          nodes: _nodes,
                          edges: _edges,
                          connectingFrom: _connectingFrom,
                          pendingEdgeEnd: _pendingEdgeEnd,
                          onDragNode: (node, delta) {
                            setState(() {
                              node.position = Offset(
                                (node.position.dx + delta.dx).clamp(
                                  0,
                                  kEditorCanvasW - kEditorNodeW,
                                ),
                                (node.position.dy + delta.dy).clamp(
                                  0,
                                  kEditorCanvasH - kEditorNodeH,
                                ),
                              );
                              _dirty = true;
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
                : Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      FloatingActionButton.small(
                        heroTag: 'fit',
                        onPressed: () => _fitToView(
                          Size(
                            outerConstraints.maxWidth,
                            outerConstraints.maxHeight,
                          ),
                        ),
                        tooltip: 'Fit to view',
                        child: const Icon(Icons.fit_screen),
                      ),
                      const SizedBox(height: 8),
                      FloatingActionButton.small(
                        heroTag: 'layout',
                        onPressed: () {
                          _autoLayout();
                          // Re-fit after layout so nodes are visible.
                          WidgetsBinding.instance.addPostFrameCallback((_) {
                            _fitToView(
                              Size(
                                outerConstraints.maxWidth,
                                outerConstraints.maxHeight,
                              ),
                            );
                          });
                        },
                        tooltip: 'Auto-layout',
                        child: const Icon(Icons.account_tree),
                      ),
                      const SizedBox(height: 8),
                      FloatingActionButton(
                        heroTag: 'add',
                        onPressed: _addNode,
                        tooltip: 'Add node',
                        child: const Icon(Icons.add),
                      ),
                    ],
                  ),
          ),
        );
      },
    );
  }
}
