import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_icon.dart';

Widget _harness(Widget child) {
  return MaterialApp(
    home: Material(child: Center(child: child)),
  );
}

void main() {
  group('AdaptiveIcon', () {
    testWidgets('renders Cupertino icon on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(
          const AdaptiveIcon(
            cupertino: CupertinoIcons.add,
            material: Icons.add,
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byIcon(CupertinoIcons.add), findsOneWidget);
      expect(find.byIcon(Icons.add), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders Material icon on Android', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(
          const AdaptiveIcon(
            cupertino: CupertinoIcons.add,
            material: Icons.add,
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.add), findsOneWidget);
      expect(find.byIcon(CupertinoIcons.add), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders Material icon on macOS (fallback)', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;

      await tester.pumpWidget(
        _harness(
          const AdaptiveIcon(
            cupertino: CupertinoIcons.delete,
            material: Icons.delete,
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.delete), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('size and color flow through', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(
          const AdaptiveIcon(
            cupertino: CupertinoIcons.add,
            material: Icons.add,
            size: 32,
            color: Color(0xFFFF0000),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final ic = tester.widget<Icon>(find.byType(Icon));
      expect(ic.size, 32);
      expect(ic.color, const Color(0xFFFF0000));

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
