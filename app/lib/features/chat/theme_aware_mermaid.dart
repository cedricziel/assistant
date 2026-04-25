import 'package:flutter/material.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';

/// Derives a [MermaidStyle] from a Flutter [ColorScheme] so that Mermaid
/// diagrams use the same tinted surfaces, accent colors, and contrast
/// ratios as the rest of the app — instead of the library's hardcoded
/// light/dark palettes.
MermaidStyle mermaidStyleFromColorScheme(ColorScheme cs) {
  final isDark = cs.brightness == Brightness.dark;

  return MermaidStyle(
    backgroundColor: cs.surfaceContainerHighest.toARGB32(),
    defaultNodeStyle: NodeStyle(
      fillColor: cs.primaryContainer.toARGB32(),
      strokeColor: cs.primary.toARGB32(),
      textColor: cs.onPrimaryContainer.toARGB32(),
    ),
    defaultEdgeStyle: EdgeStyle(
      strokeColor: cs.outline.toARGB32(),
      labelColor: cs.onSurfaceVariant.toARGB32(),
      labelBackgroundColor: cs.surfaceContainerHighest.toARGB32(),
    ),
    themeMode: isDark ? MermaidThemeMode.dark : MermaidThemeMode.light,
  );
}

/// A [MarkdownWidgetBuilder] that renders Mermaid diagrams using a
/// [MermaidStyle] derived from the app's [ColorScheme], so diagrams
/// blend with the surrounding UI instead of using hardcoded palettes.
class ThemeAwareMermaidBuilder extends MarkdownWidgetBuilder {
  const ThemeAwareMermaidBuilder({required this.style});

  final MermaidStyle style;

  @override
  bool canBuild(MarkdownNode node) => node is MermaidDiagramNode;

  @override
  Widget build(
    MarkdownNode node,
    MarkdownStyleSheet styleSheet,
    MarkdownRenderContext context,
  ) {
    if (node is! MermaidDiagramNode) return const SizedBox.shrink();

    return Container(
      margin: const EdgeInsets.symmetric(vertical: 16),
      decoration: BoxDecoration(
        color: Color(style.backgroundColor),
        borderRadius: BorderRadius.circular(8),
      ),
      clipBehavior: Clip.antiAlias,
      child: MermaidDiagram(code: node.code, style: style),
    );
  }
}
