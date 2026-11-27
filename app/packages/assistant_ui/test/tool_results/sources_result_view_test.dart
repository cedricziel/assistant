import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'contract_fixtures_test.dart' show fixturesDir, fixturesOf;

const _renderer = SourcesRenderer();

Map<String, Object?> _fixture(String name) =>
    fixturesOf(fixturesDir(), 'sources')[name]!;

Widget _host(Widget child) => MaterialApp(
  home: Scaffold(body: SingleChildScrollView(child: child)),
);

Future<void> _pump(
  WidgetTester tester,
  Map<String, Object?> data, {
  void Function(String url)? onTapLink,
}) async {
  final model = _renderer.parse(data)!;
  await tester.pumpWidget(
    _host(
      ChatMarkdownScope(
        onTapLink: onTapLink,
        child: Builder(
          builder: (context) =>
              _renderer.build(context, model, ToolCallStatus.ok),
        ),
      ),
    ),
  );
}

void main() {
  group('SourcesRenderer.parse', () {
    test('reads every result', () {
      final model = _renderer.parse(_fixture('valid-results.json'))!;

      expect(model.query, 'rust async traits');
      expect(model.results, hasLength(2));
      expect(model.results.first.title, 'Async functions in traits');
      expect(model.results.first.domain, 'blog.rust-lang.org');
      expect(
        model.results.first.snippet,
        'Async fn in traits is stable in Rust 1.75.',
      );
      expect(model.results.last.snippet, isNull);
    });

    test('accepts an empty result list', () {
      final model = _renderer.parse(_fixture('valid-empty.json'))!;

      expect(model.results, isEmpty);
    });

    test('rejects a result without a URL', () {
      expect(
        _renderer.parse(_fixture('invalid-result-missing-url.json')),
        isNull,
      );
    });

    test('rejects a non-array result list', () {
      expect(
        _renderer.parse(_fixture('invalid-results-not-array.json')),
        isNull,
      );
    });

    test('rejects a payload of another kind', () {
      expect(
        _renderer.parse({
          ..._fixture('valid-results.json'),
          'kind': 'terminal',
        }),
        isNull,
      );
    });
  });

  group('SourcesResultView', () {
    testWidgets('shows a row per result with title, domain and snippet', (
      tester,
    ) async {
      await _pump(tester, _fixture('valid-results.json'));

      expect(find.text('rust async traits'), findsOneWidget);
      expect(find.text('Async functions in traits'), findsOneWidget);
      expect(find.text('blog.rust-lang.org'), findsOneWidget);
      expect(
        find.text('Async fn in traits is stable in Rust 1.75.'),
        findsOneWidget,
      );
      expect(find.text('The async-trait crate'), findsOneWidget);
      expect(find.text('docs.rs'), findsOneWidget);
    });

    testWidgets('shows an empty state without results', (tester) async {
      await _pump(tester, _fixture('valid-empty.json'));

      expect(find.text('No results'), findsOneWidget);
    });

    testWidgets('activates a row through ChatMarkdownScope.onTapLink', (
      tester,
    ) async {
      final taps = <String>[];
      await _pump(tester, _fixture('valid-results.json'), onTapLink: taps.add);

      await tester.tap(find.text('The async-trait crate'));

      expect(taps, ['https://docs.rs/async-trait']);
    });
  });

  testWidgets('summary names the result count', (tester) async {
    final model = _renderer.parse(_fixture('valid-results.json'))!;
    await tester.pumpWidget(
      _host(Builder(builder: (context) => _renderer.summary(context, model))),
    );

    expect(find.text('2 sources'), findsOneWidget);
  });

  testWidgets('compact density shows the summary and tap expands the view', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    final registry = ToolResultRendererRegistry(
      byKind: defaultToolResultRenderers(),
    );
    final record = ToolCallRecord(
      toolName: 'web-search',
      status: ToolCallStatus.ok,
      data: _fixture('valid-results.json'),
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

      expect(find.text('2 sources'), findsOneWidget);
      expect(find.byType(SourcesResultView), findsNothing);

      await tester.tap(find.bySemanticsLabel('Show full result'));
      await tester.pump();

      expect(find.byType(SourcesResultView), findsOneWidget);
    } finally {
      handle.dispose();
    }
  });
}
