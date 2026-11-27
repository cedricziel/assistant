import 'dart:typed_data';

import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/features/chat/chat_message_wiring.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/shared/platform/adaptive_app.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

class _FakeChatNotifier extends ChatNotifier {
  final List<(String, String)> edits = [];
  final List<String> regenerates = [];
  final List<String> switched = [];

  ChatState _initial = const ChatState();

  void push(ChatState next) {
    _initial = next;
    state = AsyncData(next);
  }

  @override
  Future<ChatState> build() async => _initial;

  @override
  Future<void> editMessage(
    String messageId,
    String text, {
    String? replaceId,
  }) async {
    edits.add((messageId, text));
  }

  @override
  Future<void> regenerate(String messageId, {String? model}) async {
    regenerates.add(messageId);
  }

  @override
  Future<void> switchBranch(String messageId) async {
    switched.add(messageId);
  }
}

const _userId = '00000000-0000-4000-8000-000000000001';
const _replyId = '00000000-0000-4000-8000-000000000002';

const _attachments = [
  ChatAttachment(
    id: 'f',
    filename: 'chart.png',
    mimeType: 'image/png',
    url: '/chart.png',
  ),
];

ChatMessage _user({
  String id = _userId,
  String content = 'A question',
  List<ChatAttachment>? attachments,
  BranchInfo? replyBranch,
}) => ChatMessage(
  id: id,
  role: 'user',
  content: content,
  attachments: attachments,
  replyBranch: replyBranch,
);

ChatMessage _assistant({String id = _replyId}) =>
    ChatMessage(id: id, role: 'assistant', content: 'A reply');

Future<({_FakeChatNotifier chat, void Function(ChatState) push})> _pump(
  WidgetTester tester,
  ChatMessage message,
) async {
  final chat = _FakeChatNotifier();
  final container = ProviderContainer(
    overrides: [chatProvider.overrideWith(() => chat)],
  );
  addTearDown(container.dispose);
  container.listen(chatProvider, (_, _) {});
  await container.read(chatProvider.future);

  await tester.pumpWidget(
    UncontrolledProviderScope(
      container: container,
      child: MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.topLeft,
            child: ChatMessageActionsWiring(
              message: message,
              senderLabel: message.isUser ? 'You' : 'Ada',
              capabilities: ServerCapabilities.disabled,
              fetchMessageAudio: () async =>
                  (bytes: Uint8List(0), mimeType: ''),
            ),
          ),
        ),
        builder: (_, child) => LegacyDesignBridge(child: child!),
      ),
    ),
  );
  await tester.pump();
  return (chat: chat, push: chat.push);
}

TurnStatusSnapshot _status({String? awaitingLabel}) => TurnStatusSnapshot(
  runId: 'r1',
  lastEventKind: TurnEventKind.runStarted,
  lastEventAt: DateTime(2024),
  awaitingLabel: awaitingLabel,
);

void main() {
  group('ChatMessageActionsWiring', () {
    testWidgets('Edit swaps the bubble for an editor that keeps attachments', (
      tester,
    ) async {
      final harness = await _pump(tester, _user(attachments: _attachments));
      harness.push(ChatState(conversationId: 'c1', messages: const []));
      await tester.pump();

      await tester.tap(find.byKey(const Key('edit_action')));
      await tester.pump();

      expect(find.byType(MessageEditor), findsOneWidget);
      expect(find.byKey(const Key('message_bubble_surface')), findsNothing);
      expect(find.text('chart.png'), findsOneWidget);
      expect(find.text('Remove'), findsNothing);

      await tester.enterText(find.byType(TextField), 'Edited text');
      await tester.pump();
      await tester.tap(find.widgetWithText(FilledButton, 'Send'));
      await tester.pump();

      expect(harness.chat.edits, [(_userId, 'Edited text')]);
      expect(find.byType(MessageEditor), findsNothing);
    });

    testWidgets('Cancel leaves the bubble as it was', (tester) async {
      final harness = await _pump(tester, _user());
      harness.push(const ChatState(conversationId: 'c1'));
      await tester.pump();

      await tester.tap(find.byKey(const Key('edit_action')));
      await tester.pump();
      await tester.tap(find.text('Cancel'));
      await tester.pump();

      expect(find.byType(MessageEditor), findsNothing);
      expect(harness.chat.edits, isEmpty);
      expect(find.byKey(const Key('message_bubble_surface')), findsOneWidget);
    });

    testWidgets('Regenerate calls the notifier', (tester) async {
      final harness = await _pump(tester, _assistant());
      harness.push(const ChatState(conversationId: 'c1'));
      await tester.pump();

      await tester.tap(find.byKey(const Key('regenerate_action')));
      await tester.pump();

      expect(harness.chat.regenerates, [_replyId]);
    });

    testWidgets('a user message renders the answer picker from replyBranch', (
      tester,
    ) async {
      final harness = await _pump(
        tester,
        _user(
          replyBranch: const BranchInfo(
            count: 2,
            index: 0,
            siblingIds: ['r1', 'r2'],
          ),
        ),
      );
      harness.push(const ChatState(conversationId: 'c1'));
      await tester.pump();

      expect(find.byType(BranchPicker), findsOneWidget);
      await tester.tap(find.byTooltip('Next version'));
      await tester.pump();

      expect(harness.chat.switched, ['r2']);
    });

    testWidgets('branches are withheld and pickers busy while a turn works', (
      tester,
    ) async {
      final harness = await _pump(
        tester,
        _user(
          replyBranch: const BranchInfo(
            count: 2,
            index: 0,
            siblingIds: ['r1', 'r2'],
          ),
        ),
      );
      harness.push(ChatState(conversationId: 'c1', isSending: true));
      await tester.pump();

      expect(find.byKey(const Key('edit_action')), findsNothing);
      expect(
        tester.widget<BranchPicker>(find.byType(BranchPicker)).busy,
        isTrue,
      );
    });

    testWidgets('branches stay usable while the turn waits for input', (
      tester,
    ) async {
      final harness = await _pump(
        tester,
        _user(
          replyBranch: const BranchInfo(
            count: 2,
            index: 0,
            siblingIds: ['r1', 'r2'],
          ),
        ),
      );
      harness.push(
        ChatState(
          conversationId: 'c1',
          isSending: true,
          currentTurnStatus: _status(
            awaitingLabel: 'Waiting for your approval',
          ),
        ),
      );
      await tester.pump();

      expect(find.byKey(const Key('edit_action')), findsOneWidget);
      expect(
        tester.widget<BranchPicker>(find.byType(BranchPicker)).busy,
        isFalse,
      );
    });

    testWidgets('a switch in flight keeps the picker busy', (tester) async {
      final harness = await _pump(
        tester,
        _user(
          replyBranch: const BranchInfo(
            count: 2,
            index: 0,
            siblingIds: ['r1', 'r2'],
          ),
        ),
      );
      harness.push(
        const ChatState(conversationId: 'c1', isSwitchingBranch: true),
      );
      await tester.pump();

      expect(
        tester.widget<BranchPicker>(find.byType(BranchPicker)).busy,
        isTrue,
      );
    });

    testWidgets('an optimistic user message offers no Edit', (tester) async {
      final harness = await _pump(tester, _user(id: 'user-1'));
      harness.push(const ChatState(conversationId: 'c1'));
      await tester.pump();

      expect(find.byKey(const Key('edit_action')), findsNothing);
    });

    testWidgets('a stopped reply the server never persisted offers no '
        'Regenerate', (tester) async {
      final harness = await _pump(
        tester,
        _assistant(id: 'assistant-1700000000000'),
      );
      harness.push(const ChatState(conversationId: 'c1'));
      await tester.pump();

      expect(find.byKey(const Key('regenerate_action')), findsNothing);
    });
  });
}
