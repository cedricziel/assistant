import 'dart:ui' show Tristate;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Widget _shell({
  EntryState state = EntryState.complete,
  bool failed = false,
  bool expandable = true,
  Widget? before,
}) => MaterialApp(
  home: Scaffold(
    body: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        ?before,
        TimelineEntryShell(
          icon: Icons.build_outlined,
          label: 'web_search',
          entryState: state,
          failed: failed,
          expandable: expandable,
          body: const Text('arguments'),
        ),
      ],
    ),
  ),
);

Future<void> _pump(WidgetTester tester, Widget widget) async {
  await tester.pumpWidget(widget);
  await tester.pump();
}

SemanticsNode _headerNode(WidgetTester tester, String status) =>
    find.semantics.byLabel('web_search, $status').evaluate().single;

final _expanded = find.byIcon(Icons.expand_less_rounded);
final _collapsed = find.byIcon(Icons.expand_more_rounded);

void main() {
  group('TimelineEntryShell header label', () {
    final cases = <String, ({EntryState state, bool failed})>{
      'web_search, running': (state: EntryState.active, failed: false),
      'web_search, done': (state: EntryState.complete, failed: false),
      'web_search, failed': (state: EntryState.complete, failed: true),
      'web_search, awaiting input': (
        state: EntryState.awaitingInput,
        failed: false,
      ),
      'web_search, stale': (state: EntryState.stale, failed: false),
    };

    for (final entry in cases.entries) {
      testWidgets('is "${entry.key}"', (tester) async {
        final handle = tester.ensureSemantics();
        await _pump(
          tester,
          _shell(state: entry.value.state, failed: entry.value.failed),
        );

        expect(find.semantics.byLabel(entry.key), findsOne);
        handle.dispose();
      });
    }
  });

  group('TimelineEntryShell keyboard operation', () {
    testWidgets('Tab focuses the header and shows a focus highlight', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      await _pump(
        tester,
        _shell(
          state: EntryState.active,
          before: TextButton(onPressed: () {}, child: const Text('before')),
        ),
      );
      expect(
        tester
            .widget<Container>(find.byKey(TimelineEntryShell.headerKey))
            .foregroundDecoration,
        isNull,
        reason: 'no highlight before focus',
      );

      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pump();

      expect(
        _headerNode(tester, 'running').flagsCollection.isFocused,
        Tristate.isTrue,
      );
      expect(
        tester
            .widget<Container>(find.byKey(TimelineEntryShell.headerKey))
            .foregroundDecoration,
        isNotNull,
        reason: 'keyboard focus paints a visible highlight',
      );
      handle.dispose();
    });

    testWidgets('Enter collapses an expanded entry and flips expanded', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      await _pump(tester, _shell(state: EntryState.active));
      expect(_expanded, findsOneWidget);
      expect(
        _headerNode(tester, 'running').flagsCollection.isExpanded,
        Tristate.isTrue,
      );

      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pump();

      expect(_collapsed, findsOneWidget);
      expect(
        _headerNode(tester, 'running').flagsCollection.isExpanded,
        Tristate.isFalse,
      );
      handle.dispose();
    });

    testWidgets('Space expands a collapsed entry and flips expanded', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      await _pump(tester, _shell());
      expect(_collapsed, findsOneWidget);
      expect(
        _headerNode(tester, 'done').flagsCollection.isExpanded,
        Tristate.isFalse,
      );

      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.space);
      await tester.pump();

      expect(_expanded, findsOneWidget);
      expect(
        _headerNode(tester, 'done').flagsCollection.isExpanded,
        Tristate.isTrue,
      );
      handle.dispose();
    });
  });
}
