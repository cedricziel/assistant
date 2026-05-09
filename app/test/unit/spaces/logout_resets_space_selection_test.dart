// Tests that the web logout helper clears spaceSelectionProvider before
// deactivating the active context.
//
// Spec: openspec/specs/space-selector-resilience/spec.md
//   "Logout resets in-memory space selection"

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';
import 'package:assistant_app/features/spaces/space_provider.dart';
import 'package:assistant_app/shared/auth_actions.dart';

class _FakeSecureStorage implements SecureKeyValueStorage {
  final Map<String, String> _data = {};
  @override
  Future<String?> read({required String key}) async => _data[key];
  @override
  Future<void> write({required String key, required String value}) async {
    _data[key] = value;
  }

  @override
  Future<void> delete({required String key}) async => _data.remove(key);
}

Future<ContextRepository> _makeRepo() async {
  SharedPreferences.setMockInitialValues({});
  final p = await SharedPreferences.getInstance();
  return ContextRepository(prefs: p, secureStorage: _FakeSecureStorage());
}

void main() {
  test(
    'performWebLogout clears space selection AND deactivates context',
    () async {
      final repo = await _makeRepo();
      await repo.setActiveContextId('ctx-1');

      final container = ProviderContainer(
        overrides: [contextRepositoryProvider.overrideWithValue(repo)],
      );
      addTearDown(container.dispose);

      // Pre-seed a non-empty selection.
      container.read(spaceSelectionProvider.notifier)
        ..selectOrg(orgId: 'org-1', orgName: 'Org')
        ..selectSpace(spaceId: 'sp-1', spaceName: 'Space');

      expect(container.read(spaceSelectionProvider).orgId, 'org-1');
      expect(container.read(spaceSelectionProvider).spaceId, 'sp-1');
      expect(repo.getActiveContextId(), 'ctx-1');

      await performWebLogout(container);

      expect(container.read(spaceSelectionProvider).orgId, isNull);
      expect(container.read(spaceSelectionProvider).spaceId, isNull);
      expect(repo.getActiveContextId(), isNull);
    },
  );

  test('performWebLogout is idempotent on already-empty state', () async {
    final repo = await _makeRepo();

    final container = ProviderContainer(
      overrides: [contextRepositoryProvider.overrideWithValue(repo)],
    );
    addTearDown(container.dispose);

    await performWebLogout(container);
    await performWebLogout(container);

    expect(container.read(spaceSelectionProvider).orgId, isNull);
    expect(repo.getActiveContextId(), isNull);
  });
}
