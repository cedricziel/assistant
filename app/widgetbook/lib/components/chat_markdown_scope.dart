import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

const _markdown =
    'A [link](https://example.com) and an image:\n\n'
    '![company logo](https://example.com/logo.png)\n\n'
    '```mermaid\ngraph TD\n  A-->B\n```';

/// A body that renders markdown the way a message body does: every piece
/// comes from the nearest [ChatMarkdownScope].
class _SampleBody extends StatelessWidget {
  const _SampleBody();

  @override
  Widget build(BuildContext context) {
    final scope = ChatMarkdownScope.of(context);
    return Padding(
      padding: const EdgeInsets.all(16),
      child: SmoothMarkdown(
        data: _markdown,
        plugins: scope?.plugins,
        builderRegistry: scope?.builders,
        imageBuilder: ChatMarkdownScope.imageBuilderOf(context),
        onTapLink: scope?.onTapLink,
      ),
    );
  }
}

final chatMarkdownScopeComponent = WidgetbookComponent(
  name: 'ChatMarkdownScope',
  useCases: [
    WidgetbookUseCase(
      name: 'no scope (placeholder image, links ignored)',
      builder: (context) => const _SampleBody(),
    ),
    WidgetbookUseCase(
      name: 'app image builder and mermaid builder',
      builder: (context) => ChatMarkdownScope(
        plugins: ParserPluginRegistry()..registerBlock(const MermaidPlugin()),
        builders: BuilderRegistry()
          ..register('mermaid', const _StubMermaidBuilder()),
        imageBuilder: (url, alt, title) => Container(
          width: 160,
          height: 64,
          color: Theme.of(context).colorScheme.secondaryContainer,
          alignment: Alignment.center,
          child: Text('image: ${alt ?? ''}'),
        ),
        onTapLink: (url) => debugPrint('link tapped: $url'),
        child: const _SampleBody(),
      ),
    ),
  ],
);

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
    height: 64,
    alignment: Alignment.center,
    color: Colors.black12,
    child: const Text('mermaid diagram (app builder)'),
  );
}
