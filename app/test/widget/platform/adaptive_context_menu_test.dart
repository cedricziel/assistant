import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_context_menu.dart';

Widget _harness(Widget child) {
  return MaterialApp(
    home: Scaffold(body: Center(child: child)),
  );
}

void main() {
  group('AdaptiveMediaContextMenu', () {
    final actions = [
      AdaptiveContextMenuAction(
        label: 'Save Image',
        onPressed: () {},
        cupertinoIcon: CupertinoIcons.square_arrow_down,
        materialIcon: Icons.save_alt,
      ),
      AdaptiveContextMenuAction(
        label: 'Share',
        onPressed: () {},
        cupertinoIcon: CupertinoIcons.share,
        materialIcon: Icons.share,
      ),
    ];

    testWidgets('uses CupertinoContextMenu on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(
          AdaptiveMediaContextMenu(
            actions: actions,
            child: const SizedBox(width: 100, height: 100, key: Key('media')),
          ),
        ),
      );

      expect(
        find.byType(CupertinoContextMenu),
        findsOneWidget,
        reason: 'Should use CupertinoContextMenu on iOS',
      );

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('uses CupertinoContextMenu on macOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;

      await tester.pumpWidget(
        _harness(
          AdaptiveMediaContextMenu(
            actions: actions,
            child: const SizedBox(width: 100, height: 100, key: Key('media')),
          ),
        ),
      );

      expect(
        find.byType(CupertinoContextMenu),
        findsOneWidget,
        reason: 'Should use CupertinoContextMenu on macOS',
      );

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('uses GestureDetector on Android (Material)', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(
          AdaptiveMediaContextMenu(
            actions: actions,
            child: const SizedBox(width: 100, height: 100, key: Key('media')),
          ),
        ),
      );

      expect(
        find.byType(CupertinoContextMenu),
        findsNothing,
        reason: 'Should NOT use CupertinoContextMenu on Android',
      );
      expect(
        find.byType(GestureDetector),
        findsOneWidget,
        reason: 'Should use GestureDetector on Android',
      );

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('long-press opens popup menu on Material platform', (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(
          AdaptiveMediaContextMenu(
            actions: actions,
            child: const SizedBox(width: 100, height: 100, key: Key('media')),
          ),
        ),
      );

      await tester.longPress(find.byKey(const Key('media')));
      await tester.pumpAndSettle();

      expect(find.text('Save Image'), findsOneWidget);
      expect(find.text('Share'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
