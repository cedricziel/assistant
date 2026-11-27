import 'dart:async';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/legacy_bridge.dart';

ChatMessage _user(String content, {MessageStatus status = MessageStatus.ok}) =>
    ChatMessage(id: 'u', role: 'user', content: content, status: status);

ChatMessage _assistant(
  String content, {
  bool isStreaming = false,
  Stream<String>? tokenStream,
}) => ChatMessage(
  id: 'a',
  role: 'assistant',
  content: content,
  isStreaming: isStreaming,
  tokenStream: tokenStream,
);

Future<void> _pump(WidgetTester tester, Widget bubble) async {
  await tester.pumpWidget(
    MaterialApp(
      builder: (context, child) => withLegacyBridge(child!),
      home: Scaffold(body: Center(child: bubble)),
    ),
  );
  await tester.pump();
}

const _surface = Key('message_bubble_surface');

Container _surfaceOf(WidgetTester tester) =>
    tester.widget<Container>(find.byKey(_surface));

Color? _surfaceColor(WidgetTester tester) =>
    ((_surfaceOf(tester).decoration) as BoxDecoration).color;

Widget _marker(String key) => SizedBox(key: Key(key), width: 20, height: 20);

void main() {
  group('MessageBubble alignment and colours', () {
    testWidgets('a user message aligns right on the primary colour', (
      tester,
    ) async {
      await _pump(
        tester,
        MessageBubble(senderLabel: 'You', message: _user('Hi')),
      );

      final align = tester.widget<Align>(
        find
            .ancestor(of: find.byKey(_surface), matching: find.byType(Align))
            .first,
      );
      expect(align.alignment, Alignment.centerRight);
      final scheme = Theme.of(tester.element(find.byKey(_surface))).colorScheme;
      expect(_surfaceColor(tester), scheme.primary);
    });

    testWidgets('an assistant message aligns left on the surface colour', (
      tester,
    ) async {
      await _pump(
        tester,
        MessageBubble(senderLabel: 'Ada', message: _assistant('Hi')),
      );

      final align = tester.widget<Align>(
        find
            .ancestor(of: find.byKey(_surface), matching: find.byType(Align))
            .first,
      );
      expect(align.alignment, Alignment.centerLeft);
      final scheme = Theme.of(tester.element(find.byKey(_surface))).colorScheme;
      expect(_surfaceColor(tester), scheme.surfaceContainerHighest);
    });

    testWidgets('a grouped bubble has a smaller top margin', (tester) async {
      await _pump(
        tester,
        MessageBubble(senderLabel: 'Ada', message: _assistant('Hi')),
      );
      expect(
        (_surfaceOf(tester).margin! as EdgeInsets).top,
        8,
        reason: 'ungrouped',
      );

      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'Ada',
          message: _assistant('Hi'),
          isGrouped: true,
        ),
      );
      expect(
        (_surfaceOf(tester).margin! as EdgeInsets).top,
        2,
        reason: 'grouped',
      );
    });
  });

  group('MessageBubble failed state', () {
    testWidgets('a failed user message shows the error icon and border', (
      tester,
    ) async {
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'You',
          message: _user('x', status: MessageStatus.failed),
        ),
      );

      expect(find.byIcon(Icons.error_outline), findsOneWidget);
      final decoration = _surfaceOf(tester).decoration! as BoxDecoration;
      expect(decoration.border, isNotNull);
    });

    testWidgets('an ok user message has no error icon or border', (
      tester,
    ) async {
      await _pump(
        tester,
        MessageBubble(senderLabel: 'You', message: _user('x')),
      );

      expect(find.byIcon(Icons.error_outline), findsNothing);
      final decoration = _surfaceOf(tester).decoration! as BoxDecoration;
      expect(decoration.border, isNull);
    });
  });

  group('MessageBubble typing indicator', () {
    testWidgets('shows while streaming with empty content', (tester) async {
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'Ada',
          message: _assistant('', isStreaming: true),
        ),
      );

      expect(find.byType(TypingIndicator), findsOneWidget);
    });

    testWidgets('is gone once content arrives', (tester) async {
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'Ada',
          message: _assistant('Hello', isStreaming: true),
        ),
      );

      expect(find.byType(TypingIndicator), findsNothing);
    });

    testWidgets('is not shown for a finished empty message', (tester) async {
      await _pump(
        tester,
        MessageBubble(senderLabel: 'Ada', message: _assistant('')),
      );

      expect(find.byType(TypingIndicator), findsNothing);
    });
  });

  group('MessageBubble user slots', () {
    testWidgets('attachments render above the text', (tester) async {
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'You',
          message: _user('See attached'),
          attachments: _marker('attachments'),
        ),
      );

      final slotY = tester.getTopLeft(find.byKey(const Key('attachments'))).dy;
      final textY = tester.getTopLeft(find.text('See attached')).dy;
      expect(slotY, lessThan(textY));
    });

    testWidgets('audio replaces the text row', (tester) async {
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'You',
          message: _user('Transcript'),
          audio: _marker('audio'),
        ),
      );

      expect(find.byKey(const Key('audio')), findsOneWidget);
      expect(find.text('Transcript'), findsNothing);
    });

    testWidgets('whitespace-only content renders no text row', (tester) async {
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'You',
          message: _user('  \n '),
          attachments: _marker('attachments'),
        ),
      );

      expect(find.byType(SelectableText), findsNothing);
      expect(find.byKey(const Key('attachments')), findsOneWidget);
    });

    testWidgets('user text is plain, not markdown', (tester) async {
      await _pump(
        tester,
        MessageBubble(senderLabel: 'You', message: _user('**bold**')),
      );

      expect(find.byType(SmoothMarkdown), findsNothing);
      expect(find.text('**bold**'), findsOneWidget);
    });

    testWidgets('contextMenuBuilder is applied to the text', (tester) async {
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'You',
          message: _user('Hello'),
          contextMenuBuilder: (context, state) => const SizedBox(),
        ),
      );

      final text = tester.widget<SelectableText>(find.byType(SelectableText));
      expect(text.contextMenuBuilder, isNotNull);
    });
  });

  group('MessageBubble assistant slots', () {
    testWidgets('content renders as markdown through MessageBody', (
      tester,
    ) async {
      await _pump(
        tester,
        MessageBubble(senderLabel: 'Ada', message: _assistant('**bold**')),
      );

      expect(find.byType(MessageBody), findsOneWidget);
      expect(find.byType(SmoothMarkdown), findsOneWidget);
    });

    testWidgets('attachments then audio render below the content', (
      tester,
    ) async {
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'Ada',
          message: _assistant('Here is the chart.'),
          attachments: _marker('attachments'),
          audio: _marker('audio'),
        ),
      );

      final contentY = tester.getTopLeft(find.byType(MessageBody)).dy;
      final attachmentsY = tester
          .getTopLeft(find.byKey(const Key('attachments')))
          .dy;
      final audioY = tester.getTopLeft(find.byKey(const Key('audio'))).dy;
      expect(contentY, lessThan(attachmentsY));
      expect(attachmentsY, lessThan(audioY));
    });

    testWidgets('extensions render inside the bubble below the content', (
      tester,
    ) async {
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'Ada',
          message: _assistant('Answer'),
          extensions: [_marker('extension')],
        ),
      );

      expect(
        find.descendant(
          of: find.byKey(_surface),
          matching: find.byKey(const Key('extension')),
        ),
        findsOneWidget,
      );
      final contentY = tester.getTopLeft(find.byType(MessageBody)).dy;
      final extensionY = tester
          .getTopLeft(find.byKey(const Key('extension')))
          .dy;
      expect(contentY, lessThan(extensionY));
    });

    testWidgets('the streaming message renders through StreamMarkdown', (
      tester,
    ) async {
      final controller = StreamController<String>();
      addTearDown(controller.close);

      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'Ada',
          message: _assistant(
            '',
            isStreaming: true,
            tokenStream: controller.stream,
          ),
        ),
      );
      controller.add('streamed');
      await tester.pump(const Duration(milliseconds: 300));

      expect(find.byType(StreamMarkdown), findsOneWidget);
    });
  });

  group('MessageBubble actions', () {
    testWidgets('actions render below the bubble, outside the decoration', (
      tester,
    ) async {
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'Ada',
          message: _assistant('Answer'),
          actions: _marker('actions'),
        ),
      );

      expect(
        find.descendant(
          of: find.byKey(_surface),
          matching: find.byKey(const Key('actions')),
        ),
        findsNothing,
      );
      final bubbleBottom = tester.getBottomLeft(find.byKey(_surface)).dy;
      final actionsY = tester.getTopLeft(find.byKey(const Key('actions'))).dy;
      expect(actionsY, greaterThanOrEqualTo(bubbleBottom));
    });
  });

  group('MessageBubble media', () {
    testWidgets('does not itself fetch or build any network image', (
      tester,
    ) async {
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'Ada',
          message: _assistant('![logo](https://example.com/logo.png)'),
        ),
      );

      expect(find.byType(Image), findsNothing);
      expect(find.text('logo'), findsOneWidget);
    });
  });
}
