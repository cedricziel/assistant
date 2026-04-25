import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/spaces/space_provider.dart';

void main() {
  group('SpaceSelection', () {
    test('default is empty', () {
      const s = SpaceSelection();
      expect(s.orgId, isNull);
      expect(s.orgName, isNull);
      expect(s.spaceId, isNull);
      expect(s.spaceName, isNull);
      expect(s.isComplete, isFalse);
    });

    test('isComplete requires both orgId and spaceId', () {
      const orgOnly = SpaceSelection(orgId: 'o1', orgName: 'Org');
      expect(orgOnly.isComplete, isFalse);

      const full = SpaceSelection(
        orgId: 'o1',
        orgName: 'Org',
        spaceId: 's1',
        spaceName: 'Space',
      );
      expect(full.isComplete, isTrue);
    });

    test('copyWith updates fields', () {
      const s = SpaceSelection(orgId: 'o1', orgName: 'Org');
      final updated = s.copyWith(spaceId: 's1', spaceName: 'Space');
      expect(updated.orgId, 'o1');
      expect(updated.spaceId, 's1');
    });

    test('copyWith with clearSpace removes space', () {
      const s = SpaceSelection(
        orgId: 'o1',
        orgName: 'Org',
        spaceId: 's1',
        spaceName: 'Space',
      );
      final cleared = s.copyWith(clearSpace: true);
      expect(cleared.orgId, 'o1');
      expect(cleared.spaceId, isNull);
      expect(cleared.spaceName, isNull);
    });
  });

  group('SpaceSelectionNotifier', () {
    test('initial state is empty', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      final state = container.read(spaceSelectionProvider);
      expect(state.isComplete, isFalse);
    });

    test('selectOrg sets org and clears space', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      // Select org + space first.
      container
          .read(spaceSelectionProvider.notifier)
          .selectOrg(orgId: 'o1', orgName: 'Org 1');
      container
          .read(spaceSelectionProvider.notifier)
          .selectSpace(spaceId: 's1', spaceName: 'Space 1');
      expect(container.read(spaceSelectionProvider).isComplete, isTrue);

      // Selecting a different org clears the space.
      container
          .read(spaceSelectionProvider.notifier)
          .selectOrg(orgId: 'o2', orgName: 'Org 2');
      final state = container.read(spaceSelectionProvider);
      expect(state.orgId, 'o2');
      expect(state.orgName, 'Org 2');
      expect(state.spaceId, isNull);
      expect(state.isComplete, isFalse);
    });

    test('selectSpace sets space', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      container
          .read(spaceSelectionProvider.notifier)
          .selectOrg(orgId: 'o1', orgName: 'Org 1');
      container
          .read(spaceSelectionProvider.notifier)
          .selectSpace(spaceId: 's1', spaceName: 'Space 1');

      final state = container.read(spaceSelectionProvider);
      expect(state.spaceId, 's1');
      expect(state.spaceName, 'Space 1');
      expect(state.isComplete, isTrue);
    });

    test('clear resets to empty', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      container
          .read(spaceSelectionProvider.notifier)
          .selectOrg(orgId: 'o1', orgName: 'Org 1');
      container
          .read(spaceSelectionProvider.notifier)
          .selectSpace(spaceId: 's1', spaceName: 'Space 1');
      container.read(spaceSelectionProvider.notifier).clear();

      final state = container.read(spaceSelectionProvider);
      expect(state.orgId, isNull);
      expect(state.spaceId, isNull);
      expect(state.isComplete, isFalse);
    });
  });

  group('hasSpaceSelectionProvider', () {
    test('false when no selection', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      expect(container.read(hasSpaceSelectionProvider), isFalse);
    });

    test('true when org + space selected', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      container
          .read(spaceSelectionProvider.notifier)
          .selectOrg(orgId: 'o1', orgName: 'Org 1');
      container
          .read(spaceSelectionProvider.notifier)
          .selectSpace(spaceId: 's1', spaceName: 'Space 1');

      expect(container.read(hasSpaceSelectionProvider), isTrue);
    });
  });
}
