import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/message_meta_actions.dart';

Widget _wrap(ChatMessage message, {VoidCallback? onStop}) {
  return MaterialApp(
    home: Scaffold(
      body: MetaActionRow(
        message: message,
        capabilities: ServerCapabilities.disabled,
        fetchMessageAudio: () async => (bytes: Uint8List(0), mimeType: ''),
        onStop: onStop,
      ),
    ),
  );
}

void main() {
  group('MetaActionRow', () {
    testWidgets('a stopped assistant message shows Stopped and Copy', (
      tester,
    ) async {
      final message = ChatMessage(
        id: 'a1',
        role: 'assistant',
        content: 'The capital of France',
        status: MessageStatus.stopped,
      );

      await tester.pumpWidget(_wrap(message));

      expect(find.byKey(const Key('stopped_label')), findsOneWidget);
      expect(find.text('Stopped'), findsOneWidget);
      expect(find.byKey(const Key('copy_action')), findsOneWidget);
      expect(find.byKey(const Key('stop_action')), findsNothing);
    });

    testWidgets('an ordinary assistant message shows no Stopped label', (
      tester,
    ) async {
      final message = ChatMessage(id: 'a1', role: 'assistant', content: 'done');

      await tester.pumpWidget(_wrap(message));

      expect(find.byKey(const Key('stopped_label')), findsNothing);
      expect(find.byKey(const Key('copy_action')), findsOneWidget);
    });
  });
}
