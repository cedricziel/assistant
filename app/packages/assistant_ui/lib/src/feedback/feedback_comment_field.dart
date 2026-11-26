import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../theme/assistant_ui_spacing.dart';

/// An inline field for the optional comment that goes with a rating.
///
/// [initialText] prefills it. Send, or Enter, calls [onSubmit] with the
/// trimmed text; it is disabled while the trimmed text is empty or [sending]
/// is true. The close button, or Escape, calls [onDismiss]. The text is limited
/// to [maxLength] Unicode scalar values, not grapheme clusters, so the limit
/// matches a server that counts `char`s. The element holds only the text being
/// typed and never calls anything itself.
///
/// The parts carry the keys `feedback_comment_input`, `feedback_comment_send`
/// and `feedback_comment_dismiss`.
class FeedbackCommentField extends StatefulWidget {
  const FeedbackCommentField({
    super.key,
    required this.onSubmit,
    required this.onDismiss,
    this.initialText,
    this.maxLength = 2000,
    this.sending = false,
  });

  final String? initialText;
  final ValueChanged<String> onSubmit;
  final VoidCallback onDismiss;
  final int maxLength;
  final bool sending;

  @override
  State<FeedbackCommentField> createState() => _FeedbackCommentFieldState();
}

class _FeedbackCommentFieldState extends State<FeedbackCommentField> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.initialText,
  );

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  bool get _canSend => !widget.sending && _controller.text.trim().isNotEmpty;

  void _submit() {
    if (_canSend) widget.onSubmit(_controller.text.trim());
  }

  @override
  Widget build(BuildContext context) {
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): widget.onDismiss,
      },
      child: Row(
        children: [
          Expanded(
            child: TextField(
              key: const Key('feedback_comment_input'),
              controller: _controller,
              autofocus: true,
              textInputAction: TextInputAction.send,
              onSubmitted: (_) => _submit(),
              inputFormatters: [ScalarLengthFormatter(widget.maxLength)],
              style: const TextStyle(fontSize: 13),
              decoration: const InputDecoration(
                hintText: 'Add a comment (optional)',
                isDense: true,
                border: OutlineInputBorder(),
              ),
            ),
          ),
          const SizedBox(width: AssistantUiSpacing.xs),
          ValueListenableBuilder<TextEditingValue>(
            valueListenable: _controller,
            builder: (context, _, _) => IconButton(
              key: const Key('feedback_comment_send'),
              tooltip: 'Send comment',
              icon: const Icon(Icons.send, size: 18),
              onPressed: _canSend ? _submit : null,
            ),
          ),
          IconButton(
            key: const Key('feedback_comment_dismiss'),
            tooltip: 'Dismiss',
            icon: const Icon(Icons.close, size: 18),
            onPressed: widget.onDismiss,
          ),
        ],
      ),
    );
  }
}

/// Caps text at [maxLength] Unicode scalar values.
///
/// `TextField.maxLength` counts grapheme clusters, so a run of multi-scalar
/// emoji would pass a limit the server then rejects. Text over the limit is
/// cut, which also handles a paste that overshoots.
class ScalarLengthFormatter extends TextInputFormatter {
  const ScalarLengthFormatter(this.maxLength);

  final int maxLength;

  @override
  TextEditingValue formatEditUpdate(
    TextEditingValue oldValue,
    TextEditingValue newValue,
  ) {
    if (newValue.text.runes.length <= maxLength) return newValue;
    final text = String.fromCharCodes(newValue.text.runes.take(maxLength));
    return TextEditingValue(
      text: text,
      selection: TextSelection.collapsed(
        offset: newValue.selection.baseOffset.clamp(0, text.length),
      ),
    );
  }
}
