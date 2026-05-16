import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_text_field.dart';

Widget _harness(Widget child) {
  return MaterialApp(home: Material(child: child));
}

void main() {
  group('AdaptiveTextField', () {
    testWidgets('renders CupertinoTextField on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(_harness(const AdaptiveTextField()));
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoTextField), findsOneWidget);
      expect(find.byType(TextField), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders TextField on Android', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(_harness(const AdaptiveTextField()));
      await tester.pumpAndSettle();

      expect(find.byType(TextField), findsOneWidget);
      expect(find.byType(CupertinoTextField), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('placeholder shows on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(const AdaptiveTextField(placeholder: 'Type here')),
      );
      await tester.pumpAndSettle();

      expect(find.text('Type here'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('placeholder maps to hintText on Material', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(const AdaptiveTextField(placeholder: 'Type here')),
      );
      await tester.pumpAndSettle();

      final tf = tester.widget<TextField>(find.byType(TextField));
      expect(tf.decoration?.hintText, 'Type here');

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('onChanged fires on Material', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      String? captured;

      await tester.pumpWidget(
        _harness(AdaptiveTextField(onChanged: (v) => captured = v)),
      );
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), 'hello');
      expect(captured, 'hello');

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
