import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'contract_fixtures_test.dart' show fixturesDir, fixturesOf;

const _renderer = PlanResultRenderer();

Map<String, Object?> _step(String title, String status) => {
  'title': title,
  'status': status,
};

Map<String, Object?> _plan({
  String? title = 'Ship the feature',
  List<Map<String, Object?>>? steps,
}) => {
  'kind': 'plan',
  'title': ?title,
  'steps':
      steps ??
      [
        _step('Write tests', 'completed'),
        _step('Implement', 'in_progress'),
        _step('Review', 'pending'),
        _step('Deploy', 'failed'),
        _step('Announce', 'skipped'),
      ],
};

Future<void> _pump(WidgetTester tester, Map<String, Object?> data) async {
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

void main() {
  group('parse', () {
    test('reads the title, steps and progress', () {
      final model = _renderer.parse(_plan())!;
      expect(model.title, 'Ship the feature');
      expect(model.steps, hasLength(5));
      expect(model.steps.first.title, 'Write tests');
      expect(model.steps.first.status, PlanStepStatus.completed);
      expect(model.steps[1].status, PlanStepStatus.inProgress);
      expect(model.steps[4].status, PlanStepStatus.skipped);
      expect(model.completed, 1);
      expect(model.total, 5);
    });

    test('counts only completed steps', () {
      final model = _renderer.parse(
        _plan(
          steps: [
            _step('a', 'completed'),
            _step('b', 'completed'),
            _step('c', 'completed'),
            _step('d', 'in_progress'),
            _step('e', 'skipped'),
          ],
        ),
      )!;
      expect(model.completed, 3);
    });

    test('accepts a plan without a title', () {
      expect(_renderer.parse(_plan(title: null)), isNotNull);
    });

    test('accepts an empty step list', () {
      final model = _renderer.parse(_plan(steps: []))!;
      expect(model.total, 0);
      expect(model.completed, 0);
    });

    test('rejects a wrong kind or malformed fields', () {
      expect(_renderer.parse({..._plan(), 'kind': 'data_table'}), isNull);
      expect(_renderer.parse({..._plan()}..remove('steps')), isNull);
      expect(_renderer.parse({..._plan(), 'title': 1}), isNull);
      expect(
        _renderer.parse({..._plan(), 'title': List.filled(121, 't').join()}),
        isNull,
      );
      expect(_renderer.parse(_plan(steps: [_step('Only', 'done')])), isNull);
      expect(
        _renderer.parse(
          _plan(steps: [_step(List.filled(201, 't').join(), 'pending')]),
        ),
        isNull,
      );
      expect(
        _renderer.parse(
          _plan(steps: [for (var i = 0; i < 51; i++) _step('s$i', 'pending')]),
        ),
        isNull,
      );
    });

    test('is registered by default', () {
      expect(defaultToolResultRenderers().map((r) => r.kind), contains('plan'));
    });

    test('agrees with every shared fixture', () {
      final fixtures = fixturesOf(fixturesDir(), 'plan');
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
    Future<void> pumpSummary(WidgetTester tester, Map<String, Object?> data) =>
        tester.pumpWidget(
          MaterialApp(
            home: Builder(
              builder: (context) =>
                  _renderer.summary(context, _renderer.parse(data)!),
            ),
          ),
        );

    testWidgets('shows the title and progress', (tester) async {
      await pumpSummary(tester, _plan());
      expect(find.text('Ship the feature · 1/5'), findsOneWidget);
    });

    testWidgets('shows progress alone without a title', (tester) async {
      await pumpSummary(tester, _plan(title: null));
      expect(find.text('1/5'), findsOneWidget);
    });
  });

  group('view', () {
    testWidgets('shows the title, progress and every step', (tester) async {
      await _pump(tester, _plan());

      expect(find.byType(PlanResultView), findsOneWidget);
      expect(find.text('Ship the feature'), findsOneWidget);
      expect(find.text('1/5'), findsOneWidget);
      for (final title in [
        'Write tests',
        'Implement',
        'Review',
        'Deploy',
        'Announce',
      ]) {
        expect(find.text(title), findsOneWidget, reason: title);
      }
    });

    testWidgets('shows the 3/5 progress bar', (tester) async {
      await _pump(
        tester,
        _plan(
          steps: [
            _step('a', 'completed'),
            _step('b', 'completed'),
            _step('c', 'completed'),
            _step('d', 'in_progress'),
            _step('e', 'pending'),
          ],
        ),
      );

      expect(find.text('3/5'), findsOneWidget);
      final bar = tester.widget<LinearProgressIndicator>(
        find.byType(LinearProgressIndicator),
      );
      expect(bar.value, closeTo(0.6, 0.0001));
    });

    testWidgets('each status has its own icon', (tester) async {
      await _pump(tester, _plan());

      expect(find.byIcon(Icons.check_circle), findsOneWidget);
      expect(find.byIcon(Icons.autorenew), findsOneWidget);
      expect(find.byIcon(Icons.radio_button_unchecked), findsOneWidget);
      expect(find.byIcon(Icons.cancel), findsOneWidget);
      expect(find.byIcon(Icons.skip_next), findsOneWidget);
    });

    testWidgets('each step has a status semantics label', (tester) async {
      final handle = tester.ensureSemantics();
      try {
        await _pump(tester, _plan());

        expect(find.bySemanticsLabel('Completed: Write tests'), findsOneWidget);
        expect(find.bySemanticsLabel('In progress: Implement'), findsOneWidget);
        expect(find.bySemanticsLabel('Pending: Review'), findsOneWidget);
        expect(find.bySemanticsLabel('Failed: Deploy'), findsOneWidget);
        expect(find.bySemanticsLabel('Skipped: Announce'), findsOneWidget);
      } finally {
        handle.dispose();
      }
    });

    testWidgets('renders an empty plan with 0/0 and no title placeholder', (
      tester,
    ) async {
      await _pump(tester, _plan(title: null, steps: []));

      expect(find.text('0/0'), findsOneWidget);
      expect(find.text('Plan'), findsNothing);
    });

    testWidgets('an MCP payload with a plan kind resolves the view', (
      tester,
    ) async {
      const registry = ToolResultRendererRegistry(
        byKind: [PlanResultRenderer()],
      );
      final record = ToolCallRecord(
        toolName: 'mcp__notes__outline',
        status: ToolCallStatus.ok,
        data: _plan(),
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Builder(
              builder: (context) =>
                  registry.build(context, record) ?? const Text('fallback'),
            ),
          ),
        ),
      );

      expect(find.byType(PlanResultView), findsOneWidget);
      expect(find.text('1/5'), findsOneWidget);
    });
  });
}
