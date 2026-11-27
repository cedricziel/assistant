import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'contract_fixtures_test.dart' show fixturesDir, fixturesOf;

const _renderer = TerminalRenderer();

Map<String, Object?> _fixture(String name) =>
    fixturesOf(fixturesDir(), 'terminal')[name]!;

Map<String, Object?> _terminal({
  String command = 'make test',
  int? exitCode = 2,
  String stdout = 'running tests\n',
  String stderr = '',
  bool stdoutTruncated = false,
  bool stderrTruncated = false,
  bool timedOut = false,
}) => {
  'kind': 'terminal',
  'command': command,
  'exit_code': exitCode,
  'stdout': stdout,
  'stderr': stderr,
  'stdout_truncated': stdoutTruncated,
  'stderr_truncated': stderrTruncated,
  'duration_ms': 4210,
  'timed_out': timedOut,
};

Widget _host(Widget child) => MaterialApp(
  home: Scaffold(body: SingleChildScrollView(child: child)),
);

Future<void> _pump(
  WidgetTester tester,
  Map<String, Object?> data, {
  ToolCallStatus status = ToolCallStatus.ok,
}) async {
  final model = _renderer.parse(data)!;
  await tester.pumpWidget(
    _host(
      Builder(builder: (context) => _renderer.build(context, model, status)),
    ),
  );
}

void main() {
  group('TerminalRenderer.parse', () {
    test('reads a successful run', () {
      final model = _renderer.parse(_fixture('valid-success.json'))!;

      expect(model.command, 'echo hi');
      expect(model.exitCode, 0);
      expect(model.stdout, 'hi\n');
      expect(model.stderr, isEmpty);
      expect(model.durationMs, 12);
      expect(model.timedOut, isFalse);
    });

    test('a timed-out run has no exit code', () {
      final model = _renderer.parse(_fixture('valid-timed-out.json'))!;

      expect(model.exitCode, isNull);
      expect(model.timedOut, isTrue);
    });

    test('ignores fields a newer server adds', () {
      expect(_renderer.parse(_fixture('valid-unknown-field.json')), isNotNull);
    });

    test('rejects a non-integer exit code', () {
      expect(_renderer.parse(_fixture('invalid-exit-code-type.json')), isNull);
    });

    test('rejects a payload without a command', () {
      expect(_renderer.parse(_fixture('invalid-missing-command.json')), isNull);
    });

    test('rejects a payload of another kind', () {
      expect(_renderer.parse(_fixture('invalid-wrong-kind.json')), isNull);
    });

    test('rejects a negative duration', () {
      expect(_renderer.parse({..._terminal(), 'duration_ms': -1}), isNull);
    });

    test('rejects non-boolean truncation flags', () {
      expect(
        _renderer.parse({..._terminal(), 'stdout_truncated': 'yes'}),
        isNull,
      );
    });
  });

  group('TerminalResultView', () {
    testWidgets('shows the command, exit badge and duration', (tester) async {
      await _pump(tester, _fixture('valid-success.json'));

      expect(find.text('\$ echo hi'), findsOneWidget);
      expect(find.text('exit 0'), findsOneWidget);
      expect(find.text('12ms'), findsOneWidget);
      expect(find.text('hi'), findsOneWidget);
    });

    testWidgets('shows a failure badge for a non-zero exit', (tester) async {
      await _pump(
        tester,
        _fixture('valid-failure.json'),
        status: ToolCallStatus.error,
      );

      expect(find.text('exit 101'), findsOneWidget);
      expect(find.text('timed out'), findsNothing);
    });

    testWidgets('shows a timeout badge instead of an exit code', (
      tester,
    ) async {
      await _pump(tester, _fixture('valid-timed-out.json'));

      expect(find.text('timed out'), findsOneWidget);
      expect(find.textContaining('exit'), findsNothing);
      expect(find.text('started'), findsOneWidget);
    });

    testWidgets('renders stderr in the error colour', (tester) async {
      await _pump(
        tester,
        _fixture('valid-failure.json'),
        status: ToolCallStatus.error,
      );

      final stderr = tester.widget<Text>(find.text('error: test failed'));
      final error = Theme.of(tester.element(find.byType(TerminalResultView)))
          .colorScheme
          .error;
      expect(stderr.style?.color, error);
    });

    testWidgets('notices each truncated stream separately', (tester) async {
      await _pump(
        tester,
        _terminal(
          stdout: 'kept stdout',
          stderr: 'kept stderr',
          stdoutTruncated: true,
          stderrTruncated: true,
        ),
      );

      expect(find.text('stdout truncated'), findsOneWidget);
      expect(find.text('stderr truncated'), findsOneWidget);
    });

    testWidgets('shows no truncation notice when nothing was cut', (
      tester,
    ) async {
      await _pump(tester, _fixture('valid-success.json'));

      expect(find.textContaining('truncated'), findsNothing);
    });
  });

  group('TerminalRenderer.summary', () {
    testWidgets('counts stderr lines for a failing exit', (tester) async {
      final model = _renderer.parse(
        _terminal(stderr: List.generate(14, (i) => 'error $i').join('\n')),
      )!;
      await tester.pumpWidget(
        _host(Builder(builder: (context) => _renderer.summary(context, model))),
      );

      expect(find.text('exit 2 · 14 lines stderr'), findsOneWidget);
    });

    testWidgets('counts stdout lines when stderr is empty', (tester) async {
      final model = _renderer.parse(_terminal(exitCode: 0, stderr: ''))!;
      await tester.pumpWidget(
        _host(Builder(builder: (context) => _renderer.summary(context, model))),
      );

      expect(find.text('exit 0 · 1 line stdout'), findsOneWidget);
    });

    testWidgets('names a timeout', (tester) async {
      final model = _renderer.parse(_terminal(exitCode: null, timedOut: true))!;
      await tester.pumpWidget(
        _host(Builder(builder: (context) => _renderer.summary(context, model))),
      );

      expect(find.text('timed out · 1 line stdout'), findsOneWidget);
    });
  });

  testWidgets('compact density shows the summary and tap expands the view', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    final registry = ToolResultRendererRegistry(
      byKind: defaultToolResultRenderers(),
    );
    final record = ToolCallRecord(
      toolName: 'bash',
      status: ToolCallStatus.error,
      data: _terminal(stderr: List.generate(14, (i) => 'error $i').join('\n')),
    );

    try {
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

      expect(find.text('exit 2 · 14 lines stderr'), findsOneWidget);
      expect(find.byType(TerminalResultView), findsNothing);

      await tester.tap(find.bySemanticsLabel('Show full result'));
      await tester.pump();

      expect(find.byType(TerminalResultView), findsOneWidget);
      expect(find.text('\$ make test'), findsOneWidget);
    } finally {
      handle.dispose();
    }
  });
}
