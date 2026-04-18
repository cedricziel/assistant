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
  group('Dark mode support', () {
    testWidgets('MaterialApp has a darkTheme', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      await tester.pumpWidget(AdaptiveApp(routerConfig: _makeRouter()));

      final materialApp = tester.widget<MaterialApp>(find.byType(MaterialApp));
      expect(materialApp.darkTheme, isNotNull, reason: 'darkTheme must be set');
      expect(
        materialApp.darkTheme!.brightness,
        Brightness.dark,
        reason: 'darkTheme brightness must be dark',
      );
      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('MaterialApp light theme is set', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      await tester.pumpWidget(AdaptiveApp(routerConfig: _makeRouter()));

      final materialApp = tester.widget<MaterialApp>(find.byType(MaterialApp));
      expect(materialApp.theme, isNotNull, reason: 'theme must be set');
      expect(
        materialApp.theme!.brightness,
        Brightness.light,
        reason: 'theme brightness must be light',
      );
      debugDefaultTargetPlatformOverride = null;
    });
  });
}
