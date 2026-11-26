@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';
import '../support/legacy_bridge.dart';

const _markdown = '''
## Summary

Golden tests pin the **current** rendering.

- first point
- second point with `inline code`

```dart
final answer = 42;
```
''';

ChatMessage _user(String content, {MessageStatus status = MessageStatus.ok}) =>
    ChatMessage(id: 'u', role: 'user', content: content, status: status);

ChatMessage _assistant(String content, {bool isStreaming = false}) =>
    ChatMessage(
      id: 'a',
      role: 'assistant',
      content: content,
      isStreaming: isStreaming,
    );

/// A stand-in for the media the app injects through a slot.
Widget _slot(String label) => Container(
  margin: const EdgeInsets.symmetric(vertical: 4),
  padding: const EdgeInsets.all(8),
  decoration: BoxDecoration(
    color: const Color(0x33888888),
    borderRadius: BorderRadius.circular(8),
  ),
  child: Text(label, style: const TextStyle(fontSize: 12)),
);

Widget _padded(Widget child) =>
    Padding(padding: const EdgeInsets.symmetric(horizontal: 12), child: child);

Widget _actions() => MessageActions(
  actions: [
    MessageActionButton(
      icon: Icons.copy_outlined,
      label: 'Copy',
      onPressed: () {},
    ),
  ],
);

Future<void> _golden(
  WidgetTester tester,
  String name,
  Widget widget, {
  Size size = const Size(420, 200),
  Brightness brightness = Brightness.light,
}) async {
  await pumpGolden(
    tester,
    withLegacyBridge(_padded(widget)),
    size: size,
    brightness: brightness,
  );
  await tester.pump(const Duration(milliseconds: 100));
  await expectLater(
    find.byType(MaterialApp),
    matchesGoldenFile('goldens/$name.png'),
  );
}

void main() {
  testWidgets('user', (tester) async {
    await _golden(
      tester,
      'message_bubble_user',
      MessageBubble(message: _user('Hello there')),
    );
  });

  testWidgets('user dark', (tester) async {
    await _golden(
      tester,
      'message_bubble_user_dark',
      MessageBubble(message: _user('Hello there')),
      brightness: Brightness.dark,
    );
  });

  testWidgets('user with attachment', (tester) async {
    await _golden(
      tester,
      'message_bubble_user_attachment',
      MessageBubble(
        message: _user('See attached'),
        attachments: _slot('chart.png'),
      ),
    );
  });

  testWidgets('user voice', (tester) async {
    await _golden(
      tester,
      'message_bubble_user_voice',
      MessageBubble(
        message: _user('Remind me to call Sam'),
        audio: _slot('voice message'),
      ),
    );
  });

  testWidgets('assistant', (tester) async {
    await _golden(
      tester,
      'message_bubble_assistant',
      MessageBubble(message: _assistant(_markdown), actions: _actions()),
      size: const Size(420, 360),
    );
  });

  testWidgets('assistant dark', (tester) async {
    await _golden(
      tester,
      'message_bubble_assistant_dark',
      MessageBubble(message: _assistant(_markdown), actions: _actions()),
      size: const Size(420, 360),
      brightness: Brightness.dark,
    );
  });

  testWidgets('grouped', (tester) async {
    await _golden(
      tester,
      'message_bubble_grouped',
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
    );
  });

  testWidgets('failed', (tester) async {
    await _golden(
      tester,
      'message_bubble_failed',
      MessageBubble(
        message: _user('This did not send', status: MessageStatus.failed),
      ),
    );
  });

  testWidgets('streaming empty', (tester) async {
    await _golden(
      tester,
      'message_bubble_streaming_empty',
      MessageBubble(message: _assistant('', isStreaming: true)),
      size: const Size(420, 100),
    );
  });

  testWidgets('with extension', (tester) async {
    await _golden(
      tester,
      'message_bubble_extension',
      MessageBubble(
        message: _assistant('Here is a card.'),
        extensions: [_slot('generative UI surface')],
      ),
    );
  });

  testWidgets('MessageBody image without a scope', (tester) async {
    await _golden(
      tester,
      'message_body_image_placeholder',
      const MessageBody(content: '![logo](https://example.com/logo.png)'),
      size: const Size(420, 60),
    );
  });

  testWidgets('TypingIndicator', (tester) async {
    await _golden(
      tester,
      'typing_indicator',
      const TypingIndicator(),
      size: const Size(120, 40),
    );
  });
}
