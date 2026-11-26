import 'dart:convert';

import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';
import 'json_tree_view.dart';

/// The fallback tool details: the arguments as indented JSON, the result as
/// monospace text, and, when the record carries structured `data`, that data
/// as a collapsible [JsonTreeView]. Rendered when no `ToolResultRendererScope`
/// resolver supplies a richer view.
class GenericToolResultView extends StatelessWidget {
  const GenericToolResultView({super.key, required this.record});

  final ToolCallRecord record;

  static String _formatJson(Map<String, dynamic> json) {
    try {
      return const JsonEncoder.withIndent('  ').convert(json);
    } catch (_) {
      return json.toString();
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final arguments = record.arguments;
    final result = record.result;
    final data = record.data;
    Widget label(String text) => Text(
      text,
      style: TextStyle(
        fontSize: 10,
        fontWeight: FontWeight.w600,
        color: colorScheme.onSurfaceVariant,
      ),
    );
    final divider = Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Divider(
        height: 1,
        thickness: 0.5,
        color: colorScheme.outlineVariant.withAlpha(80),
      ),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        if (arguments != null) ...[
          label('Arguments'),
          const SizedBox(height: 2),
          SelectableText(
            _formatJson(arguments),
            style: TextStyle(
              fontSize: 10,
              fontFamily: 'monospace',
              color: colorScheme.onSurface,
            ),
          ),
        ],
        if (arguments != null && result != null) divider,
        if (result != null) ...[
          label('Result'),
          const SizedBox(height: 2),
          SelectableText(
            result,
            style: TextStyle(
              fontSize: 10,
              fontFamily: 'monospace',
              color: record.status == ToolCallStatus.error
                  ? colorScheme.error
                  : colorScheme.onSurface,
            ),
          ),
        ],
        if (data != null) ...[
          if (arguments != null || result != null) divider,
          label('Data'),
          const SizedBox(height: 2),
          JsonTreeView(data: data),
        ],
      ],
    );
  }
}
