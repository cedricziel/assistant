import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart'
    hide ToolCallStatus;

import 'chat_provider.dart';

/// Display density for timeline entries, derived from viewport width.
enum TimelineDensity {
  /// < 400px: minimal detail, icon + name only for most entries.
  compact,

  /// 400–700px: standard detail level.
  normal,

  /// > 700px: full detail including arguments preview.
  expanded;

  /// Derive density from the current viewport width.
  static TimelineDensity fromWidth(double width) {
    if (width < 400) return TimelineDensity.compact;
    if (width <= 700) return TimelineDensity.normal;
    return TimelineDensity.expanded;
  }
}

/// Lifecycle state of a timeline entry during streaming.
enum EntryState {
  /// Currently being produced (streaming thinking, tool running, etc).
  active,

  /// Finished normally — will auto-collapse after a short delay.
  complete,

  /// A newer entry superseded this one — stays collapsed, reduced opacity.
  stale,
}

/// A streaming-aware timeline entry that replaces [ChatTimelineSection].
///
/// Adapts its presentation based on [density], [entryState], and whether
/// the user has manually pinned it open.
class StreamingTimelineEntry extends StatefulWidget {
  const StreamingTimelineEntry({
    super.key,
    required this.message,
    required this.density,
    this.entryState = EntryState.complete,
  });

  final ChatMessage message;
  final TimelineDensity density;
  final EntryState entryState;

  @override
  State<StreamingTimelineEntry> createState() => _StreamingTimelineEntryState();
}

class _StreamingTimelineEntryState extends State<StreamingTimelineEntry> {
  bool _expanded = false;
  bool _userPinned = false;
  Timer? _collapseTimer;

  @override
  void initState() {
    super.initState();
    // Auto-expand if starting in active state (non-compact).
    if (widget.entryState == EntryState.active &&
        widget.density != TimelineDensity.compact) {
      _expanded = true;
    }
  }

  @override
  void didUpdateWidget(covariant StreamingTimelineEntry oldWidget) {
    super.didUpdateWidget(oldWidget);

    // Auto-expand when entry becomes active.
    if (widget.entryState == EntryState.active &&
        oldWidget.entryState != EntryState.active) {
      if (!_userPinned && widget.density != TimelineDensity.compact) {
        setState(() => _expanded = true);
      }
    }

    // Auto-collapse when entry completes (with delay).
    if (widget.entryState == EntryState.complete &&
        oldWidget.entryState == EntryState.active) {
      _scheduleAutoCollapse();
    }

    // Stale entries collapse immediately.
    if (widget.entryState == EntryState.stale &&
        oldWidget.entryState != EntryState.stale) {
      if (!_userPinned) {
        setState(() => _expanded = false);
      }
    }
  }

  void _scheduleAutoCollapse() {
    _collapseTimer?.cancel();
    if (_userPinned) return;

    final reducedMotion = MediaQuery.of(context).disableAnimations;
    if (reducedMotion) {
      setState(() => _expanded = false);
      return;
    }

    _collapseTimer = Timer(const Duration(milliseconds: 500), () {
      if (mounted && !_userPinned) {
        setState(() => _expanded = false);
      }
    });
  }

  @override
  void dispose() {
    _collapseTimer?.cancel();
    super.dispose();
  }

  void _onTap() {
    setState(() {
      _expanded = !_expanded;
      _userPinned = _expanded;
    });
  }

  double _maxHeight(bool isSubagent) {
    if (isSubagent) {
      return switch (widget.density) {
        TimelineDensity.compact => 100,
        TimelineDensity.normal => 120,
        TimelineDensity.expanded => 150,
      };
    }
    return switch (widget.density) {
      TimelineDensity.compact => 120,
      TimelineDensity.normal => 150,
      TimelineDensity.expanded => 200,
    };
  }

  @override
  Widget build(BuildContext context) {
    final opacity = widget.entryState == EntryState.stale ? 0.6 : 1.0;

    return Opacity(
      opacity: opacity,
      child: switch (widget.message.timelineType) {
        TimelineEntryType.toolCall => _buildToolCall(context),
        TimelineEntryType.thinking => _buildThinking(context),
        TimelineEntryType.subagent => _buildSubagent(context),
        TimelineEntryType.command => const SizedBox.shrink(),
        TimelineEntryType.message => const SizedBox.shrink(),
      },
    );
  }

  // -- Thinking ---------------------------------------------------------------

  Widget _buildThinking(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final content = widget.message.thinkingContent;
    final isActive = widget.entryState == EntryState.active;
    final stream = widget.message.thinkingTokenStream;
    final hasContent = content != null && content.isNotEmpty;

    // Build the expanded content based on state.
    Widget? expandedContent;
    if (isActive && stream != null) {
      // Live streaming thinking — render with StreamMarkdown.
      expandedContent = _fadeContainer(
        maxHeight: _maxHeight(false),
        isActive: true,
        child: StreamMarkdown(
          stream: stream,
          styleSheet: MarkdownStyleSheet.fromTheme(Theme.of(context)).copyWith(
            paragraphStyle: TextStyle(
              fontSize: 12,
              color: colorScheme.onSurfaceVariant,
              fontStyle: FontStyle.italic,
            ),
          ),
        ),
      );
    } else if (hasContent) {
      // Accumulated thinking content (complete or active without stream).
      expandedContent = _fadeContainer(
        maxHeight: _maxHeight(false),
        isActive: isActive,
        child: SelectableText(
          content,
          style: TextStyle(
            fontSize: 12,
            color: colorScheme.onSurfaceVariant,
            fontStyle: FontStyle.italic,
          ),
        ),
      );
    }

    final expandable = hasContent || (isActive && stream != null);

    return _buildSection(
      context,
      icon: Icons.psychology_outlined,
      label: isActive ? 'Thinking...' : 'Thought',
      statusWidget: isActive
          ? const SizedBox(
              width: 12,
              height: 12,
              child: CircularProgressIndicator.adaptive(strokeWidth: 1.5),
            )
          : null,
      expandable: expandable,
      expandedContent: expandedContent,
    );
  }

  // -- Tool call --------------------------------------------------------------

  Widget _buildToolCall(BuildContext context) {
    final record = widget.message.toolCalls.isNotEmpty
        ? widget.message.toolCalls.first
        : null;
    if (record == null) return const SizedBox.shrink();

    final hasDetails = record.arguments != null || record.result != null;

    // Density adaptation:
    // compact: icon + name only (no args preview)
    // normal: icon + name + status
    // expanded: icon + name + args preview + status
    final showArgs = widget.density == TimelineDensity.expanded && hasDetails;

    // Duration badge for completed tool calls.
    final durationLabel = record.duration != null
        ? _formatDuration(record.duration!)
        : null;

    return _buildSection(
      context,
      icon: Icons.build_outlined,
      label: record.toolName,
      statusWidget: _toolStatusWidget(context, record, durationLabel),
      expandable: hasDetails,
      expandedContent: hasDetails ? _toolDetails(context, record) : null,
      forceExpand: record.status == ToolCallStatus.error,
      showInline: showArgs,
    );
  }

  String _formatDuration(Duration d) {
    if (d.inSeconds >= 60) {
      return '${d.inMinutes}m ${d.inSeconds % 60}s';
    }
    final ms = d.inMilliseconds;
    return '${(ms / 1000).toStringAsFixed(1)}s';
  }

  Widget _toolStatusWidget(
    BuildContext context,
    ToolCallRecord record,
    String? durationLabel,
  ) {
    final colorScheme = Theme.of(context).colorScheme;
    final icon = _toolStatusIcon(context, record.status);

    if (durationLabel == null || widget.density == TimelineDensity.compact) {
      return icon;
    }

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        icon,
        const SizedBox(width: 4),
        Text(
          durationLabel,
          style: TextStyle(
            fontSize: 10,
            color: colorScheme.onSurfaceVariant.withAlpha(180),
          ),
        ),
      ],
    );
  }

  Widget _toolStatusIcon(BuildContext context, ToolCallStatus status) {
    final colorScheme = Theme.of(context).colorScheme;
    return switch (status) {
      ToolCallStatus.pending => const SizedBox(
        width: 12,
        height: 12,
        child: CircularProgressIndicator.adaptive(strokeWidth: 1.5),
      ),
      ToolCallStatus.ok => const Icon(
        Icons.check_circle_outline,
        size: 14,
        color: Colors.green,
      ),
      ToolCallStatus.error => Icon(
        Icons.error_outline,
        size: 14,
        color: colorScheme.error,
      ),
      ToolCallStatus.denied => const Icon(
        Icons.block,
        size: 14,
        color: Colors.amber,
      ),
    };
  }

  Widget _toolDetails(BuildContext context, ToolCallRecord record) {
    final colorScheme = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        if (record.arguments != null) ...[
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
            _formatJson(record.arguments!),
            style: TextStyle(
              fontSize: 10,
              fontFamily: 'monospace',
              color: colorScheme.onSurface,
            ),
          ),
        ],
        if (record.arguments != null && record.result != null)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 4),
            child: Divider(
              height: 1,
              thickness: 0.5,
              color: colorScheme.outlineVariant.withAlpha(80),
            ),
          ),
        if (record.result != null) ...[
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
            record.result!,
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

  // -- Subagent ---------------------------------------------------------------

  Widget _buildSubagent(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final hasCompleted = widget.message.subagentSummary != null;
    final isActive = widget.entryState == EntryState.active;

    Widget? expandedContent;
    if (hasCompleted) {
      expandedContent = Text(
        widget.message.subagentSummary!,
        style: TextStyle(fontSize: 12, color: colorScheme.onSurfaceVariant),
      );
    } else if (isActive) {
      // Show inner timeline of subagent events.
      expandedContent = _buildSubagentInnerTimeline(context);
    }

    return _buildSection(
      context,
      icon: Icons.smart_toy_outlined,
      label: widget.message.subagentTask ?? widget.message.subagentId ?? '',
      statusWidget: hasCompleted
          ? const Icon(
              Icons.check_circle_outline,
              size: 14,
              color: Colors.green,
            )
          : const SizedBox(
              width: 12,
              height: 12,
              child: CircularProgressIndicator.adaptive(strokeWidth: 1.5),
            ),
      expandable: hasCompleted || isActive,
      expandedContent: expandedContent,
    );
  }

  Widget _buildSubagentInnerTimeline(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final msg = widget.message;
    final entries = <Widget>[];

    // Show subagent thinking if present.
    final thinking = msg.subagentThinking;
    if (thinking.isNotEmpty) {
      entries.add(
        Padding(
          padding: const EdgeInsets.only(bottom: 4),
          child: Row(
            children: [
              Icon(
                Icons.psychology_outlined,
                size: 12,
                color: colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 4),
              Expanded(
                child: Text(
                  thinking,
                  style: TextStyle(
                    fontSize: 11,
                    color: colorScheme.onSurfaceVariant,
                    fontStyle: FontStyle.italic,
                  ),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ),
      );
    }

    // Show subagent tool calls.
    // Density adaptation: normal shows last 2 entries, expanded shows all.
    var toolCalls = msg.subagentToolCalls;
    if (widget.density == TimelineDensity.normal && toolCalls.length > 2) {
      toolCalls = toolCalls.sublist(toolCalls.length - 2);
    }

    for (final tc in toolCalls) {
      entries.add(
        Padding(
          padding: const EdgeInsets.only(bottom: 2),
          child: Row(
            children: [
              Icon(
                Icons.build_outlined,
                size: 12,
                color: colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 4),
              Text(
                tc.toolName,
                style: TextStyle(
                  fontSize: 11,
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 4),
              _toolStatusIcon(context, tc.status),
            ],
          ),
        ),
      );
    }

    if (entries.isEmpty) {
      return Text(
        'Processing...',
        style: TextStyle(
          fontSize: 11,
          color: colorScheme.onSurfaceVariant,
          fontStyle: FontStyle.italic,
        ),
      );
    }

    return _fadeContainer(
      maxHeight: _maxHeight(true),
      isActive: true,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: entries,
      ),
    );
  }

  // -- Shared layout ----------------------------------------------------------

  Widget _fadeContainer({
    required double maxHeight,
    required bool isActive,
    required Widget child,
  }) {
    return ConstrainedBox(
      constraints: BoxConstraints(maxHeight: maxHeight),
      child: ShaderMask(
        shaderCallback: (bounds) => LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          // Alpha mask values — not visible colors (used for blending).
          colors: const [
            Color(0xFFFFFFFF),
            Color(0xFFFFFFFF),
            Colors.transparent,
          ],
          stops: const [0.0, 0.8, 1.0],
        ).createShader(bounds),
        blendMode: BlendMode.dstIn,
        child: SingleChildScrollView(reverse: isActive, child: child),
      ),
    );
  }

  Widget _buildSection(
    BuildContext context, {
    required IconData icon,
    required String label,
    Widget? statusWidget,
    bool expandable = false,
    Widget? expandedContent,
    bool forceExpand = false,
    bool showInline = false,
  }) {
    final colorScheme = Theme.of(context).colorScheme;
    final isExpanded = forceExpand || _expanded;
    final reducedMotion = MediaQuery.of(context).disableAnimations;

    // In compact density, only show icon + name.
    final showStatus =
        widget.density != TimelineDensity.compact || statusWidget is! SizedBox;

    return Semantics(
      label: label,
      button: expandable,
      expanded: isExpanded,
      child: Align(
        alignment: Alignment.centerLeft,
        child: Container(
          margin: const EdgeInsets.symmetric(vertical: 2),
          constraints: const BoxConstraints(maxWidth: 640),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              GestureDetector(
                onTap: expandable ? _onTap : null,
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 6,
                  ),
                  decoration: BoxDecoration(
                    color: colorScheme.surfaceContainerHighest.withAlpha(120),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(icon, size: 14, color: colorScheme.onSurfaceVariant),
                      const SizedBox(width: 6),
                      Flexible(
                        child: Text(
                          label,
                          style: TextStyle(
                            fontSize: 12,
                            color: colorScheme.onSurfaceVariant,
                            fontWeight: FontWeight.w500,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      if (showStatus && statusWidget != null) ...[
                        const SizedBox(width: 6),
                        statusWidget,
                      ],
                      if (expandable) ...[
                        const SizedBox(width: 4),
                        Icon(
                          isExpanded
                              ? Icons.expand_less_rounded
                              : Icons.expand_more_rounded,
                          size: 14,
                          color: colorScheme.onSurfaceVariant.withAlpha(150),
                        ),
                      ],
                    ],
                  ),
                ),
              ),
              AnimatedSize(
                duration: reducedMotion
                    ? Duration.zero
                    : const Duration(milliseconds: 200),
                curve: Curves.easeInOut,
                child: isExpanded && expandedContent != null
                    ? Padding(
                        padding: const EdgeInsets.only(
                          left: 12,
                          top: 4,
                          bottom: 4,
                        ),
                        child: Container(
                          padding: const EdgeInsets.all(8),
                          decoration: BoxDecoration(
                            color: colorScheme.surfaceContainerLowest,
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(
                              color: colorScheme.outlineVariant.withAlpha(80),
                              width: 0.5,
                            ),
                          ),
                          child: expandedContent,
                        ),
                      )
                    : const SizedBox.shrink(),
              ),
            ],
          ),
        ),
      ),
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
