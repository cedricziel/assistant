import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'contract_fixtures_test.dart' show fixturesDir, fixturesOf;

Map<String, Object?> _fixture(String name) =>
    fixturesOf(fixturesDir(), 'code_diff')[name]!;

Widget _host(Widget child) => MaterialApp(
  home: Scaffold(body: SingleChildScrollView(child: child)),
);

Future<void> _pump(
  WidgetTester tester,
  Map<String, Object?> data, {
  ToolCallStatus status = ToolCallStatus.ok,
}) async {
  const renderer = CodeDiffRenderer();
  final model = renderer.parse(data)!;
  await tester.pumpWidget(
    _host(
      Builder(builder: (context) => renderer.build(context, model, status)),
    ),
  );
}

Future<void> _pumpSummary(WidgetTester tester, String fixture) async {
  const renderer = CodeDiffRenderer();
  final model = renderer.parse(_fixture(fixture))!;
  await tester.pumpWidget(
    _host(Builder(builder: (context) => renderer.summary(context, model))),
  );
}

void main() {
  const renderer = CodeDiffRenderer();

  group('CodeDiffRenderer.parse', () {
    test('reads a valid edit', () {
      final model = renderer.parse(_fixture('valid-edit.json'))!;

      expect(model.path, 'src/lib.rs');
      expect(model.language, 'rust');
      expect(model.operation, DiffOperation.edit);
      expect(model.added, 1);
      expect(model.removed, 1);
      expect(model.truncated, isFalse);
      expect(model.hunks, hasLength(1));
      final hunk = model.hunks.single;
      expect(
        [hunk.oldStart, hunk.oldLines, hunk.newStart, hunk.newLines],
        [3, 3, 3, 3],
      );
      expect(hunk.lines.map((l) => l.op), [
        DiffOp.context,
        DiffOp.remove,
        DiffOp.add,
        DiffOp.context,
      ]);
      expect(hunk.lines[1].text, '    1');
    });

    test('reads a create with no language', () {
      final model = renderer.parse(_fixture('valid-create.json'))!;

      expect(model.operation, DiffOperation.create);
      expect(model.language, isNull);
    });

    test('accepts empty hunks', () {
      expect(renderer.parse(_fixture('valid-empty-hunks.json')), isNotNull);
    });

    test('a line of exactly 512 characters is accepted', () {
      final data = _fixture('valid-edit.json');
      final hunk = ((data['hunks']! as List).single as Map)
          .cast<String, Object?>();
      final lines = [
        {'op': 'add', 'text': 'x' * 512},
      ];

      expect(
        renderer.parse({
          ...data,
          'hunks': [
            {...hunk, 'lines': lines},
          ],
        }),
        isNotNull,
      );
    });
  });

  group('CodeDiffResultView', () {
    testWidgets('prefixes added and removed lines with a gutter glyph', (
      tester,
    ) async {
      await _pump(tester, _fixture('valid-edit.json'));

      expect(find.text('+'), findsOneWidget);
      expect(find.text('−'), findsOneWidget);
      expect(find.text('    2'), findsOneWidget);
      expect(find.text('    1'), findsOneWidget);
      expect(find.text('fn a() {'), findsOneWidget);
    });

    testWidgets('shows the path, operation, counts and hunk header', (
      tester,
    ) async {
      await _pump(tester, _fixture('valid-edit.json'));

      expect(find.text('src/lib.rs'), findsOneWidget);
      expect(find.text('Edited'), findsOneWidget);
      expect(find.text('+1 −1'), findsOneWidget);
      expect(find.text('@@ -3,3 +3,3 @@'), findsOneWidget);
    });

    testWidgets('labels a created file', (tester) async {
      await _pump(tester, _fixture('valid-create.json'));

      expect(find.text('Created'), findsOneWidget);
      expect(find.text('+2 −0'), findsOneWidget);
    });

    testWidgets('labels an overwrite and says when nothing changed', (
      tester,
    ) async {
      await _pump(tester, _fixture('valid-empty-hunks.json'));

      expect(find.text('Overwritten'), findsOneWidget);
      expect(find.text('No changes'), findsOneWidget);
    });

    testWidgets('shows a truncation notice', (tester) async {
      await _pump(tester, {..._fixture('valid-edit.json'), 'truncated': true});

      expect(find.text('Diff truncated'), findsOneWidget);
    });

    testWidgets('has no truncation notice when complete', (tester) async {
      await _pump(tester, _fixture('valid-edit.json'));

      expect(find.text('Diff truncated'), findsNothing);
    });

    testWidgets('exposes the line kinds to assistive technology', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      await _pump(tester, _fixture('valid-edit.json'));

      expect(find.bySemanticsLabel('added'), findsOneWidget);
      expect(find.bySemanticsLabel('removed'), findsOneWidget);
      handle.dispose();
    });
  });

  group('CodeDiffRenderer.summary', () {
    testWidgets('shows the counts for an edit', (tester) async {
      await _pumpSummary(tester, 'valid-edit.json');

      expect(find.text('Edited src/lib.rs · +1 −1'), findsOneWidget);
    });

    testWidgets('names the create operation', (tester) async {
      await _pumpSummary(tester, 'valid-create.json');

      expect(find.text('Created new.txt · +2 −0'), findsOneWidget);
    });
  });

  testWidgets('the default registry resolves code_diff at compact density', (
    tester,
  ) async {
    final registry = ToolResultRendererRegistry(
      byKind: defaultToolResultRenderers(),
    );
    final record = ToolCallRecord(
      toolName: 'file-edit',
      status: ToolCallStatus.ok,
      data: _fixture('valid-edit.json'),
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

    expect(find.text('Edited src/lib.rs · +1 −1'), findsOneWidget);
    expect(find.byType(CodeDiffResultView), findsNothing);

    await tester.tap(find.bySemanticsLabel('Show full result'));
    await tester.pump();

    expect(find.byType(CodeDiffResultView), findsOneWidget);
  });
}
