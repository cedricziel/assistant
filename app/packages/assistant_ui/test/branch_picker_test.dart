import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/golden.dart';

Finder _button(String tooltip) => find.ancestor(
  of: find.byTooltip(tooltip),
  matching: find.byType(IconButton),
);
Finder _previous() => _button('Previous version');
Finder _next() => _button('Next version');

bool _enabled(WidgetTester tester, Finder finder) =>
    tester.widget<IconButton>(finder).onPressed != null;

void main() {
  group('BranchPicker', () {
    testWidgets('shows the one-based position and enables both controls', (
      tester,
    ) async {
      await pumpGolden(
        tester,
        BranchPicker(index: 1, count: 3, onPrevious: () {}, onNext: () {}),
      );

      expect(find.text('2/3'), findsOneWidget);
      expect(_enabled(tester, _previous()), isTrue);
      expect(_enabled(tester, _next()), isTrue);
    });

    testWidgets('disables previous at the first version', (tester) async {
      var previous = 0;
      await pumpGolden(
        tester,
        BranchPicker(
          index: 0,
          count: 3,
          onPrevious: () => previous++,
          onNext: () {},
        ),
      );

      expect(_enabled(tester, _previous()), isFalse);
      expect(_enabled(tester, _next()), isTrue);
      await tester.tap(_previous());
      expect(previous, 0, reason: 'disabled control must not fire');
    });

    testWidgets('disables next at the last version', (tester) async {
      var next = 0;
      await pumpGolden(
        tester,
        BranchPicker(
          index: 2,
          count: 3,
          onPrevious: () {},
          onNext: () => next++,
        ),
      );

      expect(_enabled(tester, _next()), isFalse);
      expect(_enabled(tester, _previous()), isTrue);
      await tester.tap(_next());
      expect(next, 0);
    });

    testWidgets('disables both controls while busy', (tester) async {
      await pumpGolden(
        tester,
        BranchPicker(
          index: 1,
          count: 3,
          busy: true,
          onPrevious: () {},
          onNext: () {},
        ),
      );

      expect(_enabled(tester, _previous()), isFalse);
      expect(_enabled(tester, _next()), isFalse);
    });

    testWidgets('invokes the callbacks when tapped', (tester) async {
      final taps = <String>[];
      await pumpGolden(
        tester,
        BranchPicker(
          index: 1,
          count: 3,
          onPrevious: () => taps.add('previous'),
          onNext: () => taps.add('next'),
        ),
      );

      await tester.tap(_previous());
      await tester.tap(_next());

      expect(taps, ['previous', 'next']);
    });

    testWidgets('exposes the "Version 2 of 3" semantics label', (tester) async {
      final handle = tester.ensureSemantics();
      await pumpGolden(
        tester,
        BranchPicker(index: 1, count: 3, onPrevious: () {}, onNext: () {}),
      );

      expect(find.bySemanticsLabel('Version 2 of 3'), findsOneWidget);
      handle.dispose();
    });
  });
}
