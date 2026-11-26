@Tags(['golden'])
library;

import 'dart:typed_data';

import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/message_meta_actions.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../support/fake_audioplayers.dart';
import '../../support/golden.dart';

// Before-image of MetaActionRow (chat-element-library PR 1b).

const _size = Size(420, 80);

Future<({Uint8List bytes, String mimeType})?> _noAudio() async => null;

Widget _row(
  ChatMessage message, {
  VoidCallback? onRetry,
  ServerCapabilities capabilities = ServerCapabilities.disabled,
}) => Padding(
  padding: const EdgeInsets.all(12),
  child: MetaActionRow(
    message: message,
    capabilities: capabilities,
    fetchMessageAudio: _noAudio,
    onRetry: onRetry,
  ),
);

Future<void> _golden(WidgetTester tester, String name, Widget widget) async {
  await pumpGolden(tester, widget, size: _size, alignment: Alignment.topLeft);
  await tester.pump(const Duration(milliseconds: 50));
  await expectGolden(name);
}

void main() {
  setUp(installFakeAudioplayers);

  testWidgets('all actions', (tester) async {
    await _golden(
      tester,
      'meta_actions_all',
      _row(
        ChatMessage(
          id: 'a',
          role: 'assistant',
          content: 'A reply',
          status: MessageStatus.failed,
        ),
        capabilities: const ServerCapabilities(
          voiceSend: false,
          voiceReceive: true,
        ),
        onRetry: () {},
      ),
    );
  });

  testWidgets('retry only', (tester) async {
    await _golden(
      tester,
      'meta_actions_retry_only',
      _row(
        ChatMessage(
          id: 'u',
          role: 'user',
          content: '',
          status: MessageStatus.failed,
        ),
        onRetry: () {},
      ),
    );
  });

  testWidgets('empty', (tester) async {
    await _golden(
      tester,
      'meta_actions_empty',
      _row(ChatMessage(id: 'u', role: 'user', content: '')),
    );
  });
}
