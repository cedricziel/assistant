import 'package:flutter/material.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/theme_aware_mermaid.dart';

void main() {
  group('mermaidStyleFromColorScheme', () {
    test('uses dark themeMode for dark ColorScheme', () {
      final cs = ColorScheme.fromSeed(
        seedColor: const Color(0xFF1A73E8),
        brightness: Brightness.dark,
      );

      final style = mermaidStyleFromColorScheme(cs);

      expect(style.themeMode, MermaidThemeMode.dark);
    });

    test('uses light themeMode for light ColorScheme', () {
      final cs = ColorScheme.fromSeed(
        seedColor: const Color(0xFF1A73E8),
        brightness: Brightness.light,
      );

      final style = mermaidStyleFromColorScheme(cs);

      expect(style.themeMode, MermaidThemeMode.light);
    });

    test('maps background to surfaceContainerHighest', () {
      final cs = ColorScheme.fromSeed(
        seedColor: const Color(0xFF1A73E8),
        brightness: Brightness.dark,
      );

      final style = mermaidStyleFromColorScheme(cs);

      expect(
        style.backgroundColor,
        cs.surfaceContainerHighest.toARGB32(),
        reason: 'Diagram background should match the bubble background',
      );
    });

    test('maps node fill to primaryContainer', () {
      final cs = ColorScheme.fromSeed(
        seedColor: const Color(0xFF1A73E8),
        brightness: Brightness.dark,
      );

      final style = mermaidStyleFromColorScheme(cs);

      expect(style.defaultNodeStyle.fillColor, cs.primaryContainer.toARGB32());
    });

    test('maps node stroke to primary', () {
      final cs = ColorScheme.fromSeed(
        seedColor: const Color(0xFF1A73E8),
        brightness: Brightness.dark,
      );

      final style = mermaidStyleFromColorScheme(cs);

      expect(style.defaultNodeStyle.strokeColor, cs.primary.toARGB32());
    });

    test('maps node text to onPrimaryContainer', () {
      final cs = ColorScheme.fromSeed(
        seedColor: const Color(0xFF1A73E8),
        brightness: Brightness.dark,
      );

      final style = mermaidStyleFromColorScheme(cs);

      expect(
        style.defaultNodeStyle.textColor,
        cs.onPrimaryContainer.toARGB32(),
      );
    });

    test('maps edge stroke to outline', () {
      final cs = ColorScheme.fromSeed(
        seedColor: const Color(0xFF1A73E8),
        brightness: Brightness.dark,
      );

      final style = mermaidStyleFromColorScheme(cs);

      expect(style.defaultEdgeStyle.strokeColor, cs.outline.toARGB32());
    });

    test('maps edge label color to onSurfaceVariant', () {
      final cs = ColorScheme.fromSeed(
        seedColor: const Color(0xFF1A73E8),
        brightness: Brightness.dark,
      );

      final style = mermaidStyleFromColorScheme(cs);

      expect(style.defaultEdgeStyle.labelColor, cs.onSurfaceVariant.toARGB32());
    });

    test('maps edge label background to surfaceContainerHighest', () {
      final cs = ColorScheme.fromSeed(
        seedColor: const Color(0xFF1A73E8),
        brightness: Brightness.dark,
      );

      final style = mermaidStyleFromColorScheme(cs);

      expect(
        style.defaultEdgeStyle.labelBackgroundColor,
        cs.surfaceContainerHighest.toARGB32(),
      );
    });

    test('produces different styles for light vs dark', () {
      final light = mermaidStyleFromColorScheme(
        ColorScheme.fromSeed(
          seedColor: const Color(0xFF1A73E8),
          brightness: Brightness.light,
        ),
      );
      final dark = mermaidStyleFromColorScheme(
        ColorScheme.fromSeed(
          seedColor: const Color(0xFF1A73E8),
          brightness: Brightness.dark,
        ),
      );

      expect(
        light.backgroundColor,
        isNot(dark.backgroundColor),
        reason: 'Light and dark should have distinct backgrounds',
      );
      expect(
        light.defaultNodeStyle.fillColor,
        isNot(dark.defaultNodeStyle.fillColor),
        reason: 'Light and dark should have distinct node fills',
      );
    });
  });

  group('ThemeAwareMermaidBuilder', () {
    test('canBuild returns true for MermaidDiagramNode', () {
      final style = mermaidStyleFromColorScheme(
        ColorScheme.fromSeed(seedColor: const Color(0xFF1A73E8)),
      );
      final builder = ThemeAwareMermaidBuilder(style: style);

      expect(
        builder.canBuild(const MermaidDiagramNode(code: 'graph TD\nA-->B')),
        isTrue,
      );
    });

    test('canBuild returns false for non-Mermaid nodes', () {
      final style = mermaidStyleFromColorScheme(
        ColorScheme.fromSeed(seedColor: const Color(0xFF1A73E8)),
      );
      final builder = ThemeAwareMermaidBuilder(style: style);

      // Use a generic MarkdownNode subtype that isn't MermaidDiagramNode.
      expect(builder.canBuild(const _FakeNode()), isFalse);
    });

    testWidgets('build produces a MermaidDiagram with the given style', (
      tester,
    ) async {
      final cs = ColorScheme.fromSeed(
        seedColor: const Color(0xFF1A73E8),
        brightness: Brightness.dark,
      );
      final style = mermaidStyleFromColorScheme(cs);
      final builder = ThemeAwareMermaidBuilder(style: style);
      const node = MermaidDiagramNode(code: 'graph TD\n  A-->B');

      final widget = builder.build(
        node,
        MarkdownStyleSheet.dark(),
        const MarkdownRenderContext(),
      );

      // Pump the widget inside a constrained box so LayoutBuilder resolves.
      await tester.pumpWidget(
        MaterialApp(
          theme: ThemeData(colorScheme: cs),
          home: Scaffold(
            body: SizedBox(width: 600, height: 400, child: widget),
          ),
        ),
      );

      // The tree should contain a MermaidDiagram somewhere.
      expect(find.byType(MermaidDiagram), findsOneWidget);
    });
  });
}

/// Minimal fake node for testing canBuild rejects non-Mermaid nodes.
class _FakeNode extends MarkdownNode {
  const _FakeNode();

  @override
  String get type => 'fake';

  @override
  Map<String, dynamic> toJson() => {'type': type};

  @override
  MarkdownNode copyWith() => this;
}
