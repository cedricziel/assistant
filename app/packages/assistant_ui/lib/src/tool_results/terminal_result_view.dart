import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';
import '../theme/assistant_ui_spacing.dart';
import 'tool_result_renderer.dart';

/// The `terminal` contract: a finished shell command. See
/// `schemas/tool-results/terminal.schema.json`.
class TerminalData {
  const TerminalData({
    required this.command,
    required this.exitCode,
    required this.stdout,
    required this.stderr,
    required this.stdoutTruncated,
    required this.stderrTruncated,
    required this.durationMs,
    required this.timedOut,
  });

  final String command;

  /// Null when the command timed out or never started.
  final int? exitCode;

  /// The render tail of each stream, cut by the producer.
  final String stdout;
  final String stderr;

  /// Whether the producer cut the matching stream.
  final bool stdoutTruncated;
  final bool stderrTruncated;

  final int durationMs;

  /// Whether the command was killed by its timeout.
  final bool timedOut;

  bool get succeeded => exitCode == 0;

  int get stdoutLines => _lineCount(stdout);
  int get stderrLines => _lineCount(stderr);

  /// The one-line form: `exit 2 · 14 lines stderr`, `timed out · 1 line
  /// stdout`.
  String get summaryText {
    final label = timedOut ? 'timed out' : 'exit $exitCode';
    if (stderrLines > 0) return '$label · ${_lines(stderrLines)} stderr';
    return '$label · ${_lines(stdoutLines)} stdout';
  }
}

int _lineCount(String text) {
  if (text.isEmpty) return 0;
  final breaks = '\n'.allMatches(text).length;
  return text.endsWith('\n') ? breaks : breaks + 1;
}

String _lines(int count) => '$count ${count == 1 ? 'line' : 'lines'}';

/// Resolves `kind: "terminal"` payloads to [TerminalResultView].
class TerminalRenderer extends ToolResultRenderer<TerminalData> {
  const TerminalRenderer();

  @override
  String get kind => 'terminal';

  @override
  TerminalData? parse(Map<String, Object?> data) {
    final command = data['command'];
    final exitCode = data['exit_code'];
    final stdout = data['stdout'];
    final stderr = data['stderr'];
    final stdoutTruncated = data['stdout_truncated'];
    final stderrTruncated = data['stderr_truncated'];
    final durationMs = data['duration_ms'];
    final timedOut = data['timed_out'];
    if (data['kind'] != kind ||
        command is! String ||
        (exitCode != null && exitCode is! int) ||
        stdout is! String ||
        stderr is! String ||
        stdoutTruncated is! bool ||
        stderrTruncated is! bool ||
        durationMs is! int ||
        durationMs < 0 ||
        timedOut is! bool) {
      return null;
    }
    return TerminalData(
      command: command,
      exitCode: exitCode as int?,
      stdout: stdout,
      stderr: stderr,
      stdoutTruncated: stdoutTruncated,
      stderrTruncated: stderrTruncated,
      durationMs: durationMs,
      timedOut: timedOut,
    );
  }

  @override
  Widget summary(BuildContext context, TerminalData model) => Text(
    model.summaryText,
    maxLines: 1,
    overflow: TextOverflow.ellipsis,
    style: Theme.of(context).textTheme.bodySmall,
  );

  @override
  Widget build(
    BuildContext context,
    TerminalData model,
    ToolCallStatus status,
  ) => TerminalResultView(data: model);
}

/// A finished shell command: the `$ command` line, an exit / failure / timeout
/// badge with the duration, then the stdout and stderr tails.
///
/// stderr is drawn in the error colour and each stream reports its own
/// truncation, so a cut tail never reads as the whole output.
class TerminalResultView extends StatelessWidget {
  const TerminalResultView({super.key, required this.data});

  final TerminalData data;

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
    final bad = data.timedOut || !data.succeeded;
    final badgeColor = bad ? scheme.error : scheme.tertiary;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Icon(Icons.terminal, size: 16, color: scheme.onSurfaceVariant),
            const SizedBox(width: AssistantUiSpacing.xs),
            Expanded(
              child: Text(
                '\$ ${data.command}',
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.labelLarge?.merge(_monospace),
              ),
            ),
          ],
        ),
        const SizedBox(height: AssistantUiSpacing.xs),
        Row(
          children: [
            _Badge(
              label: data.timedOut ? 'timed out' : 'exit ${data.exitCode}',
              color: badgeColor,
            ),
            const SizedBox(width: AssistantUiSpacing.sm),
            Text(
              _formatDuration(data.durationMs),
              style: theme.textTheme.labelSmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
        if (data.stdout.isNotEmpty)
          _Stream(
            text: data.stdout,
            truncated: data.stdoutTruncated,
            notice: 'stdout truncated',
            color: scheme.onSurface,
          ),
        if (data.stderr.isNotEmpty)
          _Stream(
            text: data.stderr,
            truncated: data.stderrTruncated,
            notice: 'stderr truncated',
            color: scheme.error,
          ),
      ],
    );
  }
}

String _formatDuration(int ms) {
  if (ms < 1000) return '${ms}ms';
  if (ms < 60000) return '${(ms / 1000).toStringAsFixed(1)}s';
  return '${(ms / 60000).toStringAsFixed(1)}m';
}

/// One captured stream, its trailing newlines dropped so the panel has no
/// blank last line.
String _display(String text) => text.replaceFirst(RegExp(r'\n+$'), '');

class _Stream extends StatelessWidget {
  const _Stream({
    required this.text,
    required this.truncated,
    required this.notice,
    required this.color,
  });

  final String text;
  final bool truncated;
  final String notice;
  final Color color;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Padding(
      padding: const EdgeInsets.only(top: AssistantUiSpacing.sm),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          DecoratedBox(
            decoration: BoxDecoration(
              color: scheme.surfaceContainerLow,
              border: Border.all(color: scheme.outlineVariant),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Padding(
              padding: const EdgeInsets.all(AssistantUiSpacing.md),
              child: Text(
                _display(text),
                style: TerminalResultView._monospace.copyWith(color: color),
              ),
            ),
          ),
          if (truncated)
            Padding(
              padding: const EdgeInsets.only(top: AssistantUiSpacing.xs),
              child: Text(
                notice,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: scheme.onSurfaceVariant,
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _Badge extends StatelessWidget {
  const _Badge({required this.label, required this.color});

  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: color.withAlpha(32),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        child: Text(
          label,
          style: Theme.of(context).textTheme.labelSmall?.copyWith(color: color),
        ),
      ),
    );
  }
}
