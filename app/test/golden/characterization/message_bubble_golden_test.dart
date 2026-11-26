@Tags(['golden'])
library;

import 'dart:typed_data';

import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/message_bubble.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../support/fake_audioplayers.dart';
import '../../support/golden.dart';

// Before-image of MessageBubble and its MetaActionRow (chat-element-library
// PR 1b). Media is stubbed: attachments have no imageBaseUrl so no network
// image is requested, voice playback runs against a fake audioplayers
// platform, and read-aloud audio is never fetched.

const _size = Size(420, 260);

Future<({Uint8List bytes, String mimeType})?> _noAudio() async => null;

Widget _bubble(
  ChatMessage message, {
  bool isGrouped = false,
  VoidCallback? onRetry,
  VoidCallback? onStop,
  ServerCapabilities capabilities = ServerCapabilities.disabled,
}) => Padding(
  padding: const EdgeInsets.symmetric(horizontal: 12),
  child: MessageBubble(
    message: message,
    capabilities: capabilities,
    fetchMessageAudio: _noAudio,
    isGrouped: isGrouped,
    onRetry: onRetry,
    onStop: onStop,
  ),
);

ChatMessage _user(
  String content, {
  MessageStatus status = MessageStatus.ok,
  List<ChatAttachment>? attachments,
  Uint8List? audioBytes,
}) => ChatMessage(
  id: 'u',
  role: 'user',
  content: content,
  status: status,
  attachments: attachments,
  audioBytes: audioBytes,
  audioMimeType: audioBytes == null ? null : 'audio/webm',
);

ChatMessage _assistant(
  String content, {
  bool isStreaming = false,
  List<ChatAttachment>? attachments,
}) => ChatMessage(
  id: 'a',
  role: 'assistant',
  content: content,
  isStreaming: isStreaming,
  attachments: attachments,
);

const _attachments = [
  ChatAttachment(
    id: 'img',
    filename: 'chart.png',
    mimeType: 'image/png',
    url: '/media/img',
  ),
  ChatAttachment(
    id: 'doc',
    filename: 'report.pdf',
    mimeType: 'application/pdf',
    url: '/media/doc',
  ),
];

const _markdown = '''
## Summary

Golden tests pin the **current** rendering.

- first point
- second point with `inline code`

```dart
final answer = 42;
```
''';

Future<void> _golden(
  WidgetTester tester,
  String name,
  Widget widget, {
  Size size = _size,
  Brightness brightness = Brightness.light,
}) async {
  await pumpGolden(tester, widget, size: size, brightness: brightness);
  await tester.pump(const Duration(milliseconds: 100));
  await expectGolden(name);
}

void main() {
  setUp(installFakeAudioplayers);

  testWidgets('user', (tester) async {
    await _golden(tester, 'bubble_user', _bubble(_user('Hello there')));
  });

  testWidgets('user dark', (tester) async {
    await _golden(
      tester,
      'bubble_user_dark',
      _bubble(_user('Hello there')),
      brightness: Brightness.dark,
    );
  });

  testWidgets('user with attachment', (tester) async {
    await _golden(
      tester,
      'bubble_user_attachment',
      _bubble(_user('See attached', attachments: _attachments)),
      size: const Size(420, 360),
    );
  });

  testWidgets('user voice', (tester) async {
    await _golden(
      tester,
      'bubble_user_voice',
      _bubble(_user('Remind me to call Sam', audioBytes: Uint8List(8))),
    );
  });

  testWidgets('assistant markdown', (tester) async {
    await _golden(
      tester,
      'bubble_assistant_markdown',
      _bubble(_assistant(_markdown)),
      size: const Size(420, 420),
    );
  });

  testWidgets('assistant with attachment', (tester) async {
    await _golden(
      tester,
      'bubble_assistant_attachment',
      _bubble(_assistant('Here is the chart.', attachments: _attachments)),
      size: const Size(420, 360),
    );
  });

  testWidgets('grouped', (tester) async {
    await _golden(
      tester,
      'bubble_grouped',
      Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          _bubble(_assistant('First part of the reply.')),
          _bubble(_assistant('Second part, grouped.'), isGrouped: true),
        ],
      ),
    );
  });

  testWidgets('failed', (tester) async {
    await _golden(
      tester,
      'bubble_failed',
      _bubble(
        _user('This did not send', status: MessageStatus.failed),
        onRetry: () {},
      ),
    );
  });

  testWidgets('streaming empty', (tester) async {
    await _golden(
      tester,
      'bubble_streaming_empty',
      _bubble(_assistant('', isStreaming: true), onStop: () {}),
    );
  });
}
