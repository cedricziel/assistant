import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';
import '../theme/assistant_ui_spacing.dart';
import 'tool_result_renderer.dart';

/// What a `code_diff` payload says happened to the file.
enum DiffOperation {
  edit('Edited'),
  create('Created'),
  overwrite('Overwritten');

  const DiffOperation(this.label);

  /// Past-tense label shown beside the path.
  final String label;
}

/// How a diff line relates to the file before and after the change.
enum DiffOp { context, add, remove }

class DiffLine {
  const DiffLine({required this.op, required this.text});

  final DiffOp op;
  final String text;
}

class DiffHunk {
  const DiffHunk({
    required this.oldStart,
    required this.oldLines,
    required this.newStart,
    required this.newLines,
    required this.lines,
  });

  final int oldStart;
  final int oldLines;
  final int newStart;
  final int newLines;
  final List<DiffLine> lines;

  /// The unified-diff header, such as `@@ -3,3 +3,3 @@`.
  String get header => '@@ -$oldStart,$oldLines +$newStart,$newLines @@';
}

/// The `code_diff` contract: a unified-diff style change to one file. See
/// `schemas/tool-results/code_diff.schema.json`.
class CodeDiffData {
  const CodeDiffData({
    required this.path,
    required this.operation,
    required this.hunks,
    required this.added,
    required this.removed,
    required this.truncated,
    this.language,
  });

  final String path;
  final String? language;
  final DiffOperation operation;
  final List<DiffHunk> hunks;
  final int added;
  final int removed;

  /// Whether the producer cut hunks or lines to fit its budget.
  final bool truncated;

  /// `+added −removed`, with a true minus sign.
  String get counts => '+$added −$removed';
}

/// Longest diff line the contract allows, in characters.
const _maxLineChars = 512;

int? _count(Object? value) => value is int && value >= 0 ? value : null;

DiffLine? _parseLine(Object? raw) {
  if (raw is! Map<String, Object?>) return null;
  final text = raw['text'];
  final op = DiffOp.values.asNameMap()[raw['op']];
  if (op == null || text is! String || text.runes.length > _maxLineChars) {
    return null;
  }
  return DiffLine(op: op, text: text);
}

DiffHunk? _parseHunk(Object? raw) {
  if (raw is! Map<String, Object?>) return null;
  final rawLines = raw['lines'];
  final oldStart = _count(raw['old_start']);
  final oldLines = _count(raw['old_lines']);
  final newStart = _count(raw['new_start']);
  final newLines = _count(raw['new_lines']);
  if (rawLines is! List<Object?> ||
      oldStart == null ||
      oldLines == null ||
      newStart == null ||
      newLines == null) {
    return null;
  }
  final lines = <DiffLine>[];
  for (final rawLine in rawLines) {
    final line = _parseLine(rawLine);
    if (line == null) return null;
    lines.add(line);
  }
  return DiffHunk(
    oldStart: oldStart,
    oldLines: oldLines,
    newStart: newStart,
    newLines: newLines,
    lines: lines,
  );
}

/// Resolves `kind: "code_diff"` payloads to [CodeDiffResultView].
class CodeDiffRenderer extends ToolResultRenderer<CodeDiffData> {
  const CodeDiffRenderer();

  @override
  String get kind => 'code_diff';

  @override
  CodeDiffData? parse(Map<String, Object?> data) {
    final path = data['path'];
    final language = data['language'];
    final operation = DiffOperation.values.asNameMap()[data['operation']];
    final rawHunks = data['hunks'];
    final added = _count(data['added']);
    final removed = _count(data['removed']);
    final truncated = data['truncated'];
    if (data['kind'] != kind ||
        path is! String ||
        (language != null && language is! String) ||
        operation == null ||
        rawHunks is! List<Object?> ||
        added == null ||
        removed == null ||
        truncated is! bool) {
      return null;
    }
    final hunks = <DiffHunk>[];
    for (final rawHunk in rawHunks) {
      final hunk = _parseHunk(rawHunk);
      if (hunk == null) return null;
      hunks.add(hunk);
    }
    return CodeDiffData(
      path: path,
      language: language as String?,
      operation: operation,
      hunks: hunks,
      added: added,
      removed: removed,
      truncated: truncated,
    );
  }

  @override
  Widget summary(BuildContext context, CodeDiffData model) => Text(
    '${model.operation.label} ${model.path} · ${model.counts}',
    maxLines: 1,
    overflow: TextOverflow.ellipsis,
    style: Theme.of(context).textTheme.bodySmall,
  );

  @override
  Widget build(
    BuildContext context,
    CodeDiffData model,
    ToolCallStatus status,
  ) => CodeDiffResultView(data: model);
}

/// A file's path, operation and `+added −removed` counts above its hunks.
///
/// Each line carries a `+` or `−` gutter glyph next to its tint, so the
/// change reads without relying on colour.
class CodeDiffResultView extends StatelessWidget {
  const CodeDiffResultView({super.key, required this.data});

  final CodeDiffData data;

  static const _monospace = TextStyle(
    fontFamily: 'monospace',
    fontFamilyFallback: ['Menlo', 'Consolas', 'Roboto Mono', 'Courier New'],
    fontSize: 13,
    height: 1.5,
  );

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Icon(
              Icons.difference_outlined,
              size: 16,
              color: scheme.onSurfaceVariant,
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
            const SizedBox(width: AssistantUiSpacing.sm),
            Text(
              data.operation.label,
              style: theme.textTheme.labelMedium?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(width: AssistantUiSpacing.sm),
            Text.rich(
              TextSpan(
                children: [
                  TextSpan(
                    text: '+${data.added}',
                    style: TextStyle(color: scheme.tertiary),
                  ),
                  const TextSpan(text: ' '),
                  TextSpan(
                    text: '−${data.removed}',
                    style: TextStyle(color: scheme.error),
                  ),
                ],
              ),
              style: theme.textTheme.labelMedium,
            ),
          ],
        ),
        const SizedBox(height: AssistantUiSpacing.sm),
        if (data.hunks.isEmpty)
          Text(
            'No changes',
            style: theme.textTheme.bodySmall?.copyWith(
              color: scheme.onSurfaceVariant,
            ),
          )
        else
          _Hunks(hunks: data.hunks),
        if (data.truncated) ...[
          const SizedBox(height: AssistantUiSpacing.xs),
          Text(
            'Diff truncated',
            style: theme.textTheme.bodySmall?.copyWith(
              color: scheme.onSurfaceVariant,
            ),
          ),
        ],
      ],
    );
  }
}

class _Hunks extends StatelessWidget {
  const _Hunks({required this.hunks});

  final List<DiffHunk> hunks;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return ClipRRect(
      borderRadius: BorderRadius.circular(8),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: scheme.surfaceContainerLow,
          border: Border.all(color: scheme.outlineVariant),
          borderRadius: BorderRadius.circular(8),
        ),
        child: LayoutBuilder(
          builder: (context, constraints) => SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: ConstrainedBox(
              constraints: BoxConstraints(minWidth: constraints.maxWidth),
              child: IntrinsicWidth(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    for (final hunk in hunks) ...[
                      _HunkHeader(hunk: hunk),
                      for (final line in hunk.lines) _Line(line: line),
                    ],
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _HunkHeader extends StatelessWidget {
  const _HunkHeader({required this.hunk});

  final DiffHunk hunk;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      color: scheme.surfaceContainerHighest,
      padding: const EdgeInsets.symmetric(
        horizontal: AssistantUiSpacing.md,
        vertical: AssistantUiSpacing.xs,
      ),
      child: Text(
        hunk.header,
        softWrap: false,
        style: CodeDiffResultView._monospace.copyWith(
          fontSize: 12,
          color: scheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _Line extends StatelessWidget {
  const _Line({required this.line});

  final DiffLine line;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final (glyph, label, tint) = switch (line.op) {
      DiffOp.add => ('+', 'added', scheme.tertiary),
      DiffOp.remove => ('−', 'removed', scheme.error),
      DiffOp.context => (' ', 'unchanged', null),
    };
    return Container(
      color: tint?.withAlpha(36),
      padding: const EdgeInsets.symmetric(horizontal: AssistantUiSpacing.md),
      child: Row(
        children: [
          SelectionContainer.disabled(
            child: SizedBox(
              width: 20,
              child: Text(
                glyph,
                semanticsLabel: label,
                style: CodeDiffResultView._monospace.copyWith(color: tint),
              ),
            ),
          ),
          Text(
            line.text,
            softWrap: false,
            style: CodeDiffResultView._monospace.copyWith(
              color: scheme.onSurface,
            ),
          ),
        ],
      ),
    );
  }
}
