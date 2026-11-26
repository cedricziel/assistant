// Widget tests for [QueuedMessageBubble]. Pins:
//   - rendering the message text and the "Queued" badge
//   - long-press and secondary tap both call onLongPress
//   - the element never opens anything itself
//
// Specified by: openspec/changes/chat-stream-progress-ux/specs/.../
//   chat-message-queue/spec.md

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Future<void> _pumpBubble(
  WidgetTester tester, {
  required PendingMessage message,
  required VoidCallback onLongPress,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: QueuedMessageBubble(message: message, onLongPress: onLongPress),
      ),
    ),
  );
}

void main() {
  const message = PendingMessage(text: 'hello world', conversationId: 'c1');

  group('QueuedMessageBubble', () {
    testWidgets('renders the message text and a "Queued" badge', (
      tester,
    ) async {
      await _pumpBubble(tester, message: message, onLongPress: () {});

      expect(find.text('hello world'), findsOneWidget);
      expect(find.byKey(const Key('queued_message_bubble_text')), findsOne);
      expect(find.text('Queued'), findsOneWidget);
    });

    testWidgets('long-press calls onLongPress once', (tester) async {
      var calls = 0;
      await _pumpBubble(tester, message: message, onLongPress: () => calls++);

      await tester.longPress(find.byType(QueuedMessageBubble));
      await tester.pump();

      expect(calls, 1);
    });

    testWidgets('secondary tap calls onLongPress once', (tester) async {
      var calls = 0;
      await _pumpBubble(tester, message: message, onLongPress: () => calls++);

      await tester.tap(
        find.byType(QueuedMessageBubble),
        buttons: kSecondaryButton,
      );
      await tester.pump();

      expect(calls, 1);
    });

    testWidgets('a plain tap does nothing', (tester) async {
      var calls = 0;
      await _pumpBubble(tester, message: message, onLongPress: () => calls++);

      await tester.tap(find.byType(QueuedMessageBubble));
      await tester.pump();

      expect(calls, 0);
    });
  });
}
