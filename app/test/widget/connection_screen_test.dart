import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/connection/connection_screen.dart';
import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';
import 'package:assistant_app/features/embedded_server/embedded_server_provider.dart';

/// Minimal in-memory secure storage for widget tests.
class _FakeSecureStorage implements SecureKeyValueStorage {
  @override
  Future<String?> read({required String key}) async => null;
  @override
  Future<void> write({required String key, required String value}) async {}
  @override
  Future<void> delete({required String key}) async {}
}

Widget buildUnderTest() {
  SharedPreferences.setMockInitialValues({});
  return FutureBuilder<ContextRepository>(
    future: SharedPreferences.getInstance().then(
      (prefs) =>
          ContextRepository(prefs: prefs, secureStorage: _FakeSecureStorage()),
    ),
    builder: (context, snap) {
      if (!snap.hasData) return const MaterialApp(home: Scaffold());
      return ProviderScope(
        overrides: [contextRepositoryProvider.overrideWithValue(snap.data!)],
        child: const MaterialApp(home: ConnectionScreen()),
      );
    },
  );
}

void main() {
  group('ConnectionScreen', () {
    testWidgets('shows URL and token fields', (tester) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('server_url_field')), findsOneWidget);
      expect(find.byKey(const Key('token_field')), findsOneWidget);
      expect(find.byKey(const Key('connect_button')), findsOneWidget);
    });

    testWidgets('embedded mode toggle is NOT shown in test environment', (
      tester,
    ) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('server_mode_toggle')),
        findsNothing,
        reason: 'Toggle only appears when the bundled binary is present',
      );
      expect(find.byKey(const Key('server_url_field')), findsOneWidget);
    });

    testWidgets('embedded provider override wires correctly', (tester) async {
      SharedPreferences.setMockInitialValues({});
      final prefs = await SharedPreferences.getInstance();
      final repo = ContextRepository(
        prefs: prefs,
        secureStorage: _FakeSecureStorage(),
      );
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            contextRepositoryProvider.overrideWithValue(repo),
            embeddedServerProvider.overrideWith(
              () => _FakeEmbeddedNotifier(const EmbeddedServerStarting()),
            ),
          ],
          child: const MaterialApp(home: ConnectionScreen()),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('server_mode_toggle')), findsNothing);
      expect(find.byType(ConnectionScreen), findsOneWidget);
    });
  });
}

class _FakeEmbeddedNotifier extends EmbeddedServerNotifier {
  _FakeEmbeddedNotifier(this._initial);
  final EmbeddedServerState _initial;

  @override
  Future<EmbeddedServerState> build() async => _initial;
}
