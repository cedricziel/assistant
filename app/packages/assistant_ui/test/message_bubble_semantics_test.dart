import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/legacy_bridge.dart';

ChatMessage _user(String content, {MessageStatus status = MessageStatus.ok}) =>
    ChatMessage(id: 'u', role: 'user', content: content, status: status);

ChatMessage _assistant(
  String content, {
  bool isStreaming = false,
  MessageStatus status = MessageStatus.ok,
}) => ChatMessage(
  id: 'a',
  role: 'assistant',
  content: content,
  isStreaming: isStreaming,
  status: status,
);

Future<void> _pump(
  WidgetTester tester,
  Widget bubble, {
  void Function(String url)? onTapLink,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      builder: (context, child) => withLegacyBridge(child!),
      home: Scaffold(
        body: ChatMarkdownScope(
          onTapLink: onTapLink,
          child: Center(child: bubble),
        ),
      ),
    ),
  );
  await tester.pump();
}

/// Collects the non-empty semantics labels of the tree depth-first, which is
/// the order assistive technology traverses them in.
List<String> _labelsInTraversalOrder(WidgetTester tester) {
  var root = tester.getSemantics(find.byType(MaterialApp));
  while (root.parent != null) {
    root = root.parent!;
  }

  final labels = <String>[];
  void visit(SemanticsNode node) {
    final label = node.label;
    if (label.isNotEmpty) labels.add(label);
    node.visitChildren((child) {
      visit(child);
      return true;
    });
  }

  visit(root);
  return labels;
}

void main() {
  group('MessageBubble sender semantics', () {
    testWidgets('the container label starts with the sender label', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      await _pump(
        tester,
        MessageBubble(senderLabel: 'Ada', message: _assistant('Hello there')),
      );

      expect(find.semantics.byLabel('Ada'), findsOne);
      expect(
        find.semantics.byLabel('Hello there'),
        findsOne,
        reason: 'the message text stays a reachable descendant node',
      );
      handle.dispose();
    });

    testWidgets('a user message is labelled "You"', (tester) async {
      final handle = tester.ensureSemantics();
      await _pump(
        tester,
        MessageBubble(senderLabel: 'You', message: _user('Hi')),
      );

      expect(find.semantics.byLabel('You'), findsOne);
      handle.dispose();
    });
  });

  group('MessageBubble status semantics', () {
    final cases = <String, ({String sender, ChatMessage message})>{
      'You, sending': (
        sender: 'You',
        message: _user('Hi', status: MessageStatus.sending),
      ),
      'You, failed': (
        sender: 'You',
        message: _user('Hi', status: MessageStatus.failed),
      ),
      'Ada, responding': (
        sender: 'Ada',
        message: _assistant('', isStreaming: true),
      ),
      'Ada, stopped': (
        sender: 'Ada',
        message: _assistant('Half an ans', status: MessageStatus.stopped),
      ),
    };

    for (final entry in cases.entries) {
      testWidgets('the container label is "${entry.key}"', (tester) async {
        final handle = tester.ensureSemantics();
        await _pump(
          tester,
          MessageBubble(
            senderLabel: entry.value.sender,
            message: entry.value.message,
          ),
        );

        expect(find.semantics.byLabel(entry.key), findsOne);
        handle.dispose();
      });
    }
  });

  group('MessageBubble descendant semantics', () {
    testWidgets('a link is its own node with the link flag', (tester) async {
      final handle = tester.ensureSemantics();
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'Ada',
          message: _assistant('See [Flutter](https://flutter.dev).'),
        ),
        onTapLink: (_) {},
      );

      final links = find.semantics
          .byFlag(SemanticsFlag.isLink)
          .evaluate()
          .toList();
      expect(links, hasLength(1));
      expect(links.single.label, contains('https://flutter.dev'));
      expect(
        find.semantics.byLabel('Ada'),
        findsOne,
        reason: 'the link is not merged into the container label',
      );
      handle.dispose();
    });

    testWidgets('the typing indicator is labelled "typing"', (tester) async {
      final handle = tester.ensureSemantics();
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'Ada',
          message: _assistant('', isStreaming: true),
        ),
      );

      expect(find.semantics.byLabel('typing'), findsOne);
      handle.dispose();
    });

    testWidgets('MessageActions follows the bubble in traversal order', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      await _pump(
        tester,
        MessageBubble(
          senderLabel: 'Ada',
          message: _assistant('Answer'),
          actions: MessageActions(
            actions: [
              MessageActionButton(
                icon: Icons.copy,
                label: 'Copy answer',
                onPressed: () {},
              ),
            ],
          ),
        ),
      );

      final labels = _labelsInTraversalOrder(tester);
      expect(
        labels.indexOf('Answer'),
        lessThan(labels.indexOf('Copy answer')),
        reason: 'actions come after the bubble content',
      );
      handle.dispose();
    });
  });
}
