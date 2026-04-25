import 'package:assistant_api/assistant_api.dart';
import 'package:built_collection/built_collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/api_keys/api_keys_provider.dart';
import 'package:assistant_app/features/api_keys/api_keys_screen.dart';

ApiKeySummary _makeKey({
  required String id,
  required String name,
  String keyPrefix = 'ask_abc',
}) {
  return ApiKeySummary(
    (b) => b
      ..id = id
      ..name = name
      ..keyPrefix = keyPrefix
      ..createdAt = '2025-01-15'
      ..scopes = ListBuilder<String>(),
  );
}

void main() {
  group('ApiKeysScreen', () {
    testWidgets('shows empty state when no keys', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            apiKeysProvider.overrideWith(() => _StubApiKeysNotifier([])),
          ],
          child: const MaterialApp(home: ApiKeysScreen()),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('No API keys'), findsOneWidget);
      expect(
        find.text('Create an API key to authenticate programmatic access.'),
        findsOneWidget,
      );
    });

    testWidgets('shows API key list', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            apiKeysProvider.overrideWith(
              () => _StubApiKeysNotifier([
                _makeKey(id: '1', name: 'CI Pipeline', keyPrefix: 'ask_ci1'),
                _makeKey(id: '2', name: 'Dev Token', keyPrefix: 'ask_dev'),
              ]),
            ),
          ],
          child: const MaterialApp(home: ApiKeysScreen()),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('CI Pipeline'), findsOneWidget);
      expect(find.text('Dev Token'), findsOneWidget);
    });

    testWidgets('shows create button (FAB)', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            apiKeysProvider.overrideWith(() => _StubApiKeysNotifier([])),
          ],
          child: const MaterialApp(home: ApiKeysScreen()),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(FloatingActionButton), findsOneWidget);
    });

    testWidgets('FAB opens create dialog', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            apiKeysProvider.overrideWith(() => _StubApiKeysNotifier([])),
          ],
          child: const MaterialApp(home: ApiKeysScreen()),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byType(FloatingActionButton));
      await tester.pumpAndSettle();

      expect(find.text('Create API key'), findsOneWidget);
      expect(find.text('Key name'), findsOneWidget);
    });

    testWidgets('revoke button shows confirmation dialog', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            apiKeysProvider.overrideWith(
              () => _StubApiKeysNotifier([_makeKey(id: '1', name: 'Test Key')]),
            ),
          ],
          child: const MaterialApp(home: ApiKeysScreen()),
        ),
      );
      await tester.pumpAndSettle();

      // Tap the delete icon button.
      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();

      expect(find.text('Revoke API key?'), findsOneWidget);
      expect(find.text('Cancel'), findsOneWidget);
      expect(find.text('Revoke'), findsOneWidget);
    });

    testWidgets('app bar shows title', (tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            apiKeysProvider.overrideWith(() => _StubApiKeysNotifier([])),
          ],
          child: const MaterialApp(home: ApiKeysScreen()),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('API Keys'), findsOneWidget);
    });
  });
}

class _StubApiKeysNotifier extends AsyncNotifier<List<ApiKeySummary>>
    implements ApiKeysNotifier {
  _StubApiKeysNotifier(this._keys);
  final List<ApiKeySummary> _keys;

  @override
  Future<List<ApiKeySummary>> build() async => _keys;

  @override
  Future<CreateApiKeyResponse?> createKey({required String name}) async {
    return null;
  }

  @override
  Future<void> revokeKey(String id) async {}
}
