import 'package:flutter/material.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';
import 'package:flutter_svg/flutter_svg.dart';

// ── AST Node ────────────────────────────────────────────────────────────────

/// AST node for inline SVG graphics.
class SvgNode extends MarkdownNode {
  const SvgNode({required this.code});

  /// Raw SVG markup.
  final String code;

  @override
  String get type => 'svg';

  @override
  Map<String, dynamic> toJson() => {'type': type, 'code': code};

  @override
  SvgNode copyWith({String? code}) => SvgNode(code: code ?? this.code);

  @override
  String toString() =>
      'SvgNode(code: ${code.substring(0, code.length > 30 ? 30 : code.length)}...)';
}

// ── Parser Plugin ───────────────────────────────────────────────────────────

/// Block parser plugin that recognises ````svg` fenced code blocks.
class SvgPlugin extends BlockParserPlugin {
  const SvgPlugin();

  @override
  String get id => 'svg';

  @override
  String get name => 'SVG Plugin';

  @override
  int get priority => 10;

  @override
  bool canParse(String line, List<String> lines, int index) {
    final trimmed = line.trim();
    return trimmed.startsWith('```svg') || trimmed.startsWith('~~~svg');
  }

  @override
  BlockParseResult? parse(List<String> lines, int startIndex) {
    final startLine = lines[startIndex].trim();
    final fenceChar = startLine.startsWith('```') ? '```' : '~~~';

    final codeLines = <String>[];
    var linesConsumed = 1;

    for (var i = startIndex + 1; i < lines.length; i++) {
      final line = lines[i];
      linesConsumed++;
      if (line.trim().startsWith(fenceChar)) break;
      codeLines.add(line);
    }

    if (codeLines.isEmpty) return null;

    final code = codeLines.join('\n').trim();
    return BlockParseResult(
      node: SvgNode(code: code),
      linesConsumed: linesConsumed,
    );
  }
}

// ── Widget Builder ──────────────────────────────────────────────────────────

/// Renders [SvgNode] as an inline SVG graphic via `flutter_svg`.
class SvgBuilder extends MarkdownWidgetBuilder {
  const SvgBuilder();

  @override
  bool canBuild(MarkdownNode node) => node is SvgNode;

  @override
  Widget build(
    MarkdownNode node,
    MarkdownStyleSheet styleSheet,
    MarkdownRenderContext context,
  ) {
    if (node is! SvgNode) return const SizedBox.shrink();

    final sanitized = sanitizeSvg(node.code);

    return Container(
      margin: const EdgeInsets.symmetric(vertical: 16),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.grey.withValues(alpha: 0.3)),
      ),
      clipBehavior: Clip.antiAlias,
      child: SvgPicture.string(
        sanitized,
        fit: BoxFit.contain,
        width: double.infinity,
      ),
    );
  }
}

// ── SVG Sanitization ────────────────────────────────────────────────────────

/// Strip potentially dangerous elements and attributes from SVG markup.
///
/// Removes `<script>`, `<foreignObject>`, `on*` event handlers, and
/// `javascript:` URIs.  Defence-in-depth: `flutter_svg` on native does not
/// execute JS, but the web target may render via HTML.
String sanitizeSvg(String svg) {
  var result = svg;

  // Remove <script>...</script> (including multiline).
  result = result.replaceAll(
    RegExp(r'<script[\s\S]*?</script>', caseSensitive: false),
    '',
  );

  // Remove self-closing <script .../>.
  result = result.replaceAll(
    RegExp(r'<script[^>]*/>', caseSensitive: false),
    '',
  );

  // Remove <foreignObject>...</foreignObject>.
  result = result.replaceAll(
    RegExp(r'<foreignObject[\s\S]*?</foreignObject>', caseSensitive: false),
    '',
  );

  // Remove on* event handler attributes (onclick, onload, etc.).
  result = result.replaceAll(
    RegExp(r'\s+on\w+\s*=\s*"[^"]*"', caseSensitive: false),
    '',
  );
  result = result.replaceAll(
    RegExp(r"\s+on\w+\s*=\s*'[^']*'", caseSensitive: false),
    '',
  );

  // Replace javascript: URIs in href / xlink:href with '#'.
  result = result.replaceAllMapped(
    RegExp(
      r'''((?:xlink:)?href\s*=\s*)(["'])javascript:[^"']*\2''',
      caseSensitive: false,
    ),
    (m) => '${m[1]}${m[2]}#${m[2]}',
  );

  return result;
}
