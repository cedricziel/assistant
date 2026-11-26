import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'theme/assistant_ui_spacing.dart';

/// Inline editor that replaces a user message bubble.
///
/// Edits text only: [attachments] render read-only beneath the field and carry
/// forward with the message. Send is disabled while the trimmed text is empty
/// or equal to [initialText]; it submits the trimmed text. Escape cancels.
class MessageEditor extends StatefulWidget {
  const MessageEditor({
    super.key,
    required this.initialText,
    required this.onSubmit,
    required this.onCancel,
    this.attachments = const [],
  });

  final String initialText;
  final ValueChanged<String> onSubmit;
  final VoidCallback onCancel;

  /// Read-only widgets shown below the field, for example attachment chips.
  final List<Widget> attachments;

  @override
  State<MessageEditor> createState() => _MessageEditorState();
}

class _MessageEditorState extends State<MessageEditor> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.initialText,
  );

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  bool _canSend(String text) {
    final trimmed = text.trim();
    return trimmed.isNotEmpty && trimmed != widget.initialText.trim();
  }

  void _submit() => widget.onSubmit(_controller.text.trim());

  @override
  Widget build(BuildContext context) {
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): widget.onCancel,
      },
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(
            controller: _controller,
            autofocus: true,
            minLines: 1,
            maxLines: 8,
            keyboardType: TextInputType.multiline,
            textInputAction: TextInputAction.newline,
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              labelText: 'Edit message',
            ),
          ),
          if (widget.attachments.isNotEmpty) ...[
            const SizedBox(height: AssistantUiSpacing.sm),
            Wrap(
              spacing: AssistantUiSpacing.sm,
              runSpacing: AssistantUiSpacing.sm,
              children: widget.attachments,
            ),
          ],
          const SizedBox(height: AssistantUiSpacing.sm),
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton(
                onPressed: widget.onCancel,
                child: const Text('Cancel'),
              ),
              const SizedBox(width: AssistantUiSpacing.sm),
              ValueListenableBuilder<TextEditingValue>(
                valueListenable: _controller,
                builder: (context, value, _) => FilledButton(
                  onPressed: _canSend(value.text) ? _submit : null,
                  child: const Text('Send'),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
