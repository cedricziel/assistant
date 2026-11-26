import 'package:material_ui/material_ui.dart';

import '../models/interaction_record.dart';
import '../theme/assistant_ui_spacing.dart';

/// Questions the assistant put to its owner, rendered inline with the
/// `ask-user` tool call.
///
/// Pending, each question shows as radios (single choice), checkboxes
/// (multiple) and/or a text field. Submit is enabled once every question has an
/// answer, and calls [onAnswer] with one [QuestionAnswer] per question in
/// order. [onDecline] means "I won't answer". The element never answers
/// anything itself. While [InteractionRecord.status] is
/// [InteractionStatus.submitting] the inputs and both actions are disabled.
/// Once answered, declined, expired or cancelled, the card is a read-only
/// receipt.
///
/// [now] is injected so the countdown is deterministic; the app rebuilds the
/// card as time passes.
class QuestionCard extends StatefulWidget {
  const QuestionCard({
    super.key,
    required this.record,
    required this.now,
    required this.onAnswer,
    required this.onDecline,
  });

  final InteractionRecord record;
  final DateTime now;
  final ValueChanged<List<QuestionAnswer>> onAnswer;
  final VoidCallback onDecline;

  /// Longest free-text answer the server accepts.
  static const maxTextLength = 4000;

  @override
  State<QuestionCard> createState() => _QuestionCardState();
}

class _QuestionCardState extends State<QuestionCard> {
  /// Chosen option ids per question id, in tap order.
  final Map<String, List<String>> _selected = {};
  final Map<String, TextEditingController> _texts = {};

  @override
  void dispose() {
    for (final controller in _texts.values) {
      controller.dispose();
    }
    super.dispose();
  }

  TextEditingController _controllerFor(String questionId) => _texts.putIfAbsent(
    questionId,
    () => TextEditingController()..addListener(_onTextChanged),
  );

  void _onTextChanged() => setState(() {});

  String _textOf(QuestionItem q) => _texts[q.id]?.text.trim() ?? '';

  bool _isAnswered(QuestionItem q) =>
      (_selected[q.id]?.isNotEmpty ?? false) ||
      (q.freeText && _textOf(q).isNotEmpty);

  bool get _complete =>
      widget.record.questions.isNotEmpty &&
      widget.record.questions.every(_isAnswered);

  void _select(QuestionItem q, String optionId, bool on) => setState(() {
    final chosen = _selected.putIfAbsent(q.id, () => []);
    if (!q.multiple) {
      chosen
        ..clear()
        ..add(optionId);
    } else if (on) {
      if (!chosen.contains(optionId)) chosen.add(optionId);
    } else {
      chosen.remove(optionId);
    }
  });

  void _submit() => widget.onAnswer([
    for (final q in widget.record.questions)
      QuestionAnswer(
        questionId: q.id,
        selectedOptionIds: [..._selected[q.id] ?? const <String>[]],
        text: q.freeText && _textOf(q).isNotEmpty ? _textOf(q) : null,
      ),
  ]);

  @override
  Widget build(BuildContext context) {
    final record = widget.record;
    final scheme = Theme.of(context).colorScheme;
    final isReceipt =
        record.status != InteractionStatus.pending &&
        record.status != InteractionStatus.submitting;

    return Semantics(
      container: true,
      // A Material, not a DecoratedBox: the list tiles paint on the nearest
      // Material and a coloured box in between would hide their ink.
      child: Material(
        color: scheme.surfaceContainerLow,
        shape: RoundedRectangleBorder(
          side: BorderSide(color: scheme.outlineVariant),
          borderRadius: BorderRadius.circular(AssistantUiSpacing.md),
        ),
        child: Padding(
          padding: const EdgeInsets.all(AssistantUiSpacing.md),
          child: isReceipt ? _buildReceipt(context) : _buildPending(context),
        ),
      ),
    );
  }

  // -- Pending --

  Widget _buildPending(BuildContext context) {
    final record = widget.record;
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final submitting = record.status == InteractionStatus.submitting;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          children: [
            Icon(Icons.help_outline, size: 18, color: scheme.primary),
            const SizedBox(width: AssistantUiSpacing.sm),
            Expanded(
              child: Text(
                'Your assistant has a question',
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
        for (final q in record.questions) ...[
          const SizedBox(height: AssistantUiSpacing.md),
          _buildQuestion(context, q, enabled: !submitting),
        ],
        const SizedBox(height: AssistantUiSpacing.md),
        Wrap(
          spacing: AssistantUiSpacing.sm,
          runSpacing: AssistantUiSpacing.sm,
          children: [
            FilledButton(
              key: const Key('question_submit'),
              onPressed: _complete && !submitting ? _submit : null,
              child: const Text('Submit'),
            ),
            TextButton(
              key: const Key('question_decline'),
              onPressed: submitting ? null : widget.onDecline,
              child: const Text("I won't answer"),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildQuestion(
    BuildContext context,
    QuestionItem q, {
    required bool enabled,
  }) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(q.prompt, style: theme.textTheme.bodyMedium),
        if (q.options.isNotEmpty)
          q.multiple
              ? _buildCheckboxes(q, enabled: enabled)
              : _buildRadios(q, enabled: enabled),
        if (q.freeText || q.options.isEmpty)
          Padding(
            padding: const EdgeInsets.only(top: AssistantUiSpacing.xs),
            child: TextField(
              controller: _controllerFor(q.id),
              enabled: enabled,
              maxLength: QuestionCard.maxTextLength,
              minLines: 1,
              maxLines: 4,
              decoration: InputDecoration(
                isDense: true,
                border: const OutlineInputBorder(),
                hintText: q.options.isEmpty
                    ? 'Your answer'
                    : 'Or write your own',
                counterText: '',
              ),
            ),
          ),
      ],
    );
  }

  Widget _buildRadios(QuestionItem q, {required bool enabled}) {
    final selected = _selected[q.id];
    return RadioGroup<String>(
      groupValue: selected == null || selected.isEmpty ? null : selected.first,
      onChanged: (value) {
        if (enabled && value != null) _select(q, value, true);
      },
      child: Column(
        children: [
          for (final o in q.options)
            RadioListTile<String>(
              value: o.id,
              enabled: enabled,
              dense: true,
              contentPadding: EdgeInsets.zero,
              title: Text(o.label),
              subtitle: o.description == null ? null : Text(o.description!),
            ),
        ],
      ),
    );
  }

  Widget _buildCheckboxes(QuestionItem q, {required bool enabled}) => Column(
    children: [
      for (final o in q.options)
        CheckboxListTile(
          value: _selected[q.id]?.contains(o.id) ?? false,
          onChanged: enabled ? (on) => _select(q, o.id, on ?? false) : null,
          dense: true,
          contentPadding: EdgeInsets.zero,
          controlAffinity: ListTileControlAffinity.leading,
          title: Text(o.label),
          subtitle: o.description == null ? null : Text(o.description!),
        ),
    ],
  );

  // -- Receipt --

  Widget _buildReceipt(BuildContext context) {
    final record = widget.record;
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final (icon, title) = _receipt(record);
    final showAnswers =
        record.status == InteractionStatus.resolved && !record.declined;

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
              for (final q in record.questions) ...[
                const SizedBox(height: AssistantUiSpacing.xs),
                Text(
                  q.prompt,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
                if (showAnswers)
                  Text(_answerText(q), style: theme.textTheme.bodyMedium),
              ],
            ],
          ),
        ),
      ],
    );
  }

  static (IconData, String) _receipt(InteractionRecord record) =>
      switch (record.status) {
        InteractionStatus.resolved when record.declined => (
          Icons.block,
          'Declined',
        ),
        InteractionStatus.resolved => (Icons.check_circle_outline, 'Answered'),
        InteractionStatus.expired => (Icons.schedule, 'Expired'),
        InteractionStatus.cancelled => (Icons.cancel_outlined, 'Cancelled'),
        InteractionStatus.pending ||
        InteractionStatus.submitting => (Icons.help_outline, 'Question'),
      };

  /// The chosen labels and any free text of the answer to [q], one line.
  String _answerText(QuestionItem q) {
    final answer = widget.record.answers
        .where((a) => a.questionId == q.id)
        .firstOrNull;
    if (answer == null) return '—';
    final labels = [
      for (final id in answer.selectedOptionIds)
        for (final o in q.options)
          if (o.id == id) o.label,
    ];
    final text = answer.text;
    final parts = [...labels, if (text != null && text.isNotEmpty) text];
    return parts.isEmpty ? '—' : parts.join(', ');
  }

  // -- Helpers --

  static String _countdown(Duration remaining) {
    if (remaining <= Duration.zero) return 'Expiring…';
    final minutes = remaining.inMinutes;
    final seconds = (remaining.inSeconds % 60).toString().padLeft(2, '0');
    return 'Expires in $minutes:$seconds';
  }
}
