import 'dart:convert';

import 'package:flutter/material.dart';

import 'chat_provider.dart';

/// An expandable timeline entry widget for tool calls, thinking, and subagent
/// events. Shows a collapsed header with icon + label + status, and expands
/// to reveal full details.
class TimelineSection extends StatefulWidget {
  const TimelineSection({super.key, required this.message});

  final ChatMessage message;

  @override
  State<TimelineSection> createState() => _TimelineSectionState();
}

class _TimelineSectionState extends State<TimelineSection> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    return switch (widget.message.timelineType) {
      TimelineEntryType.toolCall => _buildToolCall(context),
      TimelineEntryType.thinking => _buildThinking(context),
      TimelineEntryType.subagent => _buildSubagent(context),
      TimelineEntryType.message => const SizedBox.shrink(),
    };
  }

  // -- Tool call --------------------------------------------------------------

  Widget _buildToolCall(BuildContext context) {
    final record = widget.message.toolCalls.isNotEmpty
        ? widget.message.toolCalls.first
        : null;
    if (record == null) return const SizedBox.shrink();

    final hasDetails = record.arguments != null || record.result != null;

    return _buildSection(
      context,
      icon: Icons.build_outlined,
      label: record.toolName,
      statusWidget: _toolStatusIcon(record.status),
      expandable: hasDetails,
      expandedContent: hasDetails ? _toolDetails(context, record) : null,
    );
  }

  Widget _toolStatusIcon(ToolCallStatus status) {
    return switch (status) {
      ToolCallStatus.pending => const SizedBox(
        width: 12,
        height: 12,
        child: CircularProgressIndicator(strokeWidth: 1.5),
      ),
      ToolCallStatus.ok => const Icon(
        Icons.check_circle_outline,
        size: 14,
        color: Colors.green,
      ),
      ToolCallStatus.error => const Icon(
        Icons.error_outline,
        size: 14,
        color: Colors.red,
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
              color: colorScheme.onSurface,
            ),
          ),
        ],
      ],
    );
  }

  // -- Thinking ---------------------------------------------------------------

  Widget _buildThinking(BuildContext context) {
    return _buildSection(
      context,
      icon: Icons.psychology_outlined,
      label: 'Thinking',
      statusWidget: widget.message.isStreaming
          ? const SizedBox(
              width: 12,
              height: 12,
              child: CircularProgressIndicator(strokeWidth: 1.5),
            )
          : null,
      expandable: widget.message.thinkingContent != null,
      expandedContent: widget.message.thinkingContent != null
          ? SelectableText(
              widget.message.thinkingContent!,
              style: TextStyle(
                fontSize: 12,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
                fontStyle: FontStyle.italic,
              ),
            )
          : null,
    );
  }

  // -- Subagent ---------------------------------------------------------------

  Widget _buildSubagent(BuildContext context) {
    final hasCompleted = widget.message.subagentSummary != null;
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
              child: CircularProgressIndicator(strokeWidth: 1.5),
            ),
      expandable: hasCompleted,
      expandedContent: hasCompleted
          ? Text(
              widget.message.subagentSummary!,
              style: TextStyle(
                fontSize: 12,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            )
          : null,
    );
  }

  // -- Shared section layout --------------------------------------------------

  Widget _buildSection(
    BuildContext context, {
    required IconData icon,
    required String label,
    Widget? statusWidget,
    bool expandable = false,
    Widget? expandedContent,
  }) {
    final colorScheme = Theme.of(context).colorScheme;

    return Align(
      alignment: Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.symmetric(vertical: 2),
        constraints: const BoxConstraints(maxWidth: 640),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            GestureDetector(
              onTap: expandable
                  ? () => setState(() => _expanded = !_expanded)
                  : null,
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
                    if (statusWidget != null) ...[
                      const SizedBox(width: 6),
                      statusWidget,
                    ],
                    if (expandable) ...[
                      const SizedBox(width: 4),
                      Icon(
                        _expanded
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
              duration: const Duration(milliseconds: 200),
              curve: Curves.easeInOut,
              child: _expanded && expandedContent != null
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
