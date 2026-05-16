import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/shared/edge_swipe_sidebar_toggle.dart';
import 'package:assistant_app/shared/nav_shell.dart';

Widget _harness({required Widget child, required ProviderContainer container}) {
  return UncontrolledProviderScope(
    container: container,
    child: MaterialApp(home: Scaffold(body: child)),
  );
}

Future<void> _drag(
  WidgetTester tester, {
  required Offset start,
  required Offset delta,
  required PointerDeviceKind kind,
}) async {
  final gesture = await tester.startGesture(start, kind: kind);
  // Move in steps so onPointerMove fires.
  const steps = 8;
  for (int i = 1; i <= steps; i++) {
    await gesture.moveBy(Offset(delta.dx / steps, delta.dy / steps));
    await tester.pump(const Duration(milliseconds: 20));
  }
  await gesture.up();
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('EdgeSwipeSidebarToggle', () {
    testWidgets('right-swipe from x=10 expands the sidebar', (tester) async {
      SharedPreferences.setMockInitialValues({
        'assistant.sidebarCollapsed': true,
      });
      final container = ProviderContainer();
      addTearDown(container.dispose);
      expect(await container.read(sidebarCollapsedProvider.future), isTrue);

      await tester.pumpWidget(
        _harness(
          container: container,
          child: EdgeSwipeSidebarToggle(
            child: Container(
              color: Colors.transparent,
              child: const SizedBox.expand(),
            ),
          ),
        ),
      );

      await _drag(
        tester,
        start: const Offset(10, 400),
        delta: const Offset(80, 0),
        kind: PointerDeviceKind.touch,
      );
      await tester.pumpAndSettle();

      expect(
        container.read(sidebarCollapsedProvider).value,
        isFalse,
        reason: 'right-swipe from left edge should expand',
      );
    });

    testWidgets('left-swipe from x=18 collapses the sidebar', (tester) async {
      SharedPreferences.setMockInitialValues({});
      final container = ProviderContainer();
      addTearDown(container.dispose);
      expect(await container.read(sidebarCollapsedProvider.future), isFalse);

      await tester.pumpWidget(
        _harness(
          container: container,
          child: EdgeSwipeSidebarToggle(
            child: Container(
              color: Colors.transparent,
              child: const SizedBox.expand(),
            ),
          ),
        ),
      );

      await _drag(
        tester,
        start: const Offset(18, 400),
        delta: const Offset(-60, 0),
        kind: PointerDeviceKind.touch,
      );
      await tester.pumpAndSettle();

      expect(
        container.read(sidebarCollapsedProvider).value,
        isTrue,
        reason: 'left-swipe from left edge should collapse',
      );
    });

    testWidgets('drag starting outside the edge zone is ignored', (
      tester,
    ) async {
      SharedPreferences.setMockInitialValues({});
      final container = ProviderContainer();
      addTearDown(container.dispose);
      expect(await container.read(sidebarCollapsedProvider.future), isFalse);

      await tester.pumpWidget(
        _harness(
          container: container,
          child: EdgeSwipeSidebarToggle(
            child: Container(
              color: Colors.transparent,
              child: const SizedBox.expand(),
            ),
          ),
        ),
      );

      await _drag(
        tester,
        start: const Offset(200, 400),
        delta: const Offset(120, 0),
        kind: PointerDeviceKind.touch,
      );
      await tester.pumpAndSettle();

      expect(
        container.read(sidebarCollapsedProvider).value,
        isFalse,
        reason: 'drag from far inside the viewport must not toggle',
      );
    });

    testWidgets('small drag from edge below threshold is ignored', (
      tester,
    ) async {
      SharedPreferences.setMockInitialValues({});
      final container = ProviderContainer();
      addTearDown(container.dispose);
      expect(await container.read(sidebarCollapsedProvider.future), isFalse);

      await tester.pumpWidget(
        _harness(
          container: container,
          child: EdgeSwipeSidebarToggle(
            child: Container(
              color: Colors.transparent,
              child: const SizedBox.expand(),
            ),
          ),
        ),
      );

      await _drag(
        tester,
        start: const Offset(10, 400),
        delta: const Offset(20, 0),
        kind: PointerDeviceKind.touch,
      );
      await tester.pumpAndSettle();

      expect(
        container.read(sidebarCollapsedProvider).value,
        isFalse,
        reason: '20 px drag is below the 40 px threshold',
      );
    });

    testWidgets('mouse drag from edge is ignored', (tester) async {
      SharedPreferences.setMockInitialValues({});
      final container = ProviderContainer();
      addTearDown(container.dispose);
      expect(await container.read(sidebarCollapsedProvider.future), isFalse);

      await tester.pumpWidget(
        _harness(
          container: container,
          child: EdgeSwipeSidebarToggle(
            child: Container(
              color: Colors.transparent,
              child: const SizedBox.expand(),
            ),
          ),
        ),
      );

      await _drag(
        tester,
        start: const Offset(5, 400),
        delta: const Offset(120, 0),
        kind: PointerDeviceKind.mouse,
      );
      await tester.pumpAndSettle();

      expect(
        container.read(sidebarCollapsedProvider).value,
        isFalse,
        reason: 'mouse drag must not toggle the sidebar',
      );
    });
  });
}
