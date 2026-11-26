import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

const _rust = '''fn main() {
    let greeting = "hello";
    println!("{greeting}, world");
}
''';

const _plain = 'No grammar applies here.\nJust two lines of plain text.\n';

final _longLine = 'let value = ${List.filled(40, '"segment"').join(' + ')};';

Widget _framed(Widget child) =>
    Padding(padding: const EdgeInsets.all(16), child: child);

final codeBlockComponent = WidgetbookComponent(
  name: 'CodeBlock',
  useCases: [
    WidgetbookUseCase(
      name: 'rust',
      builder: (context) =>
          _framed(CodeBlock(code: _rust, language: 'rust', onCopy: (_) {})),
    ),
    WidgetbookUseCase(
      name: 'plain',
      builder: (context) => _framed(CodeBlock(code: _plain, onCopy: (_) {})),
    ),
    WidgetbookUseCase(
      name: 'long line',
      builder: (context) =>
          _framed(CodeBlock(code: _longLine, language: 'rust', onCopy: (_) {})),
    ),
    WidgetbookUseCase(
      name: 'no copy',
      builder: (context) =>
          _framed(const CodeBlock(code: _rust, language: 'rust')),
    ),
  ],
);
