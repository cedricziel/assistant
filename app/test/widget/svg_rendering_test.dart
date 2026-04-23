import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/svg_builder.dart';

void main() {
  group('sanitizeSvg', () {
    test('strips <script> elements', () {
      const input = '<svg><rect/><script>alert("xss")</script><circle/></svg>';
      final result = sanitizeSvg(input);
      expect(result, isNot(contains('<script')));
      expect(result, contains('<rect/>'));
      expect(result, contains('<circle/>'));
    });

    test('strips multiline <script> blocks', () {
      const input = '<svg>\n<script>\nalert(1);\n</script>\n<rect/></svg>';
      final result = sanitizeSvg(input);
      expect(result, isNot(contains('<script')));
      expect(result, contains('<rect/>'));
    });

    test('strips self-closing <script/> tags', () {
      const input = '<svg><script src="evil.js"/><rect/></svg>';
      final result = sanitizeSvg(input);
      expect(result, isNot(contains('<script')));
    });

    test('strips onclick attributes (double quotes)', () {
      const input = '<svg><rect onclick="alert(1)"/></svg>';
      final result = sanitizeSvg(input);
      expect(result, isNot(contains('onclick')));
      expect(result, contains('<rect'));
    });

    test('strips onload attributes (single quotes)', () {
      const input = "<svg onload='malicious()'><rect/></svg>";
      final result = sanitizeSvg(input);
      expect(result, isNot(contains('onload')));
    });

    test('strips <foreignObject> elements', () {
      const input =
          '<svg><foreignObject><div>html</div></foreignObject><rect/></svg>';
      final result = sanitizeSvg(input);
      expect(result, isNot(contains('foreignObject')));
      expect(result, contains('<rect/>'));
    });

    test('replaces javascript: URIs in href', () {
      const input =
          '<svg><a href="javascript:alert(1)"><text>click</text></a></svg>';
      final result = sanitizeSvg(input);
      expect(result, isNot(contains('javascript:')));
      expect(result, contains('href'));
    });

    test('replaces javascript: URIs in xlink:href', () {
      const input =
          '<svg><a xlink:href="javascript:alert(1)"><text>click</text></a></svg>';
      final result = sanitizeSvg(input);
      expect(result, isNot(contains('javascript:')));
    });

    test('passes clean SVG through unchanged', () {
      const input =
          '<svg viewBox="0 0 100 100">'
          '<rect x="10" y="10" width="80" height="80" fill="blue"/>'
          '</svg>';
      final result = sanitizeSvg(input);
      expect(result, equals(input));
    });
  });

  group('SvgPlugin', () {
    const plugin = SvgPlugin();

    test('detects ```svg fenced blocks', () {
      expect(plugin.canParse('```svg', ['```svg'], 0), isTrue);
    });

    test('detects ~~~svg fenced blocks', () {
      expect(plugin.canParse('~~~svg', ['~~~svg'], 0), isTrue);
    });

    test('does not match other code blocks', () {
      expect(plugin.canParse('```dart', ['```dart'], 0), isFalse);
      expect(plugin.canParse('```mermaid', ['```mermaid'], 0), isFalse);
    });

    test('parses a complete svg block', () {
      final lines = [
        '```svg',
        '<svg viewBox="0 0 100 100">',
        '  <rect width="100" height="100"/>',
        '</svg>',
        '```',
      ];
      final result = plugin.parse(lines, 0);
      expect(result, isNotNull);
      expect(result!.linesConsumed, equals(5));
      final node = result.node as SvgNode;
      expect(node.code, contains('<svg'));
      expect(node.code, contains('<rect'));
    });

    test('returns null for empty block', () {
      final lines = ['```svg', '```'];
      final result = plugin.parse(lines, 0);
      expect(result, isNull);
    });
  });
}
