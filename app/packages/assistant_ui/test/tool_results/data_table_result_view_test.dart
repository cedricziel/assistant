import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'contract_fixtures_test.dart' show fixturesDir, fixturesOf;

const _renderer = DataTableResultRenderer();

Map<String, Object?> _table({
  List<Map<String, Object?>>? columns,
  List<Map<String, Object?>>? rows,
  bool truncated = false,
  int? totalRows,
}) {
  final r =
      rows ??
      [
        {'name': 'Backup', 'runs': 10},
        {'name': 'Digest', 'runs': 9},
        {'name': 'alpha', 'runs': null},
      ];
  return {
    'kind': 'data_table',
    'columns':
        columns ??
        [
          {'key': 'name', 'label': 'Name'},
          {'key': 'runs', 'label': 'Runs', 'type': 'number'},
        ],
    'rows': r,
    'truncated': truncated,
    'total_rows': totalRows ?? r.length,
  };
}

Future<void> _pump(
  WidgetTester tester,
  Map<String, Object?> data, {
  double width = 400,
}) async {
  tester.view
    ..physicalSize = Size(width, 600)
    ..devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final model = _renderer.parse(data)!;
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (context) =>
              _renderer.build(context, model, ToolCallStatus.ok),
        ),
      ),
    ),
  );
}

/// The first-column cell texts in display order.
List<String> _names(WidgetTester tester) => [
  for (final text in tester.widgetList<Text>(find.byType(Text)))
    if (const {'Backup', 'Digest', 'alpha'}.contains(text.data)) text.data!,
];

void main() {
  group('parse', () {
    test('reads columns, rows and counts', () {
      final model = _renderer.parse(_table(truncated: true, totalRows: 500))!;
      expect(model.columns.map((c) => c.label), ['Name', 'Runs']);
      expect(model.columns[1].type, 'number');
      expect(model.rows, hasLength(3));
      expect(model.truncated, isTrue);
      expect(model.totalRows, 500);
    });

    test('accepts empty rows', () {
      expect(_renderer.parse(_table(rows: [])), isNotNull);
    });

    test('rejects a wrong kind or malformed fields', () {
      expect(_renderer.parse({..._table(), 'kind': 'plan'}), isNull);
      expect(_renderer.parse({..._table(), 'rows': 'x'}), isNull);
      expect(_renderer.parse({..._table(), 'truncated': 'no'}), isNull);
      expect(_renderer.parse({..._table(), 'total_rows': -1}), isNull);
      expect(
        _renderer.parse(
          _table(
            columns: [
              {'key': 'a'},
            ],
          ),
        ),
        isNull,
      );
    });

    test('is registered by default', () {
      expect(
        defaultToolResultRenderers().map((r) => r.kind),
        contains('data_table'),
      );
    });

    test('agrees with every shared fixture', () {
      final fixtures = fixturesOf(fixturesDir(), 'data_table');
      expect(fixtures, isNotEmpty);
      fixtures.forEach((name, data) {
        expect(
          _renderer.parse(data),
          name.startsWith('valid-') ? isNotNull : isNull,
          reason: name,
        );
      });
    });
  });

  group('summary', () {
    Future<void> pumpSummary(WidgetTester tester, Map<String, Object?> d) =>
        tester.pumpWidget(
          MaterialApp(
            home: Builder(
              builder: (context) =>
                  _renderer.summary(context, _renderer.parse(d)!),
            ),
          ),
        );

    testWidgets('counts rows', (tester) async {
      await pumpSummary(tester, _table());
      expect(find.text('3 rows'), findsOneWidget);
    });

    testWidgets('says when truncated', (tester) async {
      await pumpSummary(tester, _table(truncated: true, totalRows: 500));
      expect(find.text('3 of 500 rows'), findsOneWidget);
    });

    testWidgets('says when empty', (tester) async {
      await pumpSummary(tester, _table(rows: []));
      expect(find.text('No rows'), findsOneWidget);
    });
  });

  group('view', () {
    testWidgets('shows headers and cells, null as a dash', (tester) async {
      await _pump(tester, _table());
      expect(find.text('Name'), findsOneWidget);
      expect(find.text('Runs'), findsOneWidget);
      expect(find.text('Backup'), findsOneWidget);
      expect(find.text('10'), findsOneWidget);
      expect(find.text('—'), findsOneWidget);
    });

    testWidgets('empty rows show the empty-state line', (tester) async {
      await _pump(tester, _table(rows: []));
      expect(find.text('No rows'), findsOneWidget);
      expect(find.byType(DataTable), findsNothing);
    });

    testWidgets('truncation notice shows total_rows', (tester) async {
      await _pump(tester, _table(truncated: true, totalRows: 500));
      expect(find.text('Showing 3 of 500 rows'), findsOneWidget);
    });

    testWidgets('no notice when complete', (tester) async {
      await _pump(tester, _table());
      expect(find.textContaining('Showing'), findsNothing);
    });

    testWidgets('header tap sorts ascending, a second tap reverses', (
      tester,
    ) async {
      await _pump(tester, _table());
      expect(_names(tester), ['Backup', 'Digest', 'alpha']);

      await tester.tap(find.text('Name'));
      await tester.pump();
      expect(_names(tester), ['alpha', 'Backup', 'Digest']);

      await tester.tap(find.text('Name'));
      await tester.pump();
      expect(_names(tester), ['Digest', 'Backup', 'alpha']);
    });

    testWidgets('number columns sort numerically with nulls last', (
      tester,
    ) async {
      await _pump(tester, _table());
      await tester.tap(find.text('Runs'));
      await tester.pump();
      expect(_names(tester), ['Digest', 'Backup', 'alpha']);

      await tester.tap(find.text('Runs'));
      await tester.pump();
      expect(_names(tester), ['Backup', 'Digest', 'alpha']);
    });

    testWidgets('scrolls horizontally at 360 px without overflow', (
      tester,
    ) async {
      final columns = [
        for (var i = 0; i < 8; i++) {'key': 'c$i', 'label': 'Column number $i'},
      ];
      final row = {for (var i = 0; i < 8; i++) 'c$i': 'value of cell $i'};
      await _pump(tester, _table(columns: columns, rows: [row]), width: 360);
      expect(tester.takeException(), isNull);
      final scroll = tester.widget<SingleChildScrollView>(
        find.byType(SingleChildScrollView).first,
      );
      expect(scroll.scrollDirection, Axis.horizontal);
      expect(tester.getSize(find.byType(DataTable)).width, greaterThan(360));
    });
  });
}
