import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

const _markdown = '''
## Summary

The reply renders **markdown**.

- first point
- second point with `inline code`
''';

ChatMessage _user(String content, {MessageStatus? status}) => ChatMessage(
  id: 'u',
  role: 'user',
  content: content,
  status: status ?? MessageStatus.ok,
);

ChatMessage _assistant(String content, {bool isStreaming = false}) =>
    ChatMessage(
      id: 'a',
      role: 'assistant',
      content: content,
      isStreaming: isStreaming,
    );

/// A stand-in for the media widgets the app injects.
Widget _slot(String label, Color color) => Container(
  margin: const EdgeInsets.symmetric(vertical: 4),
  padding: const EdgeInsets.all(8),
  decoration: BoxDecoration(
    color: color,
    borderRadius: BorderRadius.circular(8),
  ),
  child: Text(label, style: const TextStyle(fontSize: 12)),
);

Widget _actions() => MessageActions(
  actions: [
    MessageActionButton(
      icon: Icons.copy_outlined,
      label: 'Copy',
      onPressed: () {},
    ),
  ],
);

Widget _padded(Widget child) =>
    Padding(padding: const EdgeInsets.symmetric(horizontal: 12), child: child);

final messageBubbleComponent = WidgetbookComponent(
  name: 'MessageBubble',
  useCases: [
    WidgetbookUseCase(
      name: 'playground',
      builder: (context) {
        final isUser = context.knobs.boolean(label: 'user', initialValue: true);
        final content = context.knobs.string(
          label: 'content',
          initialValue: 'Hello there',
        );
        return _padded(
          MessageBubble(
            message: isUser ? _user(content) : _assistant(content),
            isGrouped: context.knobs.boolean(label: 'grouped'),
            actions: _actions(),
          ),
        );
      },
    ),
    WidgetbookUseCase(
      name: 'user',
      builder: (context) =>
          _padded(MessageBubble(message: _user('Hello there'))),
    ),
    WidgetbookUseCase(
      name: 'user with attachment',
      builder: (context) => _padded(
        MessageBubble(
          message: _user('See attached'),
          attachments: _slot('chart.png', Colors.white24),
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'user voice',
      builder: (context) => _padded(
        MessageBubble(
          message: _user('Remind me to call Sam'),
          audio: _slot('voice message', Colors.white24),
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'assistant',
      builder: (context) => _padded(
        MessageBubble(message: _assistant(_markdown), actions: _actions()),
      ),
    ),
    WidgetbookUseCase(
      name: 'grouped',
      builder: (context) => _padded(
        Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            MessageBubble(message: _assistant('First part of the reply.')),
            MessageBubble(
              message: _assistant('Second part, grouped.'),
              isGrouped: true,
            ),
          ],
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'failed',
      builder: (context) => _padded(
        MessageBubble(
          message: _user('This did not send', status: MessageStatus.failed),
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'streaming empty',
      builder: (context) =>
          _padded(MessageBubble(message: _assistant('', isStreaming: true))),
    ),
    WidgetbookUseCase(
      name: 'with extension',
      builder: (context) => _padded(
        MessageBubble(
          message: _assistant('Here is a card.'),
          extensions: [_slot('generative UI surface', Colors.black12)],
        ),
      ),
    ),
  ],
);
