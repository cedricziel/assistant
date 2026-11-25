import 'package:material_ui/material_ui.dart';

import 'workflow_editor_model.dart';

// ---------------------------------------------------------------------------
// Mobile workflow editor

class MobileWorkflowEditor extends StatelessWidget {
  const MobileWorkflowEditor({
    super.key,
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
            // Flutter 3.44 deprecated `onReorder` in favour of
            // `onReorderItem` (which adjusts `newIndex` automatically). The
            // new callback isn't available in Flutter 3.41, so we keep the
            // manual `newIndex--` adjustment behind a deprecation ignore.
            // Migrate to `onReorderItem` once the minimum Flutter version
            // is bumped to >= 3.42.
            // ignore: deprecated_member_use
            onReorder: (oldIndex, newIndex) {
              if (newIndex > oldIndex) newIndex--;
              final updated = List<EditorNode>.from(nodes);
              final item = updated.removeAt(oldIndex);
              updated.insert(newIndex, item);
              onNodesChanged(updated);
            },
            itemBuilder: (context, index) {
              final node = nodes[index];
              return MobileNodeCard(
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

class MobileNodeCard extends StatelessWidget {
  const MobileNodeCard({
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
    final isDisabled = node.disabled;

    return Opacity(
      opacity: isDisabled ? 0.5 : 1.0,
      child: Card(
        margin: const EdgeInsets.only(bottom: 8),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: BorderSide(color: color.withValues(alpha: 0.4), width: 1.5),
        ),
        child: ListTile(
          contentPadding: const EdgeInsets.symmetric(
            horizontal: 16,
            vertical: 4,
          ),
          leading: Container(
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              color: color.withValues(alpha: 0.12),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Icon(node.icon, color: color, size: 20),
          ),
          title: Row(
            children: [
              Expanded(
                child: Text(
                  node.label,
                  style: TextStyle(
                    fontWeight: FontWeight.w600,
                    color: color.withValues(alpha: 0.9),
                  ),
                ),
              ),
              if (isDisabled)
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 6,
                    vertical: 2,
                  ),
                  decoration: BoxDecoration(
                    color: Colors.grey.withValues(alpha: 0.15),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: const Text(
                    'Disabled',
                    style: TextStyle(fontSize: 10, color: Colors.grey),
                  ),
                ),
            ],
          ),
          subtitle: Text(node.kind, style: const TextStyle(fontSize: 11)),
          trailing: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              IconButton(
                icon: Icon(Icons.edit_outlined, size: 20, color: color),
                onPressed: onEdit,
                tooltip: 'Edit',
              ),
              IconButton(
                icon: Icon(
                  Icons.delete_outline,
                  size: 20,
                  color: Theme.of(context).colorScheme.error,
                ),
                onPressed: onDelete,
                tooltip: 'Delete',
              ),
              ReorderableDelayedDragStartListener(
                index: index,
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Icon(
                    Icons.drag_handle,
                    size: 24,
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
