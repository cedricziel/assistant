import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

ToolCallRecord _record({
  String name = 'web_search',
  ToolCallStatus status = ToolCallStatus.ok,
  Map<String, dynamic>? arguments = const {'query': 'flutter'},
  String? result = '3 results',
  Duration? duration,
}) => ToolCallRecord(
  toolName: name,
  status: status,
  arguments: arguments,
  result: result,
  duration: duration,
);

Future<void> _pump(
  WidgetTester tester,
  Widget body, {
  ToolResultBuilder? resolve,
  TimelineDensity density = TimelineDensity.normal,
}) async {
  Widget child = ThreadDensityScope(density: density, child: body);
  if (resolve != null) {
    child = ToolResultRendererScope(resolve: resolve, child: child);
  }
  await tester.pumpWidget(MaterialApp(home: Scaffold(body: child)));
  await tester.pump();
}

Future<void> _expand(WidgetTester tester) async {
  await tester.tap(find.byKey(TimelineEntryShell.headerKey));
  await tester.pump(const Duration(milliseconds: 300));
}

Color _iconColor(WidgetTester tester, IconData icon) =>
    tester.widget<Icon>(find.byIcon(icon)).color!;

void main() {
  group('ToolCallBody resolver scope', () {
    testWidgets('the resolver result wins over the generic view', (
      tester,
    ) async {
      await _pump(
        tester,
        ToolCallBody(record: _record()),
        resolve: (context, record) =>
            record.toolName == 'web_search' ? const Text('custom') : null,
      );
      await _expand(tester);

      expect(find.text('custom'), findsOneWidget);
      expect(find.byType(GenericToolResultView), findsNothing);
    });

    testWidgets('a null result falls back to the generic view', (tester) async {
      await _pump(
        tester,
        ToolCallBody(record: _record(name: 'bash')),
        resolve: (context, record) => null,
      );
      await _expand(tester);

      expect(find.byType(GenericToolResultView), findsOneWidget);
      expect(find.text('3 results'), findsOneWidget);
    });

    testWidgets('no scope renders the generic view', (tester) async {
      await _pump(tester, ToolCallBody(record: _record()));
      await _expand(tester);

      expect(find.byType(GenericToolResultView), findsOneWidget);
      expect(find.textContaining('"query": "flutter"'), findsOneWidget);
    });

    testWidgets('the resolver receives the record', (tester) async {
      final seen = <ToolCallRecord>[];
      final record = _record();
      await _pump(
        tester,
        ToolCallBody(record: record),
        resolve: (context, r) {
          seen.add(r);
          return null;
        },
      );
      await _expand(tester);

      expect(seen, isNotEmpty);
      expect(seen.every((r) => identical(r, record)), isTrue);
    });
  });

  group('ToolCallBody expansion', () {
    testWidgets('the error status force-expands the details', (tester) async {
      await _pump(
        tester,
        ToolCallBody(
          record: _record(status: ToolCallStatus.error, result: 'boom'),
        ),
      );

      expect(find.text('boom'), findsOneWidget);
    });

    testWidgets('a record without arguments or result is not expandable', (
      tester,
    ) async {
      await _pump(
        tester,
        ToolCallBody(record: _record(arguments: null, result: null)),
      );

      final shell = tester.widget<TimelineEntryShell>(
        find.byType(TimelineEntryShell),
      );
      expect(shell.expandable, isFalse);
    });

    testWidgets('details stay collapsed until the header is tapped', (
      tester,
    ) async {
      await _pump(tester, ToolCallBody(record: _record()));

      expect(find.text('3 results'), findsNothing);
    });
  });

  group('ToolCallBody duration badge', () {
    Future<void> pumpWith(
      WidgetTester tester,
      Duration? duration, {
      TimelineDensity density = TimelineDensity.normal,
    }) => _pump(
      tester,
      ToolCallBody(record: _record(duration: duration)),
      density: density,
    );

    testWidgets('formats sub-minute durations in seconds', (tester) async {
      await pumpWith(tester, const Duration(milliseconds: 1200));

      expect(find.text('1.2s'), findsOneWidget);
    });

    testWidgets('formats minute-plus durations as minutes and seconds', (
      tester,
    ) async {
      await pumpWith(tester, const Duration(minutes: 1, seconds: 5));

      expect(find.text('1m 5s'), findsOneWidget);
    });

    testWidgets('shows no badge without a duration', (tester) async {
      await pumpWith(tester, null);

      expect(find.textContaining(RegExp(r'\d+(\.\d)?s$')), findsNothing);
    });

    testWidgets('hides the badge at compact density', (tester) async {
      await pumpWith(
        tester,
        const Duration(milliseconds: 1200),
        density: TimelineDensity.compact,
      );

      expect(find.text('1.2s'), findsNothing);
    });
  });

  group('ToolCallBody status icons', () {
    Future<void> pumpStatus(WidgetTester tester, ToolCallStatus status) =>
        _pump(tester, ToolCallBody(record: _record(status: status)));

    testWidgets('pending shows a spinner and no outcome icon', (tester) async {
      await pumpStatus(tester, ToolCallStatus.pending);

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(find.byIcon(Icons.check_circle_outline), findsNothing);
      expect(find.byIcon(Icons.error_outline), findsNothing);
      expect(find.byIcon(Icons.block), findsNothing);
    });

    testWidgets('ok shows a checkmark in the toolOk token', (tester) async {
      await pumpStatus(tester, ToolCallStatus.ok);

      expect(
        _iconColor(tester, Icons.check_circle_outline),
        AssistantUiColors.toolOk,
      );
      expect(find.byType(CircularProgressIndicator), findsNothing);
    });

    testWidgets('pending hides the spinner at compact density', (tester) async {
      await _pump(
        tester,
        ToolCallBody(record: _record(status: ToolCallStatus.pending)),
        density: TimelineDensity.compact,
      );

      expect(find.byType(CircularProgressIndicator), findsNothing);
    });

    testWidgets('error shows a cross in the error colour', (tester) async {
      await pumpStatus(tester, ToolCallStatus.error);

      final scheme = Theme.of(tester.element(find.byIcon(Icons.error_outline)))
          .colorScheme;
      expect(_iconColor(tester, Icons.error_outline), scheme.error);
    });

    testWidgets('denied shows a prohibition icon in the toolDenied token', (
      tester,
    ) async {
      await pumpStatus(tester, ToolCallStatus.denied);

      expect(_iconColor(tester, Icons.block), AssistantUiColors.toolDenied);
    });

    testWidgets('cancelled shows a muted block icon labelled Cancelled', (
      tester,
    ) async {
      await pumpStatus(tester, ToolCallStatus.cancelled);

      expect(
        _iconColor(tester, Icons.block),
        isNot(AssistantUiColors.toolDenied),
      );
      expect(
        tester.widget<Icon>(find.byIcon(Icons.block)).semanticLabel,
        'Cancelled',
      );
    });
  });

  group('ToolCallBody footer', () {
    testWidgets('renders the footer below the header while collapsed', (
      tester,
    ) async {
      await _pump(
        tester,
        ToolCallBody(
          record: _record(),
          footer: const SizedBox(key: Key('app-frame'), height: 40),
        ),
      );

      final header = find.byKey(TimelineEntryShell.headerKey);
      expect(find.byKey(const Key('app-frame')), findsOneWidget);
      expect(
        tester.getTopLeft(find.byKey(const Key('app-frame'))).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(header).dy),
      );
    });

    testWidgets('forwards the entry state to the shell', (tester) async {
      await _pump(
        tester,
        ToolCallBody(record: _record(), entryState: EntryState.awaitingInput),
      );

      expect(
        tester
            .widget<TimelineEntryShell>(find.byType(TimelineEntryShell))
            .entryState,
        EntryState.awaitingInput,
      );
    });
  });

  group('ToolStatusIcon', () {
    testWidgets('renders each status distinctly', (tester) async {
      for (final status in ToolCallStatus.values) {
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(body: ToolStatusIcon(status: status)),
          ),
        );
        expect(find.byType(ToolStatusIcon), findsOneWidget, reason: '$status');
      }
    });
  });
}
