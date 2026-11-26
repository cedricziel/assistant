// Widget tests for [QueuedMessageBubble] wired to the app's remove sheet. Pins:
//   - long-press surfaces the remove action sheet
//   - tapping "Remove from queue" calls removeFromQueue on the notifier
//
// Specified by: openspec/changes/chat-stream-progress-ux/specs/.../
//   chat-message-queue/spec.md

import 'package:assistant_ui/assistant_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/queued_message_remove_sheet.dart';

class _FakeChatNotifier extends ChatNotifier {
  final List<int> removedIndices = [];

  @override
  Future<ChatState> build() async => const ChatState();

  @override
  void removeFromQueue(int index) {
    removedIndices.add(index);
  }

  void push(ChatState s) => state = AsyncData(s);
}

Future<_FakeChatNotifier> _pumpBubble(
  WidgetTester tester, {
  required PendingMessage message,
  required int index,
}) async {
  final notifier = _FakeChatNotifier();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [chatProvider.overrideWith(() => notifier)],
      child: MaterialApp(
        home: Scaffold(
          body: Consumer(
            builder: (context, ref, _) => QueuedMessageBubble(
              message: message,
              onLongPress: () =>
                  showQueuedMessageRemoveSheet(context, ref, message, index),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pump();
  return notifier;
}

void main() {
  group('showQueuedMessageRemoveSheet via QueuedMessageBubble', () {
    testWidgets('long-press opens an action sheet with Remove', (tester) async {
      await _pumpBubble(
        tester,
        message: const PendingMessage(text: 'queued', conversationId: 'c1'),
        index: 2,
      );

      await tester.longPress(find.byType(QueuedMessageBubble));
      await tester.pumpAndSettle();

      expect(find.text('Remove from queue'), findsOneWidget);
    });

    testWidgets(
      'tapping Remove from queue calls removeFromQueue with the given index',
      (tester) async {
        final notifier = await _pumpBubble(
          tester,
          message: const PendingMessage(text: 'queued', conversationId: 'c1'),
          index: 3,
        );

        await tester.longPress(find.byType(QueuedMessageBubble));
        await tester.pumpAndSettle();
        await tester.tap(find.text('Remove from queue'));
        await tester.pumpAndSettle();

        expect(notifier.removedIndices, [
          3,
        ], reason: 'remove action must call removeFromQueue(index) once');
      },
    );
  });
}
