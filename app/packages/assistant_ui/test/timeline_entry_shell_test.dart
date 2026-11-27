import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

const _bodyText = 'shell body';
const _footerText = 'shell footer';

Widget _shell({
  EntryState state = EntryState.complete,
  TimelineDensity density = TimelineDensity.normal,
  bool expandable = true,
  bool forceExpand = false,
  Widget? footer,
  bool disableAnimations = false,
}) => MaterialApp(
  home: MediaQuery(
    data: MediaQueryData(disableAnimations: disableAnimations),
    child: ThreadDensityScope(
      density: density,
      child: Scaffold(
        body: TimelineEntryShell(
          icon: Icons.build_outlined,
          label: 'tool',
          entryState: state,
          expandable: expandable,
          forceExpand: forceExpand,
          body: const Text(_bodyText),
          footer: footer,
        ),
      ),
    ),
  ),
);

Future<void> _pump(WidgetTester tester, Widget widget) =>
    tester.pumpWidget(widget);

Future<void> _elapse(WidgetTester tester, int ms) =>
    tester.pump(Duration(milliseconds: ms));

final _expanded = find.byIcon(Icons.expand_less_rounded);
final _collapsed = find.byIcon(Icons.expand_more_rounded);

void main() {
  group('TimelineEntryShell state machine', () {
    testWidgets('active expands; complete auto-collapses after 500 ms', (
      tester,
    ) async {
      await _pump(tester, _shell(state: EntryState.active));
      expect(_expanded, findsOneWidget);
      expect(find.text(_bodyText), findsOneWidget);

      await _pump(tester, _shell());
      await tester.pump(const Duration(milliseconds: 499));
      expect(_expanded, findsOneWidget, reason: 'delay not yet elapsed');

      await _elapse(tester, 2);
      expect(_collapsed, findsOneWidget);
      expect(find.text(_bodyText), findsNothing);
    });

    testWidgets('compact density does not auto-expand when active', (
      tester,
    ) async {
      await _pump(
        tester,
        _shell(state: EntryState.active, density: TimelineDensity.compact),
      );

      expect(_collapsed, findsOneWidget);
      expect(find.text(_bodyText), findsNothing);
    });

    testWidgets('tap toggles and pins; a pin survives complete and stale', (
      tester,
    ) async {
      await _pump(tester, _shell(state: EntryState.active));

      await tester.tap(_expanded);
      await tester.pump();
      expect(_collapsed, findsOneWidget, reason: 'first tap collapses');

      await tester.tap(_collapsed);
      await tester.pump();
      expect(_expanded, findsOneWidget, reason: 'second tap expands and pins');

      await _pump(tester, _shell());
      await _elapse(tester, 600);
      expect(_expanded, findsOneWidget, reason: 'pin survives complete');

      await _pump(tester, _shell(state: EntryState.stale));
      expect(_expanded, findsOneWidget, reason: 'pin survives stale');
    });

    testWidgets('tap on a non-expandable shell does nothing', (tester) async {
      await _pump(tester, _shell(expandable: false));

      await tester.tap(find.text('tool'));
      await tester.pump();

      expect(_expanded, findsNothing);
      expect(_collapsed, findsNothing);
      expect(find.text(_bodyText), findsNothing);
    });

    testWidgets('forceExpand shows the body without a tap', (tester) async {
      await _pump(tester, _shell(forceExpand: true));

      expect(find.text(_bodyText), findsOneWidget);
    });

    testWidgets('reduced motion collapses on complete without a timer', (
      tester,
    ) async {
      await _pump(
        tester,
        _shell(state: EntryState.active, disableAnimations: true),
      );
      expect(_expanded, findsOneWidget);
      expect(find.byType(AnimatedSize), findsNothing);

      await tester.pumpWidget(_shell(disableAnimations: true));
      await tester.pump();

      // No time elapsed, and no pending timer fails the test on teardown.
      expect(_collapsed, findsOneWidget);
      expect(find.byType(AnimatedSize), findsNothing);
    });

    testWidgets('stale renders at 0.6 opacity and collapses', (tester) async {
      await _pump(tester, _shell(state: EntryState.active));
      await _pump(tester, _shell(state: EntryState.stale));

      expect(tester.widget<Opacity>(find.byType(Opacity)).opacity, 0.6);
      expect(_collapsed, findsOneWidget);
    });
  });

  group('TimelineEntryShell footer', () {
    testWidgets('renders while collapsed and the body does not', (
      tester,
    ) async {
      await _pump(tester, _shell(footer: const Text(_footerText)));

      expect(_collapsed, findsOneWidget);
      expect(find.text(_bodyText), findsNothing);
      expect(find.text(_footerText), findsOneWidget);
    });

    testWidgets('renders after the body when expanded', (tester) async {
      await _pump(
        tester,
        _shell(state: EntryState.active, footer: const Text(_footerText)),
      );

      final body = tester.getTopLeft(find.text(_bodyText));
      final footer = tester.getTopLeft(find.text(_footerText));
      expect(footer.dy, greaterThan(body.dy));
    });

    testWidgets('hosts an interactive widget that stays tappable', (
      tester,
    ) async {
      var taps = 0;
      await _pump(
        tester,
        _shell(
          footer: TextButton(
            onPressed: () => taps++,
            child: const Text('Approve'),
          ),
        ),
      );

      await tester.tap(find.text('Approve'));

      expect(taps, 1);
      expect(_collapsed, findsOneWidget, reason: 'footer tap does not toggle');
    });

    testWidgets('fills the entry width so an app frame can size itself', (
      tester,
    ) async {
      await _pump(
        tester,
        _shell(
          footer: const Align(child: SizedBox(key: Key('frame'))),
        ),
      );

      final slot = tester.getSize(
        find
            .ancestor(
              of: find.byKey(const Key('frame')),
              matching: find.byType(Align),
            )
            .first,
      );
      expect(slot.width, 640);
    });
  });

  group('TimelineEntryShell awaitingInput', () {
    testWidgets('starts expanded at every density', (tester) async {
      for (final density in TimelineDensity.values) {
        await _pump(
          tester,
          _shell(state: EntryState.awaitingInput, density: density),
        );

        expect(_expanded, findsOneWidget, reason: density.name);
      }
    });

    testWidgets('expands a collapsed compact entry', (tester) async {
      await _pump(tester, _shell(density: TimelineDensity.compact));
      expect(_collapsed, findsOneWidget);

      await _pump(
        tester,
        _shell(
          state: EntryState.awaitingInput,
          density: TimelineDensity.compact,
        ),
      );

      expect(_expanded, findsOneWidget);
    });

    testWidgets('cancels a pending auto-collapse timer', (tester) async {
      await _pump(tester, _shell(state: EntryState.active));
      await tester.pumpWidget(_shell()); // schedules the 500 ms collapse
      await tester.pumpWidget(_shell(state: EntryState.awaitingInput));

      await _elapse(tester, 600);

      expect(_expanded, findsOneWidget);
    });

    testWidgets('a tap does not collapse it', (tester) async {
      await _pump(tester, _shell(state: EntryState.awaitingInput));

      await tester.tap(_expanded);
      await tester.pump();

      expect(_expanded, findsOneWidget);
      expect(find.text(_bodyText), findsOneWidget);
    });

    testWidgets('shows an attention accent only while awaiting input', (
      tester,
    ) async {
      Container header() =>
          tester.widget<Container>(find.byKey(TimelineEntryShell.headerKey));

      await _pump(tester, _shell(state: EntryState.awaitingInput));
      expect(header().foregroundDecoration, isNotNull);

      await _pump(tester, _shell(state: EntryState.active));
      expect(header().foregroundDecoration, isNull);
    });

    testWidgets('to complete schedules auto-collapse unless pinned', (
      tester,
    ) async {
      await _pump(tester, _shell(state: EntryState.awaitingInput));
      await _pump(tester, _shell());
      await tester.pump(const Duration(milliseconds: 499));
      expect(_expanded, findsOneWidget);

      await _elapse(tester, 2);
      expect(_collapsed, findsOneWidget);
    });

    testWidgets('to complete collapses at once under reduced motion', (
      tester,
    ) async {
      await _pump(
        tester,
        _shell(state: EntryState.awaitingInput, disableAnimations: true),
      );

      await tester.pumpWidget(_shell(disableAnimations: true));
      await tester.pump();

      expect(_collapsed, findsOneWidget);
    });

    testWidgets('to active stays expanded at every density', (tester) async {
      for (final density in TimelineDensity.values) {
        await _pump(
          tester,
          _shell(state: EntryState.awaitingInput, density: density),
        );
        await _pump(tester, _shell(state: EntryState.active, density: density));
        await _elapse(tester, 1000);

        expect(_expanded, findsOneWidget, reason: density.name);
      }
    });

    testWidgets('to stale collapses and dims', (tester) async {
      await _pump(tester, _shell(state: EntryState.awaitingInput));
      await _pump(tester, _shell(state: EntryState.stale));

      expect(_collapsed, findsOneWidget);
      expect(tester.widget<Opacity>(find.byType(Opacity)).opacity, 0.6);
    });

    testWidgets('to stale stays expanded when pinned', (tester) async {
      await _pump(tester, _shell(state: EntryState.active));
      await tester.tap(_expanded);
      await tester.pump();
      await tester.tap(_collapsed);
      await tester.pump();

      await _pump(tester, _shell(state: EntryState.awaitingInput));
      await _pump(tester, _shell(state: EntryState.stale));

      expect(_expanded, findsOneWidget);
      expect(tester.widget<Opacity>(find.byType(Opacity)).opacity, 0.6);
    });

    testWidgets('a pin made while active survives awaitingInput to complete', (
      tester,
    ) async {
      await _pump(tester, _shell(state: EntryState.active));
      await tester.tap(_expanded);
      await tester.pump();
      await tester.tap(_collapsed);
      await tester.pump();
      expect(_expanded, findsOneWidget);

      await _pump(tester, _shell(state: EntryState.awaitingInput));
      await _pump(tester, _shell());
      await _elapse(tester, 600);

      expect(_expanded, findsOneWidget);
    });
  });
}
