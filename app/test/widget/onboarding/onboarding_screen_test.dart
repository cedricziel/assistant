import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/onboarding/onboarding_provider.dart';
import 'package:assistant_app/features/onboarding/onboarding_screen.dart';

GoRouter _router() => GoRouter(
  initialLocation: '/onboarding',
  routes: [
    GoRoute(path: '/onboarding', builder: (ctx, s) => const OnboardingScreen()),
    GoRoute(path: '/chat', builder: (ctx, s) => const Scaffold()),
  ],
);

TemplateResponse _makeTemplate({
  required String id,
  required String name,
  String description = '',
}) {
  return TemplateResponse(
    (b) => b
      ..id = id
      ..name = name
      ..description = description
      ..createdAt = '2025-01-01T00:00:00Z',
  );
}

void main() {
  group('OnboardingScreen', () {
    testWidgets('renders welcome heading', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            templatesProvider.overrideWith(() => _StubTemplatesNotifier([])),
          ],
          child: MaterialApp.router(routerConfig: _router()),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.textContaining('Welcome'), findsOneWidget);
    });

    testWidgets('shows "Create blank persona" button when no templates', (
      tester,
    ) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            templatesProvider.overrideWith(() => _StubTemplatesNotifier([])),
          ],
          child: MaterialApp.router(routerConfig: _router()),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Create blank persona'), findsOneWidget);
    });

    testWidgets('shows template list when templates available', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            templatesProvider.overrideWith(
              () => _StubTemplatesNotifier([
                _makeTemplate(
                  id: 'coding-assistant',
                  name: 'Coding Assistant',
                  description: 'Helps with programming tasks',
                ),
                _makeTemplate(id: 'writer', name: 'Writing Helper'),
              ]),
            ),
          ],
          child: MaterialApp.router(routerConfig: _router()),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Templates'), findsOneWidget);
      expect(find.text('Coding Assistant'), findsOneWidget);
      expect(find.text('Helps with programming tasks'), findsOneWidget);
      expect(find.text('Writing Helper'), findsOneWidget);
    });

    testWidgets('shows skip button', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            templatesProvider.overrideWith(() => _StubTemplatesNotifier([])),
          ],
          child: MaterialApp.router(routerConfig: _router()),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Skip for now'), findsOneWidget);
    });

    testWidgets('skip button navigates to chat', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            templatesProvider.overrideWith(() => _StubTemplatesNotifier([])),
          ],
          child: MaterialApp.router(routerConfig: _router()),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.text('Skip for now'));
      await tester.pumpAndSettle();

      expect(find.byType(OnboardingScreen), findsNothing);
    });
  });
}

class _StubTemplatesNotifier extends AsyncNotifier<List<TemplateResponse>>
    implements TemplatesNotifier {
  _StubTemplatesNotifier(this._templates);
  final List<TemplateResponse> _templates;

  @override
  Future<List<TemplateResponse>> build() async => _templates;
}
