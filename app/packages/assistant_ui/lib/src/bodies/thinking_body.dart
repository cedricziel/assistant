import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart'
    hide ToolCallStatus;
import 'package:material_ui/material_ui.dart';

import '../models/timeline_state.dart';
import '../scopes/chat_markdown_scope.dart';
import '../scopes/thread_density_scope.dart';
import '../theme/legacy_theme.dart';
import '../timeline/timeline_entry_shell.dart';

/// One collapsible reasoning section: "Thinking..." while it streams, then
/// "Thought" (or "Thought for 4s" when [elapsed] is known).
///
/// While [entryState] is active and a [tokenStream] is supplied, the stream
/// renders through `StreamMarkdown`, with links delegated to
/// [ChatMarkdownScope] and no parser plugins or builders. Otherwise the
/// accumulated [content] renders as plain selectable text, not markdown. The
/// section is expandable only when there is something to show. The runtime
/// sends reasoning without step boundaries, so the body is one continuous
/// block.
class ThinkingBody extends StatelessWidget {
  const ThinkingBody({
    super.key,
    this.content,
    this.tokenStream,
    this.entryState = EntryState.complete,
    this.elapsed,
  });

  /// The accumulated reasoning text.
  final String? content;

  /// Live reasoning chunks; rendered only while [entryState] is active.
  final Stream<String>? tokenStream;
  final EntryState entryState;

  /// How long the model thought. Shown only when non-null.
  final Duration? elapsed;

  double _maxHeight(BuildContext context) =>
      switch (ThreadDensityScope.of(context)) {
        TimelineDensity.compact => 120,
        TimelineDensity.normal => 150,
        TimelineDensity.expanded => 200,
      };

  String get _label {
    if (entryState == EntryState.active) return 'Thinking...';
    final elapsed = this.elapsed;
    if (elapsed == null) return 'Thought';
    return 'Thought for ${_formatElapsed(elapsed)}';
  }

  static String _formatElapsed(Duration d) {
    if (d.inSeconds >= 60) return '${d.inMinutes}m ${d.inSeconds % 60}s';
    return d.inSeconds < 1 ? '<1s' : '${d.inSeconds}s';
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final content = this.content;
    final stream = tokenStream;
    final isActive = entryState == EntryState.active;
    final hasContent = content != null && content.isNotEmpty;
    final isStreaming = isActive && stream != null;
    final textStyle = TextStyle(
      fontSize: 12,
      color: colorScheme.onSurfaceVariant,
      fontStyle: FontStyle.italic,
    );

    Widget? body;
    if (isStreaming) {
      body = TimelineEntryShell.fadeContainer(
        maxHeight: _maxHeight(context),
        isActive: true,
        child: StreamMarkdown(
          stream: stream,
          styleSheet: MarkdownStyleSheet.fromTheme(legacyThemeOf(context))
              .copyWith(paragraphStyle: textStyle),
          imageBuilder: ChatMarkdownScope.imageBuilderOf(context),
          onTapLink: ChatMarkdownScope.of(context)?.onTapLink,
        ),
      );
    } else if (hasContent) {
      body = TimelineEntryShell.fadeContainer(
        maxHeight: _maxHeight(context),
        isActive: isActive,
        child: SelectableText(content, style: textStyle),
      );
    }

    return TimelineEntryShell(
      icon: Icons.psychology_outlined,
      label: _label,
      status: isActive
          ? const SizedBox(
              width: 12,
              height: 12,
              child: CircularProgressIndicator.adaptive(strokeWidth: 1.5),
            )
          : null,
      entryState: entryState,
      expandable: hasContent || isStreaming,
      body: body,
    );
  }
}
