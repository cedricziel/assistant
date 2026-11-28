import 'dart:async';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Future<void> _pump(
  WidgetTester tester,
  Widget body, {
  ChatMarkdownScope Function(Widget child)? scope,
}) async {
  await tester.pumpWidget(
    MaterialApp(home: Scaffold(body: scope == null ? body : scope(body))),
  );
  await tester.pump();
}

Future<void> _expand(WidgetTester tester) async {
  await tester.tap(find.byKey(TimelineEntryShell.headerKey));
  await tester.pump(const Duration(milliseconds: 300));
}

void main() {
  group('ThinkingBody', () {
    testWidgets('active with a stream renders StreamMarkdown labelled '
        '"Thinking..."', (tester) async {
      final controller = StreamController<String>();
      addTearDown(controller.close);

      await _pump(
        tester,
        ThinkingBody(
          entryState: EntryState.active,
          tokenStream: controller.stream,
        ),
      );
      controller
        ..add('first chunk, ')
        ..add('second chunk');
      await tester.pump(const Duration(milliseconds: 300));

      expect(find.text('Thinking...'), findsOneWidget);
      expect(find.byType(StreamMarkdown), findsOneWidget);
    });

    testWidgets('active stream delegates link taps to the scope', (
      tester,
    ) async {
      final controller = StreamController<String>();
      addTearDown(controller.close);
      final taps = <String>[];

      await _pump(
        tester,
        ThinkingBody(
          entryState: EntryState.active,
          tokenStream: controller.stream,
        ),
        scope: (child) => ChatMarkdownScope(onTapLink: taps.add, child: child),
      );
      controller.add('[docs](https://example.com/docs)');
      await tester.pump(const Duration(milliseconds: 300));

      await tester.tap(find.text('docs', findRichText: true));
      await tester.pump();

      expect(taps, ['https://example.com/docs']);
    });

    testWidgets('active stream applies no plugins or scope builders, but adds '
        'the code-block builder', (tester) async {
      final controller = StreamController<String>();
      addTearDown(controller.close);

      await _pump(
        tester,
        ThinkingBody(
          entryState: EntryState.active,
          tokenStream: controller.stream,
        ),
        scope: (child) => ChatMarkdownScope(
          plugins: ParserPluginRegistry()..registerBlock(const MermaidPlugin()),
          builders: BuilderRegistry()
            ..register('mermaid', const _TestMermaidBuilder()),
          child: child,
        ),
      );
      controller.add('```mermaid\ngraph TD\n  A-->B\n```');
      await tester.pump(const Duration(milliseconds: 300));

      final markdown = tester.widget<StreamMarkdown>(
        find.byType(StreamMarkdown),
      );
      expect(markdown.plugins, isNull);
      expect(markdown.builderRegistry!.entries.map((entry) => entry.key), [
        'code_block',
      ]);
      expect(find.text('test-mermaid'), findsNothing);
    });

    testWidgets('an active stream renders a code fence as a CodeBlock', (
      tester,
    ) async {
      final controller = StreamController<String>();
      addTearDown(controller.close);

      await _pump(
        tester,
        ThinkingBody(
          entryState: EntryState.active,
          tokenStream: controller.stream,
        ),
      );
      controller.add('```python\nprint(1)\n```');
      await tester.pump(const Duration(milliseconds: 300));

      expect(find.byType(CodeBlock), findsOneWidget);
    });

    testWidgets('complete renders bold markers literally as selectable text', (
      tester,
    ) async {
      await _pump(
        tester,
        const ThinkingBody(
          entryState: EntryState.complete,
          content: 'I weighed **bold** options',
        ),
      );
      await _expand(tester);

      expect(find.text('Thought'), findsOneWidget);
      expect(find.byIcon(Icons.expand_less_rounded), findsOneWidget);
      expect(find.byType(SelectableText), findsOneWidget);
      expect(find.text('I weighed **bold** options'), findsOneWidget);
      expect(find.byType(StreamMarkdown), findsNothing);
    });

    testWidgets('empty content is not expandable', (tester) async {
      await _pump(
        tester,
        const ThinkingBody(entryState: EntryState.complete, content: ''),
      );

      expect(find.byType(SelectableText), findsNothing);
      await _expand(tester);
      expect(find.byType(SelectableText), findsNothing);
      expect(find.byIcon(Icons.expand_more_rounded), findsNothing);
      expect(find.byIcon(Icons.expand_less_rounded), findsNothing);
    });

    testWidgets('elapsed reads "Thought for 4s"', (tester) async {
      await _pump(
        tester,
        const ThinkingBody(
          entryState: EntryState.complete,
          content: 'done',
          elapsed: Duration(seconds: 4),
        ),
      );

      expect(find.text('Thought for 4s'), findsOneWidget);
    });

    testWidgets('no elapsed reads "Thought"', (tester) async {
      await _pump(
        tester,
        const ThinkingBody(entryState: EntryState.complete, content: 'done'),
      );

      expect(find.text('Thought'), findsOneWidget);
    });

    testWidgets('chunks render as one continuous body', (tester) async {
      final controller = StreamController<String>();
      addTearDown(controller.close);

      await _pump(
        tester,
        ThinkingBody(
          entryState: EntryState.active,
          tokenStream: controller.stream,
        ),
      );
      controller
        ..add('alpha ')
        ..add('beta ')
        ..add('gamma');
      await tester.pump(const Duration(milliseconds: 300));

      expect(find.byType(StreamMarkdown), findsOneWidget);
      expect(find.byType(Divider), findsNothing);
      expect(
        find.textContaining('alpha beta gamma', findRichText: true),
        findsOneWidget,
      );
    });
  });
}

class _TestMermaidBuilder extends MarkdownWidgetBuilder {
  const _TestMermaidBuilder();

  @override
  bool canBuild(MarkdownNode node) => node is MermaidDiagramNode;

  @override
  Widget build(
    MarkdownNode node,
    MarkdownStyleSheet styleSheet,
    MarkdownRenderContext context,
  ) => const Text('test-mermaid');
}
