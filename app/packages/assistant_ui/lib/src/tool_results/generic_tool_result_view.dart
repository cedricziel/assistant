import 'dart:convert';

import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';

/// The fallback tool details: the arguments as indented JSON and the result
/// as monospace text. Rendered when no `ToolResultRendererScope` resolver
/// supplies a richer view.
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
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        if (arguments != null) ...[
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
            _formatJson(arguments),
            style: TextStyle(
              fontSize: 10,
              fontFamily: 'monospace',
              color: colorScheme.onSurface,
            ),
          ),
        ],
        if (arguments != null && result != null)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 4),
            child: Divider(
              height: 1,
              thickness: 0.5,
              color: colorScheme.outlineVariant.withAlpha(80),
            ),
          ),
        if (result != null) ...[
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
      ],
    );
  }
}
