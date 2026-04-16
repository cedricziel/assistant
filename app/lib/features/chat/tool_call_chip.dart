import 'package:flutter/material.dart';

import 'chat_provider.dart';

/// Inline chip displayed inside an assistant message bubble for a single tool
/// invocation.  Shows a spinner while pending and a status icon once resolved.
class ToolCallChip extends StatelessWidget {
  const ToolCallChip({super.key, required this.record});

  final ToolCallRecord record;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    final (Widget icon, Color color) = switch (record.status) {
      ToolCallStatus.pending => (
        SizedBox(
          width: 12,
          height: 12,
          child: CircularProgressIndicator(
            strokeWidth: 1.5,
            color: colorScheme.primary,
          ),
        ),
        colorScheme.primary,
      ),
      ToolCallStatus.ok => (
        const Icon(Icons.check_circle_outline, size: 14, color: Colors.green),
        Colors.green,
      ),
      ToolCallStatus.error => (
        const Icon(Icons.error_outline, size: 14, color: Colors.red),
        Colors.red,
      ),
      ToolCallStatus.denied => (
        const Icon(Icons.block, size: 14, color: Colors.amber),
        Colors.amber,
      ),
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withAlpha(20),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withAlpha(60), width: 0.5),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          icon,
          const SizedBox(width: 5),
          Text(
            record.toolName,
            style: TextStyle(
              fontSize: 11,
              color: color.withAlpha(220),
              fontWeight: FontWeight.w500,
            ),
          ),
        ],
      ),
    );
  }
}
