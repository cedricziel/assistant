import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

/// Parses `{"kind": "fake", "value": "<string>"}`; a `boom` value throws.
class _FakeRenderer extends ToolResultRenderer<String> {
  const _FakeRenderer({this.label = 'fake'});

  final String label;

  @override
  String get kind => 'fake';

  @override
  String? parse(Map<String, Object?> data) {
    final value = data['value'];
    if (value == 'boom') throw StateError('parse blew up');
    return value is String ? value : null;
  }

  @override
  Widget summary(BuildContext context, String model) =>
      Text('$label summary $model');

  @override
  Widget build(BuildContext context, String model, ToolCallStatus status) =>
      Text('$label full $model ${status.name}');
}

ToolCallRecord _record({
  String toolName = 'bash',
  ToolCallStatus status = ToolCallStatus.ok,
  Map<String, Object?>? data = const {'kind': 'fake', 'value': 'v'},
  McpApp? app,
}) => ToolCallRecord(toolName: toolName, status: status, data: data, app: app);

Future<Widget?> _resolve(
  WidgetTester tester,
  ToolResultRendererRegistry registry,
  ToolCallRecord record, {
  TimelineDensity density = TimelineDensity.normal,
}) async {
  Widget? built;
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: ThreadDensityScope(
          density: density,
          child: Builder(
            builder: (context) {
              built = registry.build(context, record);
              return built ?? const Text('fallback');
            },
          ),
        ),
      ),
    ),
  );
  return built;
}

void main() {
  const registry = ToolResultRendererRegistry(byKind: [_FakeRenderer()]);

  group('ToolResultRendererRegistry resolution', () {
    testWidgets('a tool-name renderer wins over the kind renderer', (
      tester,
    ) async {
      const byName = ToolResultRendererRegistry(
        byKind: [_FakeRenderer()],
        byToolName: {'mcp__acme__deploy': _FakeRenderer(label: 'named')},
      );

      await _resolve(tester, byName, _record(toolName: 'mcp__acme__deploy'));

      expect(find.text('named full v ok'), findsOneWidget);
    });

    testWidgets('the kind selects the renderer', (tester) async {
      await _resolve(tester, registry, _record());

      expect(find.text('fake full v ok'), findsOneWidget);
    });

    testWidgets('an unknown kind falls back', (tester) async {
      final built = await _resolve(
        tester,
        registry,
        _record(data: {'kind': 'weather', 'value': 'v'}),
      );

      expect(built, isNull);
    });

    testWidgets('data without a kind falls back', (tester) async {
      final built = await _resolve(
        tester,
        registry,
        _record(data: {'value': 'v'}),
      );

      expect(built, isNull);
    });

    testWidgets('missing data falls back', (tester) async {
      final built = await _resolve(tester, registry, _record(data: null));

      expect(built, isNull);
    });

    testWidgets('a parse that returns null falls back', (tester) async {
      final built = await _resolve(
        tester,
        registry,
        _record(data: {'kind': 'fake', 'value': 1}),
      );

      expect(built, isNull);
    });

    testWidgets('a parse that throws falls back without surfacing', (
      tester,
    ) async {
      final built = await _resolve(
        tester,
        registry,
        _record(data: {'kind': 'fake', 'value': 'boom'}),
      );

      expect(built, isNull);
      expect(tester.takeException(), isNull);
    });
  });

  group('ToolResultRendererRegistry status', () {
    testWidgets('denied falls back whatever the data says', (tester) async {
      final built = await _resolve(
        tester,
        registry,
        _record(status: ToolCallStatus.denied),
      );

      expect(built, isNull);
    });

    testWidgets('pending renders no result body', (tester) async {
      final built = await _resolve(
        tester,
        registry,
        _record(status: ToolCallStatus.pending),
      );

      expect(built, isNull);
    });

    testWidgets('cancelled falls back', (tester) async {
      final built = await _resolve(
        tester,
        registry,
        _record(status: ToolCallStatus.cancelled),
      );

      expect(built, isNull);
    });

    testWidgets('error uses the renderer when the data parses', (tester) async {
      await _resolve(tester, registry, _record(status: ToolCallStatus.error));

      expect(find.text('fake full v error'), findsOneWidget);
    });

    testWidgets('a record with an MCP App is left to the app frame', (
      tester,
    ) async {
      final built = await _resolve(
        tester,
        registry,
        _record(
          app: const McpApp(
            server: 'notes',
            resourceUri: 'ui://notes/outline',
            html: '<p>hi</p>',
          ),
        ),
      );

      expect(built, isNull);
    });
  });

  group('ToolResultRendererRegistry density', () {
    testWidgets('normal density builds the full view', (tester) async {
      await _resolve(tester, registry, _record());

      expect(find.text('fake full v ok'), findsOneWidget);
      expect(find.byType(CompactExpandable), findsNothing);
    });

    testWidgets('expanded density builds the full view', (tester) async {
      await _resolve(
        tester,
        registry,
        _record(),
        density: TimelineDensity.expanded,
      );

      expect(find.text('fake full v ok'), findsOneWidget);
      expect(find.byType(CompactExpandable), findsNothing);
    });

    testWidgets('compact density builds the summary behind a toggle', (
      tester,
    ) async {
      await _resolve(
        tester,
        registry,
        _record(),
        density: TimelineDensity.compact,
      );

      expect(find.byType(CompactExpandable), findsOneWidget);
      expect(find.text('fake summary v'), findsOneWidget);
      expect(find.text('fake full v ok'), findsNothing);
    });
  });

  testWidgets('build installs as the scope resolver', (tester) async {
    final ToolResultBuilder resolve = registry.build;
    Widget? built;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ToolResultRendererScope(
            resolve: resolve,
            child: Builder(
              builder: (context) {
                built = ToolResultRendererScope.of(context)
                    ?.call(context, _record());
                return built ?? const SizedBox();
              },
            ),
          ),
        ),
      ),
    );

    expect(find.text('fake full v ok'), findsOneWidget);
  });
}
