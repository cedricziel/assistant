import 'dart:convert';

import 'package:flutter/material.dart';

import 'chat_provider.dart';

/// Inline chip displayed inside an assistant message bubble for a single tool
/// invocation.  Shows a spinner while pending and a status icon once resolved.
/// Tapping the chip expands it to show the tool's arguments and result.
class ToolCallChip extends StatefulWidget {
  const ToolCallChip({super.key, required this.record});

  final ToolCallRecord record;

  @override
  State<ToolCallChip> createState() => _ToolCallChipState();
}

class _ToolCallChipState extends State<ToolCallChip> {
  bool _expanded = false;

  bool get _hasDetails =>
      widget.record.arguments != null || widget.record.result != null;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    final (Widget icon, Color color) = switch (widget.record.status) {
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

    final chip = GestureDetector(
      onTap: _hasDetails ? () => setState(() => _expanded = !_expanded) : null,
      child: Container(
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
              widget.record.toolName,
              style: TextStyle(
                fontSize: 11,
                color: color.withAlpha(220),
                fontWeight: FontWeight.w500,
              ),
            ),
            if (_hasDetails) ...[
              const SizedBox(width: 3),
              Icon(
                _expanded
                    ? Icons.expand_less_rounded
                    : Icons.expand_more_rounded,
                size: 14,
                color: color.withAlpha(150),
              ),
            ],
          ],
        ),
      ),
    );

    if (!_expanded) return chip;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        chip,
        const SizedBox(height: 4),
        Container(
          constraints: const BoxConstraints(maxWidth: 560),
          padding: const EdgeInsets.all(8),
          decoration: BoxDecoration(
            color: colorScheme.surfaceContainerLowest,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: colorScheme.outlineVariant.withAlpha(80),
              width: 0.5,
            ),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              if (widget.record.arguments != null) ...[
                Text(
                  'Arguments',
                  style: TextStyle(
                    fontSize: 10,
                    fontWeight: FontWeight.w600,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 2),
                SelectableText(
                  _formatJson(widget.record.arguments!),
                  style: TextStyle(
                    fontSize: 10,
                    fontFamily: 'monospace',
                    color: colorScheme.onSurface,
                  ),
                ),
              ],
              if (widget.record.arguments != null &&
                  widget.record.result != null)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  child: Divider(
                    height: 1,
                    thickness: 0.5,
                    color: colorScheme.outlineVariant.withAlpha(80),
                  ),
                ),
              if (widget.record.result != null) ...[
                Text(
                  'Result',
                  style: TextStyle(
                    fontSize: 10,
                    fontWeight: FontWeight.w600,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 2),
                SelectableText(
                  widget.record.result!,
                  style: TextStyle(
                    fontSize: 10,
                    fontFamily: 'monospace',
                    color: colorScheme.onSurface,
                  ),
                ),
              ],
            ],
          ),
        ),
      ],
    );
  }

  String _formatJson(Map<String, dynamic> json) {
    try {
      return const JsonEncoder.withIndent('  ').convert(json);
    } catch (_) {
      return json.toString();
    }
  }
}
