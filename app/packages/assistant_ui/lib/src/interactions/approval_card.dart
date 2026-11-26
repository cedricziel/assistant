import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../models/interaction_record.dart';
import '../theme/assistant_ui_spacing.dart';

/// A tool approval the turn is waiting on, rendered inline with the tool call.
///
/// Pending, it shows the tool, a one-line summary of its primary argument,
/// expandable full arguments, the reason, the time left, and one button per
/// option the server offered. A tap calls [onDecision]; the element never
/// answers anything itself. While [InteractionRecord.status] is
/// [InteractionStatus.submitting] the buttons are disabled. Once answered,
/// expired or cancelled, the card is a read-only receipt with no buttons.
///
/// [now] is injected so the countdown is deterministic; the app rebuilds the
/// card as time passes.
///
/// Enter on the focused card means "Allow once", on every platform. Escape is
/// not bound, so a stray key cannot deny a call. Enter on a focused button
/// still activates that button.
class ApprovalCard extends StatefulWidget {
  const ApprovalCard({
    super.key,
    required this.record,
    required this.now,
    required this.onDecision,
  });

  final InteractionRecord record;
  final DateTime now;
  final ValueChanged<ApprovalDecision> onDecision;

  @override
  State<ApprovalCard> createState() => _ApprovalCardState();
}

class _ApprovalCardState extends State<ApprovalCard> {
  final FocusNode _focusNode = FocusNode(debugLabel: 'ApprovalCard');
  bool _argumentsExpanded = false;

  @override
  void dispose() {
    _focusNode.dispose();
    super.dispose();
  }

  KeyEventResult _onKeyEvent(FocusNode node, KeyEvent event) {
    final record = widget.record;
    final isEnter =
        event.logicalKey == LogicalKeyboardKey.enter ||
        event.logicalKey == LogicalKeyboardKey.numpadEnter;
    if (event is! KeyDownEvent ||
        !isEnter ||
        !node.hasPrimaryFocus ||
        record.status != InteractionStatus.pending ||
        !record.options.contains(ApprovalDecision.allowOnce)) {
      return KeyEventResult.ignored;
    }
    widget.onDecision(ApprovalDecision.allowOnce);
    return KeyEventResult.handled;
  }

  @override
  Widget build(BuildContext context) {
    final record = widget.record;
    final scheme = Theme.of(context).colorScheme;
    final isReceipt =
        record.status != InteractionStatus.pending &&
        record.status != InteractionStatus.submitting;

    return Focus(
      focusNode: _focusNode,
      canRequestFocus: !isReceipt,
      onKeyEvent: _onKeyEvent,
      child: Semantics(
        key: const Key('approval_card_focus'),
        container: true,
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: scheme.surfaceContainerLow,
            border: Border.all(color: scheme.outlineVariant),
            borderRadius: BorderRadius.circular(AssistantUiSpacing.md),
          ),
          child: Padding(
            padding: const EdgeInsets.all(AssistantUiSpacing.md),
            child: isReceipt ? _buildReceipt(context) : _buildPending(context),
          ),
        ),
      ),
    );
  }

  // -- Pending --

  Widget _buildPending(BuildContext context) {
    final record = widget.record;
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final summary = _summary(record.arguments);
    final reason = record.reason;
    final offered = [
      for (final d in ApprovalDecision.values)
        if (record.options.contains(d)) d,
    ];
    final offersAlways = offered.any((d) => d.isAlways);
    final submitting = record.status == InteractionStatus.submitting;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          children: [
            Icon(Icons.shield_outlined, size: 18, color: scheme.primary),
            const SizedBox(width: AssistantUiSpacing.sm),
            Expanded(
              child: Text(
                record.toolName ?? 'Unknown tool',
                style: theme.textTheme.titleSmall,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            Text(
              _countdown(record.expiresAt.difference(widget.now)),
              style: theme.textTheme.labelSmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
        if (summary != null) ...[
          const SizedBox(height: AssistantUiSpacing.xs),
          Text(
            summary,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: _mono(theme.textTheme.bodyMedium),
          ),
        ],
        if (reason != null && reason.isNotEmpty) ...[
          const SizedBox(height: AssistantUiSpacing.xs),
          Text(
            reason,
            style: theme.textTheme.bodySmall?.copyWith(
              color: scheme.onSurfaceVariant,
            ),
          ),
        ],
        if (record.arguments.isNotEmpty) ..._buildArguments(context),
        const SizedBox(height: AssistantUiSpacing.md),
        Wrap(
          spacing: AssistantUiSpacing.sm,
          runSpacing: AssistantUiSpacing.sm,
          children: [
            for (final d in offered) _buildButton(d, enabled: !submitting),
          ],
        ),
        if (offersAlways) ...[
          const SizedBox(height: AssistantUiSpacing.sm),
          if (record.suggestedRule != null)
            Text(
              record.suggestedRule!,
              style: _mono(theme.textTheme.bodySmall),
            ),
          Text(
            'Always allow or always reject applies to all your turns in this '
            'space, including scheduled tasks and messenger chats.',
            style: theme.textTheme.bodySmall?.copyWith(
              color: scheme.onSurfaceVariant,
            ),
          ),
        ],
      ],
    );
  }

  List<Widget> _buildArguments(BuildContext context) {
    final record = widget.record;
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return [
      Align(
        alignment: AlignmentDirectional.centerStart,
        child: TextButton.icon(
          key: const Key('approval_arguments_toggle'),
          onPressed: () =>
              setState(() => _argumentsExpanded = !_argumentsExpanded),
          icon: Icon(
            _argumentsExpanded ? Icons.expand_less : Icons.expand_more,
            size: 18,
          ),
          label: Text(_argumentsExpanded ? 'Hide arguments' : 'Show arguments'),
        ),
      ),
      if (_argumentsExpanded) ...[
        Container(
          key: const Key('approval_arguments'),
          padding: const EdgeInsets.all(AssistantUiSpacing.sm),
          decoration: BoxDecoration(
            color: scheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(AssistantUiSpacing.sm),
          ),
          child: Text(
            const JsonEncoder.withIndent('  ').convert(record.arguments),
            style: _mono(theme.textTheme.bodySmall),
          ),
        ),
        if (record.argumentsTruncated)
          Padding(
            padding: const EdgeInsets.only(top: AssistantUiSpacing.xs),
            child: Text(
              'These arguments were shortened. The tool runs with the full '
              'arguments.',
              style: theme.textTheme.bodySmall?.copyWith(
                color: scheme.onSurfaceVariant,
              ),
            ),
          ),
      ],
    ];
  }

  Widget _buildButton(ApprovalDecision decision, {required bool enabled}) {
    final tool = widget.record.toolName ?? 'tool';
    final (label, semantics) = switch (decision) {
      ApprovalDecision.allowOnce => ('Allow once', 'Allow $tool once'),
      ApprovalDecision.allowAlways => ('Always allow', 'Always allow $tool'),
      ApprovalDecision.rejectOnce => ('Reject once', 'Reject $tool once'),
      ApprovalDecision.rejectAlways => ('Always reject', 'Always reject $tool'),
    };
    final key = Key('approval_${decision.wire}');
    final onPressed = enabled ? () => widget.onDecision(decision) : null;
    final text = Text(label);
    final button = switch (decision) {
      ApprovalDecision.allowOnce => FilledButton(
        key: key,
        onPressed: onPressed,
        child: text,
      ),
      ApprovalDecision.rejectOnce => OutlinedButton(
        key: key,
        onPressed: onPressed,
        child: text,
      ),
      ApprovalDecision.allowAlways || ApprovalDecision.rejectAlways =>
        TextButton(key: key, onPressed: onPressed, child: text),
    };
    return Semantics(
      label: semantics,
      button: true,
      enabled: enabled,
      onTap: onPressed,
      excludeSemantics: true,
      child: button,
    );
  }

  // -- Receipt --

  Widget _buildReceipt(BuildContext context) {
    final record = widget.record;
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final (icon, title) = _receipt(record);
    final subject = [
      record.toolName,
      _summary(record.arguments),
    ].whereType<String>().join(' · ');

    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, size: 18, color: scheme.onSurfaceVariant),
        const SizedBox(width: AssistantUiSpacing.sm),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: theme.textTheme.titleSmall),
              if (subject.isNotEmpty)
                Text(
                  subject,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }

  static (IconData, String) _receipt(InteractionRecord record) {
    final rule = record.suggestedRule;
    String withRule(String text) => rule == null ? text : '$text · $rule';
    return switch (record.status) {
      InteractionStatus.resolved => switch (record.decision) {
        ApprovalDecision.allowOnce => (
          Icons.check_circle_outline,
          'Allowed once',
        ),
        ApprovalDecision.allowAlways => (
          Icons.check_circle_outline,
          withRule('Always allowed'),
        ),
        ApprovalDecision.rejectOnce => (Icons.block, 'Rejected once'),
        ApprovalDecision.rejectAlways => (
          Icons.block,
          withRule('Always rejected'),
        ),
        null => (Icons.check_circle_outline, 'Answered'),
      },
      InteractionStatus.expired => (
        Icons.schedule,
        'Expired without an answer',
      ),
      InteractionStatus.cancelled => (Icons.cancel_outlined, 'Cancelled'),
      InteractionStatus.pending || InteractionStatus.submitting => (
        Icons.shield_outlined,
        'Approval needed',
      ),
    };
  }

  // -- Helpers --

  static const _summaryKeys = ['command', 'url', 'path', 'query', 'pattern'];

  /// One line naming what the call acts on, or null when there is nothing.
  static String? _summary(Map<String, Object?> arguments) {
    if (arguments.isEmpty) return null;
    for (final key in _summaryKeys) {
      final value = arguments[key];
      if (value is String && value.isNotEmpty) return _oneLine(value);
    }
    for (final value in arguments.values) {
      if (value is String && value.isNotEmpty) return _oneLine(value);
    }
    return _oneLine(jsonEncode(arguments));
  }

  static String _oneLine(String text) => text.replaceAll(RegExp(r'\s+'), ' ');

  static String _countdown(Duration remaining) {
    if (remaining <= Duration.zero) return 'Expiring…';
    final minutes = remaining.inMinutes;
    final seconds = (remaining.inSeconds % 60).toString().padLeft(2, '0');
    return 'Expires in $minutes:$seconds';
  }

  static TextStyle? _mono(TextStyle? base) =>
      base?.copyWith(fontFamily: 'monospace');
}
