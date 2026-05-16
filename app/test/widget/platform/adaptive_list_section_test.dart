import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_list_section.dart';
import 'package:assistant_app/shared/platform/adaptive_list_tile.dart';

Widget _harness(Widget child) {
  return MaterialApp(home: Material(child: child));
}

void main() {
  group('AdaptiveListSection', () {
    testWidgets('renders CupertinoListSection on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(
          AdaptiveListSection(
            header: const Text('Section'),
            children: const [AdaptiveListTile(title: Text('Item'))],
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoListSection), findsOneWidget);
      expect(find.text('Section'), findsOneWidget);
      expect(find.text('Item'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders Material header + column on Android', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(
          AdaptiveListSection(
            header: const Text('Section'),
            children: const [AdaptiveListTile(title: Text('Item'))],
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoListSection), findsNothing);
      expect(find.text('Section'), findsOneWidget);
      expect(find.text('Item'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders without header on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(
          AdaptiveListSection(
            children: const [AdaptiveListTile(title: Text('Item'))],
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoListSection), findsOneWidget);
      expect(find.text('Item'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
