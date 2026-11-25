import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/account/account_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/integrations/caller_roles.dart';
import 'package:assistant_app/features/spaces/space_provider.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'integration_fakes.dart';

class _Selection extends SpaceSelectionNotifier {
  @override
  SpaceSelection build() =>
      const SpaceSelection(orgId: 'org-1', spaceId: 'eng');
}

class _Spaces extends SpacesNotifier {
  _Spaces(this._spaces);
  final List<SpaceSummary> _spaces;

  @override
  Future<List<SpaceSummary>> build() async => _spaces;
}

ProviderContainer _container(Map<String, Object> members) {
  final container = ProviderContainer(
    overrides: [
      apiClientProvider.overrideWithValue(
        FakeApiClient(membersApi: FakeMembersApi(members)),
      ),
      spaceSelectionProvider.overrideWith(_Selection.new),
      spacesProvider.overrideWith(
        () => _Spaces([spaceSummary('eng'), spaceSummary('ops')]),
      ),
      currentUserIdProvider.overrideWith((ref) => 'me'),
    ],
  );
  addTearDown(container.dispose);
  return container;
}

void main() {
  group('callerRolesProvider', () {
    test('an org admin manages every space', () async {
      final container = _container({
        'eng': [memberEntry('eng', 'me', 'org_admin')],
        'ops': [memberEntry('ops', 'someone', 'member')],
      });

      final roles = await container.read(callerRolesProvider.future);

      expect(roles.isOrgAdmin, isTrue);
      expect(roles.manageableSpaces.map((s) => s.id), ['eng', 'ops']);
    });

    test('a space admin manages only their space', () async {
      final container = _container({
        'eng': [memberEntry('eng', 'me', 'space_admin')],
        'ops': [memberEntry('ops', 'me', 'member')],
      });

      final roles = await container.read(callerRolesProvider.future);

      expect(roles.isOrgAdmin, isFalse);
      expect(roles.canManageSpace('eng'), isTrue);
      expect(roles.canManageSpace('ops'), isFalse);
      expect(roles.manageableSpaces.map((s) => s.id), ['eng']);
    });

    test('a plain member manages nothing', () async {
      final container = _container({
        'eng': [memberEntry('eng', 'me', 'member')],
        'ops': [memberEntry('ops', 'me', 'viewer')],
      });

      final roles = await container.read(callerRolesProvider.future);

      expect(roles.isOrgAdmin, isFalse);
      expect(roles.manageableSpaces, isEmpty);
    });

    test('a space whose members cannot be read counts as no role', () async {
      final container = _container({
        'eng': dioError(403),
        'ops': [memberEntry('ops', 'me', 'space_admin')],
      });

      final roles = await container.read(callerRolesProvider.future);

      expect(roles.canManageSpace('eng'), isFalse);
      expect(roles.canManageSpace('ops'), isTrue);
    });
  });
}
