@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

Widget _padded(Widget child) =>
    Padding(padding: const EdgeInsets.all(16), child: child);

void noop(String _) {}

void main() {
  const rust =
      'fn main() {\n'
      '    let greeting = "hello";\n'
      '    println!("{greeting}, world");\n'
      '}\n';
  final longLine = 'let value = ${List.filled(40, '"segment"').join(' + ')};';

  final cases = <String, ({Widget widget, Size size, bool copied})>{
    'code_block_rust': (
      widget: const CodeBlock(code: rust, language: 'rust', onCopy: noop),
      size: const Size(400, 180),
      copied: false,
    ),
    'code_block_plain': (
      widget: const CodeBlock(
        code: 'Just plain text.\nTwo lines.',
        onCopy: noop,
      ),
      size: const Size(400, 140),
      copied: false,
    ),
    'code_block_long_line': (
      widget: CodeBlock(code: longLine, language: 'rust', onCopy: noop),
      size: const Size(360, 110),
      copied: false,
    ),
    'code_block_copied': (
      widget: const CodeBlock(code: rust, language: 'rust', onCopy: noop),
      size: const Size(400, 180),
      copied: true,
    ),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          _padded(entry.value.widget),
          size: entry.value.size,
          brightness: brightness,
        );
        if (entry.value.copied) {
          await tester.tap(find.bySemanticsLabel('Copy code'));
        }
        await tester.pump(const Duration(milliseconds: 50));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
