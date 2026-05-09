// When the active context is loaded with credentialsCorrupted=true, the
// router MUST redirect to /login?reason=session-ended even though the
// context is non-null.
//
// Spec: openspec/changes/web-session-resilience/specs/web-session-resilience/spec.md
//   "Router treats corrupted contexts as unauthenticated"

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/api/capabilities_provider.dart';
import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';
import 'package:assistant_app/features/login/login_screen.dart';
import 'package:assistant_app/router/app_router.dart';

class _FakeSecureStorage implements SecureKeyValueStorage {
  @override
  Future<String?> read({required String key}) async => null;
  @override
  Future<void> write({required String key, required String value}) async {}
  @override
  Future<void> delete({required String key}) async {}
}

class _CorruptedActiveContextNotifier extends ActiveContextNotifier {
  _CorruptedActiveContextNotifier(this._ctx);
  final AssistantContext _ctx;
  @override
  Future<AssistantContext?> build() async => _ctx;
}

Future<ContextRepository> _makeRepo() async {
  SharedPreferences.setMockInitialValues({});
  final p = await SharedPreferences.getInstance();
  return ContextRepository(prefs: p, secureStorage: _FakeSecureStorage());
}

void main() {
  testWidgets('corrupted active context → /login?reason=session-ended', (
    tester,
  ) async {
    final repo = await _makeRepo();
    final ctx = AssistantContext(
      id: 'ctx-1',
      name: 'Work',
      serverUrl: 'https://work.example.com',
      authMode: AuthMode.oauth2,
      createdAt: DateTime.utc(2024, 1, 1),
      credentialsCorrupted: true,
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          contextRepositoryProvider.overrideWithValue(repo),
          capabilitiesProvider.overrideWith(
            (ref) async => ServerCapabilities.disabled,
          ),
          apiClientProvider.overrideWithValue(null),
          activeContextProvider.overrideWith(
            () => _CorruptedActiveContextNotifier(ctx),
          ),
        ],
        child: Consumer(
          builder: (context, ref, child) {
            final router = ref.watch(routerProvider);
            return MaterialApp.router(routerConfig: router);
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Router should have bounced us to /login with the reason query param.
    expect(find.byType(LoginScreen), findsOneWidget);

    // The actual URI should carry ?reason=session-ended.
    final container = ProviderScope.containerOf(
      tester.element(find.byType(Consumer)),
    );
    final router = container.read(routerProvider);
    final location = router.routerDelegate.currentConfiguration.uri.toString();
    expect(
      location,
      contains('reason=session-ended'),
      reason: 'router must include the reason query param, got: $location',
    );
  });

  testWidgets('healthy active context — no session-ended redirect', (
    tester,
  ) async {
    final repo = await _makeRepo();
    final ctx = AssistantContext(
      id: 'ctx-1',
      name: 'Work',
      serverUrl: 'https://work.example.com',
      authMode: AuthMode.oauth2,
      createdAt: DateTime.utc(2024, 1, 1),
      // credentialsCorrupted defaults to false
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          contextRepositoryProvider.overrideWithValue(repo),
          capabilitiesProvider.overrideWith(
            (ref) async => ServerCapabilities.disabled,
          ),
          apiClientProvider.overrideWithValue(null),
          activeContextProvider.overrideWith(
            () => _CorruptedActiveContextNotifier(ctx),
          ),
        ],
        child: Consumer(
          builder: (context, ref, child) {
            final router = ref.watch(routerProvider);
            return MaterialApp.router(routerConfig: router);
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Healthy context → user lands on /chat (or /spaces) — definitely
    // NOT on /login.
    expect(find.byType(LoginScreen), findsNothing);
  });
}
