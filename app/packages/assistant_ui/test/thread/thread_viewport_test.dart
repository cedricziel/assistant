import 'package:assistant_ui/assistant_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';

const _fab = Key('scroll_to_bottom_button');

/// A 400 dp tall viewport over [count] rows of 50 dp each.
Widget _harness({
  required int count,
  required int revision,
  ThreadViewportController? controller,
  bool disableAnimations = false,
}) => MaterialApp(
  home: Builder(
    builder: (context) => MediaQuery(
      data: MediaQuery.of(context)
          .copyWith(disableAnimations: disableAnimations),
      child: Scaffold(
        body: SizedBox(
          height: 400,
          child: ThreadViewport(
            itemCount: count,
            revision: revision,
            controller: controller,
            itemBuilder: (context, index) =>
                SizedBox(height: 50, child: Text('row $index')),
          ),
        ),
      ),
    ),
  ),
);

ScrollPosition _position(WidgetTester tester) =>
    tester.state<ScrollableState>(find.byType(Scrollable)).position;

void main() {
  group('ThreadViewport', () {
    testWidgets('hides the button when at the bottom', (tester) async {
      await tester.pumpWidget(_harness(count: 40, revision: 0));

      expect(find.byKey(_fab), findsNothing);
    });

    testWidgets('opens at the bottom when it appears with content', (
      tester,
    ) async {
      await tester.pumpWidget(_harness(count: 40, revision: 0));
      await tester.pumpAndSettle();

      final position = _position(tester);
      expect(position.pixels, position.maxScrollExtent);
      expect(position.maxScrollExtent, greaterThan(0));
    });

    testWidgets('shows the button once more than 80 dp from the bottom', (
      tester,
    ) async {
      await tester.pumpWidget(_harness(count: 40, revision: 0));
      final position = _position(tester);

      position.jumpTo(position.maxScrollExtent - 79);
      await tester.pump();
      expect(find.byKey(_fab), findsNothing, reason: '79 dp is near the end');

      position.jumpTo(position.maxScrollExtent - 81);
      await tester.pump();
      expect(
        find.byKey(_fab),
        findsOneWidget,
        reason: '81 dp is scrolled away',
      );
    });

    testWidgets('a higher revision at the bottom animates to the bottom', (
      tester,
    ) async {
      await tester.pumpWidget(_harness(count: 40, revision: 0));
      final position = _position(tester);
      position.jumpTo(position.maxScrollExtent);
      await tester.pump();

      await tester.pumpWidget(_harness(count: 41, revision: 1));
      await tester.pumpAndSettle();

      expect(position.pixels, position.maxScrollExtent);
      expect(position.maxScrollExtent, greaterThan(1600));
      expect(find.byKey(_fab), findsNothing);
    });

    testWidgets('an unchanged revision does not scroll', (tester) async {
      await tester.pumpWidget(_harness(count: 40, revision: 0));
      final position = _position(tester);
      position.jumpTo(position.maxScrollExtent);
      await tester.pump();

      await tester.pumpWidget(_harness(count: 41, revision: 0));
      await tester.pumpAndSettle();

      expect(position.pixels, lessThan(position.maxScrollExtent));
    });

    testWidgets('a higher revision while scrolled away does not move', (
      tester,
    ) async {
      await tester.pumpWidget(_harness(count: 40, revision: 0));
      final position = _position(tester);
      position.jumpTo(500);
      await tester.pump();

      await tester.pumpWidget(_harness(count: 41, revision: 1));
      await tester.pumpAndSettle();

      expect(position.pixels, 500);
      expect(find.byKey(_fab), findsOneWidget);
    });

    testWidgets('the button returns to the bottom and hides', (tester) async {
      await tester.pumpWidget(_harness(count: 40, revision: 0));
      _position(tester).jumpTo(500);
      await tester.pump();

      await tester.tap(find.byKey(_fab));
      await tester.pumpAndSettle();

      final position = _position(tester);
      expect(position.pixels, position.maxScrollExtent);
      expect(find.byKey(_fab), findsNothing);
    });

    testWidgets('scrollToBottom() scrolls even when scrolled away', (
      tester,
    ) async {
      final controller = ThreadViewportController();
      await tester.pumpWidget(
        _harness(count: 40, revision: 0, controller: controller),
      );
      _position(tester).jumpTo(500);
      await tester.pump();

      controller.scrollToBottom();
      await tester.pumpAndSettle();

      final position = _position(tester);
      expect(position.pixels, position.maxScrollExtent);
    });

    testWidgets('scrollToBottomIfNear() only scrolls within 200 dp', (
      tester,
    ) async {
      final controller = ThreadViewportController();
      await tester.pumpWidget(
        _harness(count: 40, revision: 0, controller: controller),
      );
      final position = _position(tester);

      position.jumpTo(position.maxScrollExtent - 300);
      await tester.pump();
      controller.scrollToBottomIfNear();
      await tester.pumpAndSettle();
      expect(position.pixels, position.maxScrollExtent - 300);

      position.jumpTo(position.maxScrollExtent - 150);
      await tester.pump();
      controller.scrollToBottomIfNear();
      await tester.pumpAndSettle();
      expect(position.pixels, position.maxScrollExtent);
    });

    testWidgets('a detached controller ignores scroll requests', (
      tester,
    ) async {
      final controller = ThreadViewportController();

      controller.scrollToBottom();
      controller.scrollToBottomIfNear();
      await tester.pump();
    });

    testWidgets('an empty thread tolerates revision changes', (tester) async {
      await tester.pumpWidget(_harness(count: 0, revision: 0));
      await tester.pumpWidget(_harness(count: 0, revision: 1));
      await tester.pumpAndSettle();

      expect(find.byKey(_fab), findsNothing);
    });

    testWidgets('reduced motion opens on the last item after one pump', (
      tester,
    ) async {
      await tester.pumpWidget(
        _harness(count: 40, revision: 0, disableAnimations: true),
      );

      final position = _position(tester);
      expect(position.pixels, position.maxScrollExtent);
    });

    testWidgets('reduced motion jumps when the button is pressed', (
      tester,
    ) async {
      await tester.pumpWidget(
        _harness(count: 40, revision: 0, disableAnimations: true),
      );
      _position(tester).jumpTo(500);
      await tester.pump();

      await tester.tap(find.byKey(_fab));
      await tester.pump();

      final position = _position(tester);
      expect(position.pixels, position.maxScrollExtent);
    });

    testWidgets('reduced motion jumps when a new message arrives', (
      tester,
    ) async {
      await tester.pumpWidget(
        _harness(count: 40, revision: 0, disableAnimations: true),
      );
      final position = _position(tester);

      await tester.pumpWidget(
        _harness(count: 41, revision: 1, disableAnimations: true),
      );
      await tester.pump();

      expect(position.pixels, position.maxScrollExtent);
      expect(position.maxScrollExtent, greaterThan(1600));
    });

    testWidgets('without reduced motion scrolling is still animated', (
      tester,
    ) async {
      await tester.pumpWidget(_harness(count: 40, revision: 0));
      final position = _position(tester);
      position.jumpTo(500);
      await tester.pump();

      await tester.tap(find.byKey(_fab));
      await tester.pump();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 16));

      expect(position.pixels, greaterThan(500));
      expect(position.pixels, lessThan(position.maxScrollExtent));
    });
  });
}
