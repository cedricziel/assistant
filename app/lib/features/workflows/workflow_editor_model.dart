import 'package:material_ui/material_ui.dart';

// ---------------------------------------------------------------------------
// Canvas layout constants (shared across canvas widgets)

const kEditorNodeW = 160.0;
const kEditorNodeH = 72.0;
const kEditorNodeSize = Size(kEditorNodeW, kEditorNodeH);
const kEditorCanvasW = 3000.0;
const kEditorCanvasH = 3000.0;

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

  bool get disabled => config['disabled'] == true;

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
