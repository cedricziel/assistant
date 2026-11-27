import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'contract_fixtures_test.dart' show fixturesDir, fixturesOf;

const _renderer = LinkPreviewRenderer();

Map<String, Object?> _preview({
  String url = 'https://example.com/a',
  String? finalUrl,
  Object? status = 200,
  Object? title = 'Example',
  Object? contentType = 'text/html; charset=utf-8',
  Object? excerpt = 'A short excerpt of the page.',
}) => {
  'kind': 'link_preview',
  'url': url,
  'final_url': finalUrl ?? url,
  'status': status,
  'title': ?title,
  'content_type': ?contentType,
  'excerpt': ?excerpt,
};

Future<void> _pump(
  WidgetTester tester,
  Map<String, Object?> data, {
  ToolCallStatus status = ToolCallStatus.ok,
  void Function(String url)? onTapLink,
}) async {
  final model = _renderer.parse(data)!;
  await tester.pumpWidget(
    MaterialApp(
      home: ChatMarkdownScope(
        onTapLink: onTapLink,
        child: Scaffold(
          body: Builder(
            builder: (context) => _renderer.build(context, model, status),
          ),
        ),
      ),
    ),
  );
}

void main() {
  group('parse', () {
    test('reads the preview fields', () {
      final model = _renderer.parse(_preview())!;
      expect(model.url, 'https://example.com/a');
      expect(model.finalUrl, 'https://example.com/a');
      expect(model.status, 200);
      expect(model.title, 'Example');
      expect(model.contentType, 'text/html; charset=utf-8');
      expect(model.excerpt, 'A short excerpt of the page.');
      expect(model.redirected, isFalse);
    });

    test('accepts the minimal payload without title, type or excerpt', () {
      final model = _renderer.parse(
        _preview(title: null, contentType: null, excerpt: null),
      )!;
      expect(model.title, isNull);
      expect(model.contentType, isNull);
      expect(model.excerpt, isNull);
    });

    test('marks a redirect when final_url differs', () {
      final model = _renderer.parse(
        _preview(finalUrl: 'https://www.example.com/a/'),
      )!;
      expect(model.redirected, isTrue);
      expect(model.finalHost, 'www.example.com');
    });

    test('rejects a wrong kind or malformed fields', () {
      expect(_renderer.parse({..._preview(), 'kind': 'sources'}), isNull);
      expect(_renderer.parse({..._preview(), 'status': 'ok'}), isNull);
      expect(_renderer.parse({..._preview(), 'url': 1}), isNull);
      expect(_renderer.parse({..._preview()}..remove('final_url')), isNull);
      expect(_renderer.parse({..._preview(), 'title': 1}), isNull);
      expect(
        _renderer.parse({..._preview(), 'title': List.filled(201, 't').join()}),
        isNull,
      );
      expect(
        _renderer.parse({
          ..._preview(),
          'excerpt': List.filled(281, 'x').join(),
        }),
        isNull,
      );
    });

    test('is registered by default', () {
      expect(
        defaultToolResultRenderers().map((r) => r.kind),
        contains('link_preview'),
      );
    });

    test('agrees with every shared fixture', () {
      final fixtures = fixturesOf(fixturesDir(), 'link_preview');
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

    testWidgets('shows the title and HTTP status', (tester) async {
      await pumpSummary(tester, _preview());
      expect(find.text('Example · HTTP 200'), findsOneWidget);
    });

    testWidgets('falls back to the final URL without a title', (tester) async {
      await pumpSummary(
        tester,
        _preview(
          finalUrl: 'https://www.example.com/',
          status: 404,
          title: null,
        ),
      );
      expect(find.text('https://www.example.com/ · HTTP 404'), findsOneWidget);
    });
  });

  group('view', () {
    testWidgets('shows the title, host, status, type and excerpt', (
      tester,
    ) async {
      await _pump(tester, _preview());

      expect(find.text('Example'), findsOneWidget);
      expect(find.text('example.com'), findsOneWidget);
      expect(find.text('HTTP 200'), findsOneWidget);
      expect(find.text('text/html; charset=utf-8'), findsOneWidget);
      expect(find.text('A short excerpt of the page.'), findsOneWidget);
      expect(find.textContaining('Redirected'), findsNothing);
    });

    testWidgets('falls back to the URL for the heading', (tester) async {
      await _pump(tester, _preview(title: null));
      expect(find.text('https://example.com/a'), findsWidgets);
    });

    testWidgets('notes a redirect to a different final URL', (tester) async {
      await _pump(tester, _preview(finalUrl: 'https://www.example.com/a/'));
      expect(
        find.text('Redirected to https://www.example.com/a/'),
        findsOneWidget,
      );
    });

    testWidgets('tapping opens the final URL through onTapLink', (
      tester,
    ) async {
      final opened = <String>[];
      await _pump(
        tester,
        _preview(finalUrl: 'https://www.example.com/a/'),
        onTapLink: opened.add,
      );

      await tester.tap(find.byType(LinkPreviewResultView));
      await tester.pump();

      expect(opened, ['https://www.example.com/a/']);
    });

    testWidgets('does not crash without a scope', (tester) async {
      final model = _renderer.parse(_preview())!;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: LinkPreviewResultView(data: model)),
        ),
      );

      await tester.tap(find.byType(LinkPreviewResultView));
      await tester.pump();

      expect(tester.takeException(), isNull);
    });

    testWidgets('an error record still shows its 404 badge', (tester) async {
      const registry = ToolResultRendererRegistry(
        byKind: [LinkPreviewRenderer()],
      );
      final record = ToolCallRecord(
        toolName: 'web-fetch',
        status: ToolCallStatus.error,
        data: _preview(
          finalUrl: 'https://example.com/missing',
          status: 404,
          title: null,
          contentType: null,
          excerpt: null,
        ),
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

      expect(find.byType(LinkPreviewResultView), findsOneWidget);
      expect(find.text('HTTP 404'), findsOneWidget);
    });
  });
}
