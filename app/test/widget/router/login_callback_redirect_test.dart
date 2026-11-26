// The server's sign-in page redirects to /login/callback?code=…&state=…
// on a fresh page load, before saved contexts have loaded and with no
// usable context. The router must keep that URL so the code is exchanged,
// instead of bouncing to /login.

import 'dart:async';

import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/api/capabilities_provider.dart';
import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';
import 'package:assistant_app/features/login/login_callback_screen.dart';
import 'package:assistant_app/router/app_router.dart';

class _FakeSecureStorage implements SecureKeyValueStorage {
  @override
  Future<String?> read({required String key}) async => null;
  @override
  Future<void> write({required String key, required String value}) async {}
  @override
  Future<void> delete({required String key}) async {}
}

class _SlowActiveContextNotifier extends ActiveContextNotifier {
  _SlowActiveContextNotifier(this._loaded);
  final Completer<AssistantContext?> _loaded;
  @override
  Future<AssistantContext?> build() => _loaded.future;
}

void main() {
  testWidgets('/login/callback survives context loading without a context', (
    tester,
  ) async {
    SharedPreferences.setMockInitialValues({});
    final prefs = await SharedPreferences.getInstance();
    final repo = ContextRepository(
      prefs: prefs,
      secureStorage: _FakeSecureStorage(),
    );
    final loaded = Completer<AssistantContext?>();

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          contextRepositoryProvider.overrideWithValue(repo),
          capabilitiesProvider.overrideWith(
            (ref) async => ServerCapabilities.disabled,
          ),
          apiClientProvider.overrideWithValue(null),
          activeContextProvider.overrideWith(
            () => _SlowActiveContextNotifier(loaded),
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
    await tester.pump();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(Consumer)),
    );
    container.read(routerProvider).go('/login/callback?code=abc&state=xyz');
    await tester.pump();

    loaded.complete(null);
    await tester.pumpAndSettle();

    expect(find.byType(LoginCallbackScreen), findsOneWidget);
    final location = container
        .read(routerProvider)
        .routerDelegate
        .currentConfiguration
        .uri
        .toString();
    expect(location, contains('code=abc'), reason: 'got: $location');
  });
}
