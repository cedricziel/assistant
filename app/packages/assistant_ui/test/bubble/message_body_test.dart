import 'dart:async';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/src/highlight/code_highlighter.dart';
import 'package:assistant_ui/src/highlight/shared_code_highlighter.dart';
import 'package:assistant_ui/src/highlight/syntax_tokenizer.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/legacy_bridge.dart';

Future<void> _pump(
  WidgetTester tester,
  Widget body, {
  ChatMarkdownScope Function(Widget child)? scope,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      builder: (context, child) => withLegacyBridge(child!),
      home: Scaffold(body: scope == null ? body : scope(body)),
    ),
  );
  await tester.pump();
}

void main() {
  group('MessageBody', () {
    testWidgets('static content renders through selectable SmoothMarkdown', (
      tester,
    ) async {
      await _pump(tester, const MessageBody(content: 'Hello **world**'));

      final markdown = tester.widget<SmoothMarkdown>(
        find.byType(SmoothMarkdown),
      );
      expect(markdown.selectable, isTrue);
      expect(find.byType(StreamMarkdown), findsNothing);
    });

    testWidgets('a streaming body with a stream renders StreamMarkdown', (
      tester,
    ) async {
      final controller = StreamController<String>();
      addTearDown(controller.close);

      await _pump(
        tester,
        MessageBody(
          content: '',
          isStreaming: true,
          tokenStream: controller.stream,
        ),
      );
      controller.add('streamed text');
      await tester.pump(const Duration(milliseconds: 300));

      expect(find.byType(StreamMarkdown), findsOneWidget);
      // 0.11 renders retained stream blocks directly instead of wrapping a
      // SmoothMarkdown, so only the streamed text is observable.
      expect(
        find.textContaining('streamed text', findRichText: true),
        findsOneWidget,
      );
    });

    testWidgets('empty content renders nothing', (tester) async {
      await _pump(tester, const MessageBody(content: ''));

      expect(find.byType(SmoothMarkdown), findsNothing);
      expect(find.byType(StreamMarkdown), findsNothing);
    });

    testWidgets('a markdown image without a scope builds no network image', (
      tester,
    ) async {
      await _pump(
        tester,
        const MessageBody(content: '![logo](https://example.com/logo.png)'),
      );

      expect(find.text('logo'), findsOneWidget);
      expect(find.byType(Image), findsNothing);
    });

    testWidgets('the scope image builder renders markdown images', (
      tester,
    ) async {
      await _pump(
        tester,
        const MessageBody(content: '![logo](https://example.com/logo.png)'),
        scope: (child) => ChatMarkdownScope(
          imageBuilder: (url, alt, title) =>
              SizedBox(key: const Key('scoped_image'), child: Text(url)),
          child: child,
        ),
      );

      expect(find.byKey(const Key('scoped_image')), findsOneWidget);
    });

    testWidgets('the scope builder registry renders plugin blocks', (
      tester,
    ) async {
      await _pump(
        tester,
        const MessageBody(content: '```mermaid\ngraph TD; A-->B\n```'),
        scope: (child) => ChatMarkdownScope(
          plugins: ParserPluginRegistry()..registerBlock(const MermaidPlugin()),
          builders: BuilderRegistry()
            ..register('mermaid', const _MermaidBuilder()),
          child: child,
        ),
      );

      expect(find.byKey(const Key('test_mermaid')), findsOneWidget);
    });

    testWidgets('link taps go to the scope callback', (tester) async {
      String? tapped;
      await _pump(
        tester,
        const MessageBody(content: '[docs](https://example.com/docs)'),
        scope: (child) =>
            ChatMarkdownScope(onTapLink: (url) => tapped = url, child: child),
      );

      final markdown = tester.widget<SmoothMarkdown>(
        find.byType(SmoothMarkdown),
      );
      markdown.onTapLink!('https://example.com/docs');

      expect(tapped, 'https://example.com/docs');
    });

    testWidgets('the scope style sheet override wins', (tester) async {
      final sheet = MarkdownStyleSheet(
        paragraphStyle: const TextStyle(color: Color(0xFF123456)),
      );
      await _pump(
        tester,
        const MessageBody(content: 'text'),
        scope: (child) =>
            ChatMarkdownScope(styleSheetOverride: sheet, child: child),
      );

      final markdown = tester.widget<SmoothMarkdown>(
        find.byType(SmoothMarkdown),
      );
      expect(markdown.styleSheet, same(sheet));
    });

    testWidgets('a finished python fence renders one CodeBlock', (
      tester,
    ) async {
      await _pump(
        tester,
        const MessageBody(content: 'Run:\n\n```python\nprint(1)\n```'),
      );

      final block = tester.widget<CodeBlock>(find.byType(CodeBlock));
      expect(find.byType(CodeBlock), findsOneWidget);
      expect(block.code, 'print(1)');
      expect(block.language, 'python');
    });

    testWidgets('a streaming python fence renders one CodeBlock', (
      tester,
    ) async {
      final controller = StreamController<String>();
      addTearDown(controller.close);

      await _pump(
        tester,
        MessageBody(
          content: '',
          isStreaming: true,
          tokenStream: controller.stream,
        ),
      );
      controller.add('Run:\n\n```python\nprint(1)\n```');
      await tester.pump(const Duration(milliseconds: 300));

      final block = tester.widget<CodeBlock>(find.byType(CodeBlock));
      expect(find.byType(CodeBlock), findsOneWidget);
      expect(block.language, 'python');
    });

    testWidgets('a mermaid fence renders through the registered builder', (
      tester,
    ) async {
      await _pump(
        tester,
        const MessageBody(content: '```mermaid\ngraph TD; A-->B\n```'),
        scope: (child) => ChatMarkdownScope(
          plugins: ParserPluginRegistry()..registerBlock(const MermaidPlugin()),
          builders: BuilderRegistry()
            ..register('mermaid', const _MermaidBuilder()),
          child: child,
        ),
      );

      expect(find.byKey(const Key('test_mermaid')), findsOneWidget);
      expect(find.byType(CodeBlock), findsNothing);
    });

    testWidgets('a finished fence is served from the memo as the next chunk '
        'arrives', (tester) async {
      final spy = _SpyTokenizer();
      final previous = sharedCodeHighlighter;
      sharedCodeHighlighter = CodeHighlighter(tokenizer: spy);
      addTearDown(() => sharedCodeHighlighter = previous);

      final controller = StreamController<String>();
      addTearDown(controller.close);

      await _pump(
        tester,
        MessageBody(
          content: '',
          isStreaming: true,
          tokenStream: controller.stream,
        ),
      );
      controller.add('```dart\nvoid main() {}\n```\n\n');
      await tester.pump(const Duration(milliseconds: 60));
      expect(spy.calls, contains(('dart', 'void main() {}')));

      spy.calls.clear();
      controller.add('```rust\n');
      await tester.pump(const Duration(milliseconds: 60));
      spy.calls.clear();

      for (final chunk in ['let a = 1;\n', 'let b = 2;\n', 'let c = 3;\n']) {
        controller.add(chunk);
        await tester.pump(const Duration(milliseconds: 60));

        expect(
          spy.calls.every((call) => call.$1 == 'rust'),
          isTrue,
          reason: 'the finished dart fence is served from the memo',
        );
        spy.calls.clear();
      }
    });
  });
}

/// Records every call and tokenises to one token per call.
class _SpyTokenizer implements SyntaxTokenizer {
  final calls = <(String, String)>[];

  @override
  bool supports(String language) => language == 'dart' || language == 'rust';

  @override
  List<CodeToken> tokenize(String code, String language) {
    calls.add((language, code));
    return [CodeToken(code, scope: 'keyword')];
  }
}

/// Replaces every `mermaid` block with a recognisable marker.
class _MermaidBuilder extends MarkdownWidgetBuilder {
  const _MermaidBuilder();

  @override
  bool canBuild(MarkdownNode node) => node is MermaidDiagramNode;

  @override
  Widget build(
    MarkdownNode node,
    MarkdownStyleSheet styleSheet,
    MarkdownRenderContext context,
  ) => const SizedBox(key: Key('test_mermaid'));
}
