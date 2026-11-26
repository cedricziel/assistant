import 'dart:typed_data';

import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/message_actions_builder.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../support/fake_audioplayers.dart';

const _voice = ServerCapabilities(voiceSend: false, voiceReceive: true);

Widget _wrap(
  ChatMessage message, {
  ServerCapabilities capabilities = ServerCapabilities.disabled,
  VoidCallback? onRetry,
  VoidCallback? onStop,
  Widget? timing,
  Widget? feedback,
}) => MaterialApp(
  home: Scaffold(
    body: Align(
      alignment: Alignment.topLeft,
      child: buildMessageActions(
        message: message,
        capabilities: capabilities,
        fetchMessageAudio: () async => (bytes: Uint8List(0), mimeType: ''),
        onRetry: onRetry,
        onStop: onStop,
        timing: timing,
        feedback: feedback,
      ),
    ),
  ),
);

ChatMessage _assistant({
  String content = 'A reply',
  MessageStatus status = MessageStatus.ok,
  bool isStreaming = false,
}) => ChatMessage(
  id: 'a',
  role: 'assistant',
  content: content,
  status: status,
  isStreaming: isStreaming,
);

double _left(WidgetTester tester, String key) =>
    tester.getTopLeft(find.byKey(Key(key))).dx;

void main() {
  setUp(installFakeAudioplayers);

  group('buildMessageActions', () {
    testWidgets('failed assistant message lists actions in today\'s order', (
      tester,
    ) async {
      await tester.pumpWidget(
        _wrap(
          _assistant(status: MessageStatus.failed),
          capabilities: _voice,
          onRetry: () {},
        ),
      );

      const keys = [
        'read_aloud_action',
        'copy_action',
        'share_action',
        'save_action',
        'retry_button',
      ];
      for (final k in keys) {
        expect(find.byKey(Key(k)), findsOneWidget, reason: k);
      }
      final lefts = [for (final k in keys) _left(tester, k)];
      expect(lefts, [...lefts]..sort(), reason: 'left-to-right order');
      expect(find.byKey(const Key('stop_action')), findsNothing);
    });

    testWidgets('retry uses the danger tone', (tester) async {
      await tester.pumpWidget(
        _wrap(_assistant(status: MessageStatus.failed), onRetry: () {}),
      );

      final retry = tester.widget<MessageActionButton>(
        find.byKey(const Key('retry_button')),
      );
      expect(retry.tone, MessageActionTone.danger);
      final copy = tester.widget<MessageActionButton>(
        find.byKey(const Key('copy_action')),
      );
      expect(copy.tone, MessageActionTone.muted);
    });

    testWidgets('user messages only get Copy', (tester) async {
      await tester.pumpWidget(
        _wrap(ChatMessage(id: 'u', role: 'user', content: 'hi')),
      );

      expect(find.byKey(const Key('copy_action')), findsOneWidget);
      expect(find.byKey(const Key('share_action')), findsNothing);
      expect(find.byKey(const Key('save_action')), findsNothing);
      expect(find.byKey(const Key('read_aloud_action')), findsNothing);
    });

    testWidgets('a streaming message shows Stop only', (tester) async {
      await tester.pumpWidget(
        _wrap(_assistant(isStreaming: true), onStop: () {}),
      );

      expect(find.byKey(const Key('stop_action')), findsOneWidget);
      expect(find.byKey(const Key('copy_action')), findsNothing);
    });

    testWidgets('a streaming message without onStop renders nothing', (
      tester,
    ) async {
      await tester.pumpWidget(_wrap(_assistant(isStreaming: true)));

      expect(find.byType(MessageActionButton), findsNothing);
    });

    testWidgets('an empty user message renders nothing', (tester) async {
      await tester.pumpWidget(
        _wrap(ChatMessage(id: 'u', role: 'user', content: '')),
      );

      expect(tester.getSize(find.byType(MessageActions)), Size.zero);
    });

    testWidgets('trailing is timing first, then feedback, after the actions', (
      tester,
    ) async {
      await tester.pumpWidget(
        _wrap(
          _assistant(),
          timing: const Text('timing', key: Key('timing')),
          feedback: const Text('feedback', key: Key('feedback')),
        ),
      );

      expect(_left(tester, 'save_action'), lessThan(_left(tester, 'timing')));
      expect(_left(tester, 'timing'), lessThan(_left(tester, 'feedback')));
    });

    testWidgets('a stopped assistant message shows Stopped and Copy', (
      tester,
    ) async {
      await tester.pumpWidget(
        _wrap(
          ChatMessage(
            id: 'a1',
            role: 'assistant',
            content: 'The capital of France',
            status: MessageStatus.stopped,
          ),
        ),
      );

      expect(find.byKey(const Key('stopped_label')), findsOneWidget);
      expect(find.text('Stopped'), findsOneWidget);
      expect(find.byKey(const Key('copy_action')), findsOneWidget);
      expect(find.byKey(const Key('stop_action')), findsNothing);
    });

    testWidgets('an ordinary assistant message shows no Stopped label', (
      tester,
    ) async {
      await tester.pumpWidget(_wrap(_assistant(content: 'done')));

      expect(find.byKey(const Key('stopped_label')), findsNothing);
      expect(find.byKey(const Key('copy_action')), findsOneWidget);
    });
  });
}
