import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

ToolCallRecord _call(
  String name, [
  ToolCallStatus status = ToolCallStatus.ok,
]) => ToolCallRecord(
  toolName: name,
  status: status,
  arguments: const {'query': 'secret-argument'},
  result: 'secret-result',
);

Future<void> _pump(
  WidgetTester tester,
  Widget body, {
  TimelineDensity density = TimelineDensity.normal,
  ToolResultBuilder? resolve,
}) async {
  Widget child = ThreadDensityScope(density: density, child: body);
  if (resolve != null) {
    child = ToolResultRendererScope(resolve: resolve, child: child);
  }
  await tester.pumpWidget(MaterialApp(home: Scaffold(body: child)));
  await tester.pump(const Duration(milliseconds: 100));
}

Future<void> _expand(WidgetTester tester) async {
  await tester.tap(find.byKey(TimelineEntryShell.headerKey));
  await tester.pump(const Duration(milliseconds: 300));
}

void main() {
  group('SubagentBody running', () {
    testWidgets('shows the task, a spinner, the thinking and the inner calls', (
      tester,
    ) async {
      await _pump(
        tester,
        SubagentBody(
          label: 'Research authentication',
          entryState: EntryState.active,
          thinking: 'Analyzing OAuth providers...',
          toolCalls: [
            _call('web-search'),
            _call('file-read', ToolCallStatus.pending),
          ],
        ),
        density: TimelineDensity.expanded,
      );

      expect(find.text('Research authentication'), findsOneWidget);
      expect(find.text('Analyzing OAuth providers...'), findsOneWidget);
      expect(find.text('web-search'), findsOneWidget);
      expect(find.text('file-read'), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsWidgets);
    });

    testWidgets('with nothing yet shows the Processing... placeholder', (
      tester,
    ) async {
      await _pump(
        tester,
        const SubagentBody(label: 'Task', entryState: EntryState.active),
      );

      expect(find.text('Processing...'), findsOneWidget);
    });

    testWidgets('normal density shows the last 2 inner calls', (tester) async {
      await _pump(
        tester,
        SubagentBody(
          label: 'Task',
          entryState: EntryState.active,
          toolCalls: [_call('one'), _call('two'), _call('three')],
        ),
      );

      expect(find.text('one'), findsNothing);
      expect(find.text('two'), findsOneWidget);
      expect(find.text('three'), findsOneWidget);
    });

    testWidgets('expanded density shows every inner call', (tester) async {
      await _pump(
        tester,
        SubagentBody(
          label: 'Task',
          entryState: EntryState.active,
          toolCalls: [_call('one'), _call('two'), _call('three')],
        ),
        density: TimelineDensity.expanded,
      );

      expect(find.text('one'), findsOneWidget);
      expect(find.text('two'), findsOneWidget);
      expect(find.text('three'), findsOneWidget);
    });
  });

  group('SubagentBody inner calls are compact rows', () {
    for (final density in TimelineDensity.values) {
      testWidgets('at ${density.name}: icon, name and status, no details', (
        tester,
      ) async {
        await _pump(
          tester,
          SubagentBody(
            label: 'Task',
            entryState: EntryState.active,
            toolCalls: [_call('web-search')],
          ),
          density: density,
        );
        // Compact starts collapsed, so open the entry to see the rows.
        if (density == TimelineDensity.compact) await _expand(tester);

        expect(find.byIcon(Icons.build_outlined), findsOneWidget);
        expect(find.text('web-search'), findsOneWidget);
        expect(find.byType(ToolStatusIcon), findsOneWidget);
        expect(find.textContaining('secret-argument'), findsNothing);
        expect(find.textContaining('secret-result'), findsNothing);
        expect(find.byType(GenericToolResultView), findsNothing);
      });
    }

    testWidgets('tapping an inner call does not expand it', (tester) async {
      await _pump(
        tester,
        SubagentBody(
          label: 'Task',
          entryState: EntryState.active,
          toolCalls: [_call('web-search')],
        ),
      );

      await tester.tap(find.text('web-search'));
      await tester.pump(const Duration(milliseconds: 300));

      expect(find.textContaining('secret-result'), findsNothing);
    });

    testWidgets('never calls the resolver of a ToolResultRendererScope', (
      tester,
    ) async {
      final resolved = <String>[];
      await _pump(
        tester,
        SubagentBody(
          label: 'Task',
          entryState: EntryState.active,
          toolCalls: [_call('web-search')],
        ),
        resolve: (context, record) {
          resolved.add(record.toolName);
          return const Text('rich renderer');
        },
      );

      expect(resolved, isEmpty);
      expect(find.text('rich renderer'), findsNothing);
    });
  });

  group('SubagentBody complete', () {
    testWidgets('shows the check icon and collapses to the summary', (
      tester,
    ) async {
      await _pump(
        tester,
        const SubagentBody(
          label: 'Research caching',
          summary: 'Found 3 caching strategies.',
        ),
      );

      expect(find.text('Research caching'), findsOneWidget);
      expect(find.byIcon(Icons.check_circle_outline), findsOneWidget);
      expect(find.byIcon(Icons.expand_more_rounded), findsOneWidget);

      await _expand(tester);

      expect(find.text('Found 3 caching strategies.'), findsOneWidget);
    });

    testWidgets('stale after completion shows the cancelled icon in the '
        'amber token', (tester) async {
      await _pump(
        tester,
        const SubagentBody(
          label: 'Research caching',
          summary: 'Found 3 caching strategies.',
          entryState: EntryState.stale,
        ),
      );

      final icon = tester.widget<Icon>(find.byIcon(Icons.cancel_outlined));
      expect(icon.color, AssistantUiColors.toolDenied);
      expect(find.byIcon(Icons.check_circle_outline), findsNothing);
    });

    testWidgets('the complete icon uses the green token', (tester) async {
      await _pump(tester, const SubagentBody(label: 'Task', summary: 'Done.'));

      final icon = tester.widget<Icon>(find.byIcon(Icons.check_circle_outline));
      expect(icon.color, AssistantUiColors.toolOk);
    });
  });
}
