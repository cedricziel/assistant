import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_sliver_nav_bar.dart';

Widget _harness(Widget sliver) {
  return MaterialApp(
    home: Material(
      child: CustomScrollView(
        slivers: [
          sliver,
          const SliverToBoxAdapter(child: SizedBox(height: 100)),
        ],
      ),
    ),
  );
}

void main() {
  group('AdaptiveSliverNavBar', () {
    testWidgets('renders CupertinoSliverNavigationBar on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(const AdaptiveSliverNavBar(title: 'Test Title')),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoSliverNavigationBar), findsOneWidget);
      expect(find.byType(SliverAppBar), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders SliverAppBar on Android', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(const AdaptiveSliverNavBar(title: 'Test Title')),
      );
      await tester.pumpAndSettle();

      expect(find.byType(SliverAppBar), findsOneWidget);
      expect(find.byType(CupertinoSliverNavigationBar), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders SliverAppBar on macOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;

      await tester.pumpWidget(
        _harness(const AdaptiveSliverNavBar(title: 'Test Title')),
      );
      await tester.pumpAndSettle();

      expect(find.byType(SliverAppBar), findsOneWidget);
      expect(find.byType(CupertinoSliverNavigationBar), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders title text on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(const AdaptiveSliverNavBar(title: 'Hello iOS')),
      );
      await tester.pumpAndSettle();

      expect(find.text('Hello iOS'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders title text on Material', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(const AdaptiveSliverNavBar(title: 'Hello Material')),
      );
      await tester.pumpAndSettle();

      // SliverAppBar.large renders both a collapsed and an expanded Text
      // for the title, so at least one occurrence is enough to verify
      // the title flows through.
      expect(find.text('Hello Material'), findsAtLeastNWidgets(1));

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders trailing actions on Material', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(
          AdaptiveSliverNavBar(
            title: 'With Actions',
            actions: [
              IconButton(icon: const Icon(Icons.add), onPressed: () {}),
            ],
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.add), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
