import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_button.dart';

Widget _harness(Widget child) {
  return MaterialApp(
    home: Material(child: Center(child: child)),
  );
}

void main() {
  group('AdaptiveButton', () {
    testWidgets('filled renders CupertinoButton.filled on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(
          AdaptiveButton.filled(onPressed: () {}, child: const Text('Save')),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoButton), findsOneWidget);
      expect(find.byType(FilledButton), findsNothing);
      expect(find.text('Save'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('filled renders FilledButton on Android', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(
          AdaptiveButton.filled(onPressed: () {}, child: const Text('Save')),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(FilledButton), findsOneWidget);
      expect(find.byType(CupertinoButton), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('plain renders CupertinoButton on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(
          AdaptiveButton.plain(onPressed: () {}, child: const Text('Cancel')),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoButton), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('plain renders TextButton on Android', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(
          AdaptiveButton.plain(onPressed: () {}, child: const Text('Cancel')),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(TextButton), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('onPressed fires on Material', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      var tapped = false;

      await tester.pumpWidget(
        _harness(
          AdaptiveButton.filled(
            onPressed: () => tapped = true,
            child: const Text('Click'),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byType(FilledButton));
      expect(tapped, isTrue);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('null onPressed renders disabled', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(
          AdaptiveButton.filled(onPressed: null, child: const Text('Click')),
        ),
      );
      await tester.pumpAndSettle();

      final btn = tester.widget<FilledButton>(find.byType(FilledButton));
      expect(btn.onPressed, isNull);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
