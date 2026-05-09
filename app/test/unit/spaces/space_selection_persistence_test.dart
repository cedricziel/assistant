// Verifies SpaceSelectionNotifier round-trips through an injectable
// key-value store on web. Native platforms keep the in-memory behavior.
//
// We don't run real localStorage in the test (VM has none); instead we
// inject a fake KeyValueStorage via the spaceSelectionStorageProvider
// override and assert hydration + persistence.
//
// Spec: openspec/changes/web-session-resilience/specs/web-session-resilience/spec.md
//   "Space selection persists across hard reload on web"

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/spaces/space_provider.dart';
import 'package:assistant_app/features/spaces/space_selection_storage.dart';

class _FakeKeyValueStorage implements SpaceSelectionStorage {
  String? _value;
  int reads = 0;
  int writes = 0;
  int deletes = 0;

  @override
  String? read(String key) {
    reads++;
    return _value;
  }

  @override
  void write(String key, String value) {
    writes++;
    _value = value;
  }

  @override
  void remove(String key) {
    deletes++;
    _value = null;
  }
}

void main() {
  test('selectOrg + selectSpace persists JSON to storage', () async {
    final storage = _FakeKeyValueStorage();
    final container = ProviderContainer(
      overrides: [spaceSelectionStorageProvider.overrideWithValue(storage)],
    );
    addTearDown(container.dispose);

    container.read(spaceSelectionProvider.notifier)
      ..selectOrg(orgId: 'o1', orgName: 'Org 1')
      ..selectSpace(spaceId: 's1', spaceName: 'Space 1');

    expect(storage.writes, greaterThanOrEqualTo(1));
    expect(storage._value, isNotNull);
    expect(storage._value, contains('"orgId":"o1"'));
    expect(storage._value, contains('"spaceId":"s1"'));
  });

  test('rebuild rehydrates state from storage', () async {
    final storage = _FakeKeyValueStorage();
    storage._value =
        '{"orgId":"o1","orgName":"Org 1","spaceId":"s1","spaceName":"Space 1"}';

    final container = ProviderContainer(
      overrides: [spaceSelectionStorageProvider.overrideWithValue(storage)],
    );
    addTearDown(container.dispose);

    final selection = container.read(spaceSelectionProvider);
    expect(selection.orgId, 'o1');
    expect(selection.orgName, 'Org 1');
    expect(selection.spaceId, 's1');
    expect(selection.spaceName, 'Space 1');
  });

  test('clear() removes the persisted entry', () async {
    final storage = _FakeKeyValueStorage();
    storage._value =
        '{"orgId":"o1","orgName":"Org 1","spaceId":"s1","spaceName":"Space 1"}';

    final container = ProviderContainer(
      overrides: [spaceSelectionStorageProvider.overrideWithValue(storage)],
    );
    addTearDown(container.dispose);

    container.read(spaceSelectionProvider.notifier).clear();

    expect(storage.deletes, greaterThanOrEqualTo(1));
    expect(storage._value, isNull);
  });

  test(
    'storage write failure is non-fatal — selection still updates',
    () async {
      final container = ProviderContainer(
        overrides: [
          spaceSelectionStorageProvider.overrideWithValue(_ThrowingStorage()),
        ],
      );
      addTearDown(container.dispose);

      // Must not throw even though the storage write throws.
      container.read(spaceSelectionProvider.notifier)
        ..selectOrg(orgId: 'o1', orgName: 'Org 1')
        ..selectSpace(spaceId: 's1', spaceName: 'Space 1');

      final selection = container.read(spaceSelectionProvider);
      expect(selection.orgId, 'o1');
      expect(selection.spaceId, 's1');
    },
  );

  test('corrupt JSON in storage is treated as empty selection', () async {
    final storage = _FakeKeyValueStorage();
    storage._value = 'not json';

    final container = ProviderContainer(
      overrides: [spaceSelectionStorageProvider.overrideWithValue(storage)],
    );
    addTearDown(container.dispose);

    final selection = container.read(spaceSelectionProvider);
    expect(selection.orgId, isNull);
    expect(selection.spaceId, isNull);
  });
}

class _ThrowingStorage implements SpaceSelectionStorage {
  @override
  String? read(String key) => null;

  @override
  void write(String key, String value) {
    throw StateError('quota exceeded');
  }

  @override
  void remove(String key) {
    throw StateError('quota exceeded');
  }
}
