@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/golden.dart';
import 'support/scoped_markdown.dart';

const _markdown =
    'A [link](https://example.com) and an image:\n\n'
    '![company logo](https://example.com/logo.png)\n\n'
    '```mermaid\ngraph TD\n  A-->B\n```';

/// What an app registers: a mermaid builder and a stub image builder.
Widget _appScope(Widget child) => ChatMarkdownScope(
  plugins: ParserPluginRegistry()..registerBlock(const MermaidPlugin()),
  builders: BuilderRegistry()..register('mermaid', const _StubMermaidBuilder()),
  imageBuilder: (url, alt, title) => Container(
    width: 120,
    height: 48,
    color: const Color(0xFF9E9E9E),
    alignment: Alignment.center,
    child: Text('image: ${alt ?? ''}'),
  ),
  onTapLink: (_) {},
  child: child,
);

void main() {
  final cases = <String, ({Widget widget, Size size})>{
    // No scope: the image is its alt text and mermaid stays a code block.
    'chat_markdown_scope_none': (
      widget: const Padding(
        padding: EdgeInsets.all(16),
        child: ScopedMarkdownSample(_markdown),
      ),
      size: const Size(420, 260),
    ),
    // An app scope: the image and mermaid builders render in place.
    'chat_markdown_scope_app': (
      widget: _appScope(
        const Padding(
          padding: EdgeInsets.all(16),
          child: ScopedMarkdownSample(_markdown),
        ),
      ),
      size: const Size(420, 260),
    ),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          entry.value.widget,
          size: entry.value.size,
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 250));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}

class _StubMermaidBuilder extends MarkdownWidgetBuilder {
  const _StubMermaidBuilder();

  @override
  bool canBuild(MarkdownNode node) => node is MermaidDiagramNode;

  @override
  Widget build(
    MarkdownNode node,
    MarkdownStyleSheet styleSheet,
    MarkdownRenderContext context,
  ) => Container(
    height: 48,
    color: const Color(0xFFBDBDBD),
    alignment: Alignment.center,
    child: const Text('mermaid diagram'),
  );
}
