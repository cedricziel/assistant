import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/scoped_markdown.dart';

const _mermaid = '```mermaid\ngraph TD\n  A-->B\n```';
const _image = '![company logo](https://example.com/logo.png)';

Future<void> _pump(
  WidgetTester tester,
  String markdown, {
  ChatMarkdownScope Function(Widget child)? scope,
}) async {
  final body = ScopedMarkdownSample(markdown);
  await tester.pumpWidget(
    MaterialApp(home: Scaffold(body: scope == null ? body : scope(body))),
  );
  await tester.pump();
}

Set<String> _widgetTypeNames(WidgetTester tester) =>
    tester.allWidgets.map((w) => w.runtimeType.toString()).toSet();

void main() {
  group('ChatMarkdownScope', () {
    testWidgets('a registered mermaid builder renders the block', (
      tester,
    ) async {
      await _pump(
        tester,
        _mermaid,
        scope: (child) => ChatMarkdownScope(
          plugins: ParserPluginRegistry()..registerBlock(const MermaidPlugin()),
          builders: BuilderRegistry()
            ..register('mermaid', const _TestMermaidBuilder()),
          child: child,
        ),
      );

      expect(find.text('test-mermaid'), findsOneWidget);
    });

    testWidgets('a scope imageBuilder renders in place of the image', (
      tester,
    ) async {
      await _pump(
        tester,
        _image,
        scope: (child) => ChatMarkdownScope(
          imageBuilder: (url, alt, title) => Text('custom image: $url'),
          child: child,
        ),
      );

      expect(
        find.text('custom image: https://example.com/logo.png'),
        findsOneWidget,
      );
    });

    testWidgets('without a scope an image renders its alt text and no network '
        'widget', (tester) async {
      await _pump(tester, _image);

      expect(find.text('company logo'), findsOneWidget);
      // The package cannot depend on the network image packages, so match the
      // widget types by name.
      expect(_widgetTypeNames(tester), isNot(contains('CachedNetworkImage')));
      expect(_widgetTypeNames(tester), isNot(contains('SvgPicture')));
      expect(find.byType(Image), findsNothing);
    });

    testWidgets('onTapLink receives the tapped URL', (tester) async {
      final taps = <String>[];
      await _pump(
        tester,
        '[docs](https://example.com/docs)',
        scope: (child) => ChatMarkdownScope(onTapLink: taps.add, child: child),
      );

      await tester.tap(find.text('docs', findRichText: true));
      await tester.pump();

      expect(taps, ['https://example.com/docs']);
    });

    testWidgets('without a scope a link tap is ignored and does not throw', (
      tester,
    ) async {
      await _pump(tester, '[docs](https://example.com/docs)');

      await tester.tap(find.text('docs', findRichText: true));
      await tester.pump();

      expect(tester.takeException(), isNull);
    });

    testWidgets('exposes the style sheet override', (tester) async {
      final sheet = MarkdownStyleSheet.light();
      late ChatMarkdownScope? found;
      await tester.pumpWidget(
        ChatMarkdownScope(
          styleSheetOverride: sheet,
          child: Builder(
            builder: (context) {
              found = ChatMarkdownScope.of(context);
              return const SizedBox.shrink();
            },
          ),
        ),
      );

      expect(found?.styleSheetOverride, same(sheet));
    });

    testWidgets('of returns null without an ancestor', (tester) async {
      late ChatMarkdownScope? found;
      await tester.pumpWidget(
        Builder(
          builder: (context) {
            found = ChatMarkdownScope.of(context);
            return const SizedBox.shrink();
          },
        ),
      );

      expect(found, isNull);
    });
  });
}

/// Replaces every `mermaid` block with a recognisable marker.
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
