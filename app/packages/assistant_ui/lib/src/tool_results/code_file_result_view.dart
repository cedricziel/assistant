import 'package:material_ui/material_ui.dart';

import '../code/code_block.dart';
import '../models/chat_message.dart';
import '../theme/assistant_ui_spacing.dart';
import 'tool_result_renderer.dart';

/// The `code_file` contract: the contents of a file that was read, possibly
/// cut. See `schemas/tool-results/code_file.schema.json`.
class CodeFileData {
  const CodeFileData({
    required this.path,
    required this.content,
    required this.truncated,
    required this.totalChars,
    this.language,
  });

  final String path;

  /// Language id hint for the highlighter, such as `rust`; null when unknown.
  final String? language;

  final String content;

  /// Whether [content] is shorter than the file.
  final bool truncated;

  /// Character count of the whole file.
  final int totalChars;

  /// Lines in [content]; a trailing newline does not start another line.
  int get lineCount {
    if (content.isEmpty) return 0;
    final breaks = '\n'.allMatches(content).length;
    return content.endsWith('\n') ? breaks : breaks + 1;
  }
}

/// Resolves `kind: "code_file"` payloads to [CodeFileResultView].
class CodeFileRenderer extends ToolResultRenderer<CodeFileData> {
  const CodeFileRenderer();

  @override
  String get kind => 'code_file';

  @override
  CodeFileData? parse(Map<String, Object?> data) {
    final path = data['path'];
    final language = data['language'];
    final content = data['content'];
    final truncated = data['truncated'];
    final totalChars = data['total_chars'];
    if (data['kind'] != kind ||
        path is! String ||
        (language != null && language is! String) ||
        content is! String ||
        truncated is! bool ||
        totalChars is! int ||
        totalChars < 0) {
      return null;
    }
    return CodeFileData(
      path: path,
      language: language as String?,
      content: content,
      truncated: truncated,
      totalChars: totalChars,
    );
  }

  @override
  Widget summary(BuildContext context, CodeFileData model) {
    final lines = model.lineCount;
    final count = '$lines ${lines == 1 ? 'line' : 'lines'}';
    return Text(
      '${model.path} · $count${model.truncated ? ' · truncated' : ''}',
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: Theme.of(context).textTheme.bodySmall,
    );
  }

  @override
  Widget build(
    BuildContext context,
    CodeFileData model,
    ToolCallStatus status,
  ) => CodeFileResultView(data: model);
}

/// A file's path above its contents in a [CodeBlock], with a notice when the
/// contents were cut.
class CodeFileResultView extends StatelessWidget {
  const CodeFileResultView({super.key, required this.data});

  final CodeFileData data;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Icon(
              Icons.description_outlined,
              size: 16,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: AssistantUiSpacing.xs),
            Expanded(
              child: Text(
                data.path,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.labelLarge,
              ),
            ),
          ],
        ),
        const SizedBox(height: AssistantUiSpacing.sm),
        CodeBlock(code: data.content, language: data.language),
        if (data.truncated) ...[
          const SizedBox(height: AssistantUiSpacing.xs),
          Text(
            'Truncated: showing ${data.content.length} of ${data.totalChars} '
            'characters',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ],
    );
  }
}
