import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';
import 'package:material_ui/material_ui.dart';

/// A minimal markdown body wired to the nearest [ChatMarkdownScope], the way a
/// real message body is: plugins, builders, image builder and link taps all
/// come from the scope.
class ScopedMarkdownSample extends StatelessWidget {
  const ScopedMarkdownSample(this.data, {super.key});

  final String data;

  @override
  Widget build(BuildContext context) {
    final scope = ChatMarkdownScope.of(context);
    return SmoothMarkdown(
      data: data,
      plugins: scope?.plugins,
      builderRegistry: scope?.builders,
      imageBuilder: ChatMarkdownScope.imageBuilderOf(context),
      onTapLink: scope?.onTapLink,
    );
  }
}
