import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_scaffold.dart';

void main() {
  group('AdaptiveScaffold', () {
    testWidgets('renders CupertinoPageScaffold on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        const MaterialApp(
          home: AdaptiveScaffold(
            navigationBar: CupertinoNavigationBar(middle: Text('Test')),
            appBar: null,
            body: Text('body'),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoPageScaffold), findsOneWidget);
      // No Material Scaffold on iOS path
      expect(find.byType(Scaffold), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders Scaffold on macOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;

      await tester.pumpWidget(
        MaterialApp(
          home: AdaptiveScaffold(
            appBar: AppBar(title: const Text('Test')),
            body: const Text('body'),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(Scaffold), findsOneWidget);
      expect(find.byType(CupertinoPageScaffold), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
