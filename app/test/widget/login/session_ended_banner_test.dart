// Verifies the LoginScreen renders the session-ended banner when the
// `?reason=session-ended` query param is present, and not otherwise.
//
// Spec: openspec/changes/web-session-resilience/specs/web-session-resilience/spec.md
//   "Login screen surfaces the session-ended banner"

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/login/login_screen.dart';

const _bannerKey = Key('session-ended-banner');

GoRouter _router(String initialLocation) => GoRouter(
  initialLocation: initialLocation,
  routes: [GoRoute(path: '/login', builder: (_, _) => const LoginScreen())],
);

void main() {
  testWidgets('banner visible when reason=session-ended is in the URL', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp.router(
          routerConfig: _router('/login?reason=session-ended'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(_bannerKey), findsOneWidget);
    expect(
      find.textContaining('session was reset', findRichText: true),
      findsOneWidget,
    );
  });

  testWidgets('banner absent when no reason query param is present', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(child: MaterialApp.router(routerConfig: _router('/login'))),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(_bannerKey), findsNothing);
  });

  testWidgets('tapping dismiss hides the banner', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp.router(
          routerConfig: _router('/login?reason=session-ended'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(_bannerKey), findsOneWidget);

    await tester.tap(find.byKey(const Key('session-ended-banner-dismiss')));
    await tester.pumpAndSettle();

    expect(find.byKey(_bannerKey), findsNothing);
  });
}
