import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Widget _host(Widget child) => MaterialApp(
  home: Scaffold(body: Center(child: child)),
);

/// A stand-in for the rendered A2UI tree: a field and a button.
class _Probe extends StatelessWidget {
  const _Probe({required this.onTap});

  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      const TextField(key: Key('probe-field')),
      ElevatedButton(
        key: const Key('probe-button'),
        onPressed: onTap,
        child: const Text('Send'),
      ),
    ],
  );
}

void main() {
  group('SurfaceRecord', () {
    test('maps wire statuses and falls back to active', () {
      expect(SurfaceStatus.fromWire('submitted'), SurfaceStatus.submitted);
      expect(SurfaceStatus.fromWire('active'), SurfaceStatus.active);
      expect(SurfaceStatus.fromWire('whatever'), SurfaceStatus.active);
    });

    test('is closed only when active and not answerable', () {
      SurfaceRecord record(SurfaceStatus status, {required bool answerable}) =>
          SurfaceRecord(
            surfaceId: 's1',
            anchorMessageId: 'm1',
            anchorCallIndex: 0,
            title: 'Trip',
            fallbackText: 'Where to?',
            status: status,
            answerable: answerable,
          );

      expect(record(SurfaceStatus.active, answerable: true).isClosed, isFalse);
      expect(record(SurfaceStatus.active, answerable: false).isClosed, isTrue);
      expect(
        record(SurfaceStatus.submitted, answerable: false).isClosed,
        isFalse,
      );
    });

    test('has an optional surface timeline type', () {
      expect(TimelineEntryType.values, contains(TimelineEntryType.surface));
    });
  });

  group('SurfaceFrame', () {
    testWidgets('fallback-only shows the fallback text', (tester) async {
      await tester.pumpWidget(
        _host(
          const SurfaceFrame(
            title: 'Trip details',
            status: SurfaceStatus.active,
            answerable: true,
            fallbackText: 'Where would you like to go?',
          ),
        ),
      );

      expect(find.text('Trip details'), findsOneWidget);
      expect(find.text('Where would you like to go?'), findsOneWidget);
      expect(find.byIcon(Icons.error_outline), findsNothing);
    });

    testWidgets('the error flag shows the fallback, not the content', (
      tester,
    ) async {
      await tester.pumpWidget(
        _host(
          SurfaceFrame(
            title: 'Trip details',
            status: SurfaceStatus.active,
            answerable: true,
            fallbackText: 'Where would you like to go?',
            hasError: true,
            content: _Probe(onTap: () {}),
          ),
        ),
      );

      expect(find.text('Where would you like to go?'), findsOneWidget);
      expect(find.byKey(const Key('probe-button')), findsNothing);
      expect(find.byIcon(Icons.error_outline), findsOneWidget);
    });

    testWidgets('an answerable active surface fires its actions', (
      tester,
    ) async {
      var taps = 0;
      await tester.pumpWidget(
        _host(
          SurfaceFrame(
            title: 'Trip details',
            status: SurfaceStatus.active,
            answerable: true,
            fallbackText: 'Where would you like to go?',
            content: _Probe(onTap: () => taps++),
          ),
        ),
      );

      await tester.tap(find.byKey(const Key('probe-button')));

      expect(taps, 1);
      expect(find.text('Closed'), findsNothing);
      expect(find.text('via form'), findsNothing);
    });

    testWidgets('submitted shows the label and a disabled child', (
      tester,
    ) async {
      var taps = 0;
      await tester.pumpWidget(
        _host(
          SurfaceFrame(
            title: 'Trip details',
            status: SurfaceStatus.submitted,
            answerable: false,
            fallbackText: 'Where would you like to go?',
            content: _Probe(onTap: () => taps++),
          ),
        ),
      );

      expect(find.text('Submitted “Trip details”'), findsOneWidget);
      expect(find.text('via form'), findsOneWidget);
      expect(find.byKey(const Key('probe-button')), findsOneWidget);

      await tester.tap(
        find.byKey(const Key('probe-button')),
        warnIfMissed: false,
      );
      await tester.tap(
        find.byKey(const Key('probe-field')),
        warnIfMissed: false,
      );

      expect(taps, 0, reason: 'no action fires on a submitted surface');
      expect(
        FocusManager.instance.primaryFocus?.context?.widget,
        isNot(isA<EditableText>()),
        reason: 'the field cannot take focus',
      );
    });

    testWidgets('closed shows the label and a disabled child', (tester) async {
      var taps = 0;
      await tester.pumpWidget(
        _host(
          SurfaceFrame(
            title: 'Trip details',
            status: SurfaceStatus.active,
            answerable: false,
            fallbackText: 'Where would you like to go?',
            content: _Probe(onTap: () => taps++),
          ),
        ),
      );

      expect(find.text('Trip details'), findsOneWidget);
      expect(find.text('Closed'), findsOneWidget);
      expect(find.text('via form'), findsNothing);

      await tester.tap(
        find.byKey(const Key('probe-button')),
        warnIfMissed: false,
      );

      expect(taps, 0, reason: 'no action fires on a closed surface');
    });

    testWidgets('pending shows progress instead of the content', (
      tester,
    ) async {
      await tester.pumpWidget(
        _host(
          SurfaceFrame(
            title: 'Trip details',
            status: SurfaceStatus.active,
            answerable: true,
            fallbackText: 'Where would you like to go?',
            pending: true,
            content: _Probe(onTap: () {}),
          ),
        ),
      );

      expect(find.byType(LinearProgressIndicator), findsOneWidget);
      expect(find.byKey(const Key('probe-button')), findsNothing);
    });

    testWidgets('awaiting input maps to the awaitingInput entry state', (
      tester,
    ) async {
      const waiting = SurfaceFrame(
        title: 'Trip details',
        status: SurfaceStatus.active,
        answerable: true,
        fallbackText: 'Where would you like to go?',
        awaitingInput: true,
      );
      const idle = SurfaceFrame(
        title: 'Trip details',
        status: SurfaceStatus.active,
        answerable: true,
        fallbackText: 'Where would you like to go?',
      );
      const pending = SurfaceFrame(
        title: 'Trip details',
        status: SurfaceStatus.active,
        answerable: true,
        fallbackText: 'Where would you like to go?',
        pending: true,
      );

      expect(waiting.entryState, EntryState.awaitingInput);
      expect(idle.entryState, EntryState.complete);
      expect(pending.entryState, EntryState.active);

      await tester.pumpWidget(_host(waiting));

      expect(find.text('Waiting for your answer'), findsOneWidget);
    });
  });
}
