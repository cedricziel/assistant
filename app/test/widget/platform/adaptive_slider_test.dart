import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_slider.dart';

Widget _harness(Widget child) {
  return MaterialApp(
    home: Material(child: Center(child: child)),
  );
}

void main() {
  group('AdaptiveSlider', () {
    testWidgets('renders CupertinoSlider on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(AdaptiveSlider(value: 0.5, onChanged: (_) {})),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoSlider), findsOneWidget);
      expect(find.byType(Slider), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders Slider on Android', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(AdaptiveSlider(value: 0.5, onChanged: (_) {})),
      );
      await tester.pumpAndSettle();

      expect(find.byType(Slider), findsOneWidget);
      expect(find.byType(CupertinoSlider), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('value and bounds flow through on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(
          AdaptiveSlider(value: 25, min: 0, max: 100, onChanged: (_) {}),
        ),
      );
      await tester.pumpAndSettle();

      final s = tester.widget<CupertinoSlider>(find.byType(CupertinoSlider));
      expect(s.value, 25);
      expect(s.min, 0);
      expect(s.max, 100);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('null onChanged disables Material slider', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(const AdaptiveSlider(value: 0.5, onChanged: null)),
      );
      await tester.pumpAndSettle();

      final s = tester.widget<Slider>(find.byType(Slider));
      expect(s.onChanged, isNull);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
