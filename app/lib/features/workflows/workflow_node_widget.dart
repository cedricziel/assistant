import 'package:material_ui/material_ui.dart';

import 'workflow_editor_model.dart';

// ---------------------------------------------------------------------------
// Node widget (canvas)

class NodeWidget extends StatelessWidget {
  const NodeWidget({
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
      child: Opacity(
        opacity: node.disabled ? 0.45 : 1.0,
        child: GestureDetector(
          onPanUpdate: (details) => onDrag(details.delta),
          onTap: onTap,
          child: SizedBox(
            width: kEditorNodeW,
            height: kEditorNodeH,
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
                              color: Theme.of(context)
                                  .colorScheme
                                  .onSurfaceVariant,
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
                    top: -15,
                    right: 17,
                    child: GestureDetector(
                      behavior: HitTestBehavior.opaque,
                      onTap: onEdit,
                      child: SizedBox(
                        width: 36,
                        height: 36,
                        child: Center(
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
                    ),
                  ),

                  // Delete button (top-right, offset)
                  Positioned(
                    top: -15,
                    right: -9,
                    child: GestureDetector(
                      behavior: HitTestBehavior.opaque,
                      onTap: onDelete,
                      child: SizedBox(
                        width: 36,
                        height: 36,
                        child: Center(
                          child: Container(
                            width: 22,
                            height: 22,
                            decoration: BoxDecoration(
                              color: Theme.of(context).colorScheme.error,
                              shape: BoxShape.circle,
                              boxShadow: [
                                BoxShadow(
                                  color: Theme.of(context).colorScheme.error
                                      .withValues(alpha: 0.3),
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
                    ),
                  ),

                  // Connect button (bottom-center)
                  Positioned(
                    bottom: -17,
                    left: kEditorNodeW / 2 - 18,
                    child: GestureDetector(
                      behavior: HitTestBehavior.opaque,
                      onTap: onStartConnect,
                      child: SizedBox(
                        width: 36,
                        height: 36,
                        child: Center(
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
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}
