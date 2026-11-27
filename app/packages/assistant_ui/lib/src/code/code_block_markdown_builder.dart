import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';
import 'package:material_ui/material_ui.dart';

import 'code_block.dart';

/// Renders markdown `code_block` nodes through [CodeBlock].
///
/// Registering this under `code_block` overrides
/// `flutter_smooth_markdown`'s enhanced code builder while leaving the other
/// enhanced builders (blockquote, link, header) in place.
class CodeBlockMarkdownBuilder extends MarkdownWidgetBuilder {
  const CodeBlockMarkdownBuilder();

  @override
  bool canBuild(MarkdownNode node) => node is CodeBlockNode;

  @override
  Widget build(
    MarkdownNode node,
    MarkdownStyleSheet styleSheet,
    MarkdownRenderContext context,
  ) {
    if (node is! CodeBlockNode) return const SizedBox.shrink();
    return CodeBlock(code: node.code, language: node.language);
  }
}

/// A registry containing [scoped]'s builders plus the package's `code_block`
/// builder.
///
/// The scope's registry is copied, never mutated, so a body does not leak the
/// override into other markdown consumers.
BuilderRegistry markdownBuildersWithCodeBlock(BuilderRegistry? scoped) {
  final registry = BuilderRegistry();
  if (scoped != null) {
    for (final entry in scoped.entries) {
      registry.register(entry.key, entry.value);
    }
  }
  registry.register('code_block', const CodeBlockMarkdownBuilder());
  return registry;
}
