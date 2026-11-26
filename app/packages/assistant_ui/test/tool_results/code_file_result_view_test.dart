import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'contract_fixtures_test.dart' show fixturesDir, fixturesOf;

Map<String, Object?> _fixture(String name) =>
    fixturesOf(fixturesDir(), 'code_file')[name]!;

Widget _host(Widget child) => MaterialApp(
  home: Scaffold(body: SingleChildScrollView(child: child)),
);

Future<void> _pump(
  WidgetTester tester,
  String fixture, {
  ToolCallStatus status = ToolCallStatus.ok,
}) async {
  const renderer = CodeFileRenderer();
  final model = renderer.parse(_fixture(fixture))!;
  await tester.pumpWidget(
    _host(
      Builder(builder: (context) => renderer.build(context, model, status)),
    ),
  );
}

void main() {
  const renderer = CodeFileRenderer();

  group('CodeFileRenderer.parse', () {
    test('reads a valid payload', () {
      final model = renderer.parse(_fixture('valid-rust.json'))!;

      expect(model.path, 'src/main.rs');
      expect(model.language, 'rust');
      expect(model.content, 'fn main() {}\n');
      expect(model.truncated, isFalse);
      expect(model.totalChars, 13);
    });

    test('language is optional', () {
      final model = renderer.parse(
        _fixture('valid-truncated-no-language.json'),
      )!;

      expect(model.language, isNull);
      expect(model.truncated, isTrue);
    });

    test('rejects a payload of another kind', () {
      expect(
        renderer.parse({..._fixture('valid-rust.json'), 'kind': 'code_diff'}),
        isNull,
      );
    });

    test('rejects a negative total', () {
      expect(
        renderer.parse({..._fixture('valid-rust.json'), 'total_chars': -1}),
        isNull,
      );
    });

    test('rejects a non-string language', () {
      expect(
        renderer.parse({..._fixture('valid-rust.json'), 'language': 3}),
        isNull,
      );
    });
  });

  group('CodeFileResultView', () {
    testWidgets('hands the content and language hint to CodeBlock', (
      tester,
    ) async {
      await _pump(tester, 'valid-rust.json');

      final block = tester.widget<CodeBlock>(find.byType(CodeBlock));
      expect(block.language, 'rust');
      expect(block.code, 'fn main() {}\n');
      expect(find.text('src/main.rs'), findsOneWidget);
      expect(find.textContaining('Showing'), findsNothing);
    });

    testWidgets('shows a truncation notice with the full size', (tester) async {
      await _pump(tester, 'valid-truncated-no-language.json');

      expect(
        find.text('Truncated: showing 10 of 90000 characters'),
        findsOneWidget,
      );
      expect(tester.widget<CodeBlock>(find.byType(CodeBlock)).language, isNull);
    });
  });

  group('CodeFileRenderer.summary', () {
    testWidgets('names the path and the line count', (tester) async {
      final model = renderer.parse(_fixture('valid-rust.json'))!;
      await tester.pumpWidget(
        _host(Builder(builder: (context) => renderer.summary(context, model))),
      );

      expect(find.text('src/main.rs · 1 line'), findsOneWidget);
    });

    testWidgets('flags a truncated file', (tester) async {
      final model = renderer.parse(
        _fixture('valid-truncated-no-language.json'),
      )!;
      await tester.pumpWidget(
        _host(Builder(builder: (context) => renderer.summary(context, model))),
      );

      expect(find.text('notes.txt · 1 line · truncated'), findsOneWidget);
    });
  });

  testWidgets('the default registry resolves code_file at compact density', (
    tester,
  ) async {
    final registry = ToolResultRendererRegistry(
      byKind: defaultToolResultRenderers(),
    );
    final record = ToolCallRecord(
      toolName: 'file-read',
      status: ToolCallStatus.ok,
      data: _fixture('valid-rust.json'),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ThreadDensityScope(
            density: TimelineDensity.compact,
            child: Builder(
              builder: (context) =>
                  registry.build(context, record) ?? const Text('fallback'),
            ),
          ),
        ),
      ),
    );

    expect(find.text('src/main.rs · 1 line'), findsOneWidget);
    expect(find.byType(CodeBlock), findsNothing);

    await tester.tap(find.bySemanticsLabel('Show full result'));
    await tester.pump();

    expect(find.byType(CodeBlock), findsOneWidget);
  });
}
