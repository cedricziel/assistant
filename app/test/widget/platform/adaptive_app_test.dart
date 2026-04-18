import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/shared/platform/adaptive_app.dart';

GoRouter _makeRouter() => GoRouter(
  initialLocation: '/',
  routes: [
    GoRoute(
      path: '/',
      builder: (_, _) => const Scaffold(body: Text('hi')),
    ),
  ],
);

void main() {
  group('AdaptiveApp', () {
    testWidgets('renders MaterialApp on Material platforms', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      await tester.pumpWidget(AdaptiveApp(routerConfig: _makeRouter()));

      expect(find.byType(MaterialApp), findsOneWidget);
      expect(find.byType(CupertinoApp), findsNothing);
      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders CupertinoApp on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      await tester.pumpWidget(AdaptiveApp(routerConfig: _makeRouter()));

      expect(find.byType(CupertinoApp), findsOneWidget);
      expect(find.byType(MaterialApp), findsNothing);
      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders MaterialApp on macOS (for now)', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      await tester.pumpWidget(AdaptiveApp(routerConfig: _makeRouter()));

      expect(find.byType(MaterialApp), findsOneWidget);
      expect(find.byType(CupertinoApp), findsNothing);
      debugDefaultTargetPlatformOverride = null;
    });
  });
}
