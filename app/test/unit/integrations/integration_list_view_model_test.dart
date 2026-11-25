import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/integrations/caller_roles.dart';
import 'package:assistant_app/features/integrations/integration_list_view_model.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'integration_fakes.dart';

ProviderContainer _container(
  FakeIntegrationsApi api,
  IntegrationListScope scope,
) {
  final container = ProviderContainer(
    overrides: [
      apiClientProvider.overrideWithValue(FakeApiClient(integrationsApi: api)),
      currentUserIdProvider.overrideWith((ref) => 'me'),
    ],
  );
  addTearDown(container.dispose);
  container.listen(integrationListViewModelProvider(scope), (_, _) {});
  return container;
}

Future<IntegrationListState> _load(
  ProviderContainer container,
  IntegrationListScope scope,
) => container.read(integrationListViewModelProvider(scope).future);

IntegrationListViewModel _notifier(
  ProviderContainer container,
  IntegrationListScope scope,
) => container.read(integrationListViewModelProvider(scope).notifier);

IntegrationListState _state(
  ProviderContainer container,
  IntegrationListScope scope,
) => container.read(integrationListViewModelProvider(scope)).requireValue;

void main() {
  group('org scope', () {
    const scope = IntegrationListScope.org();

    test('lists the org-layer integrations', () async {
      final api = FakeIntegrationsApi()
        ..onList = ((layer, spaceId) async => [integrationSummary('int_a')]);
      final container = _container(api, scope);

      final state = await _load(container, scope);

      expect(state.rows.map((r) => r.integration.id), ['int_a']);
      expect(api.calls, ['list layer=org space=null']);
    });

    test('is empty when there are no integrations', () async {
      final api = FakeIntegrationsApi()..onList = (_, _) async => [];
      final container = _container(api, scope);

      expect((await _load(container, scope)).rows, isEmpty);
    });

    test('is loading before the first response', () {
      final api = FakeIntegrationsApi()
        ..onList = ((_, _) =>
            Future.delayed(const Duration(days: 1), () => []));
      final container = _container(api, scope);

      expect(
        container.read(integrationListViewModelProvider(scope)).isLoading,
        isTrue,
      );
    });

    test('surfaces a failed load as an error', () async {
      final api = FakeIntegrationsApi()
        ..onList = ((_, _) async => throw dioError(500));
      final container = _container(api, scope);

      await expectLater(
        container.read(integrationListViewModelProvider(scope).future),
        throwsA(anything),
      );
      expect(
        container.read(integrationListViewModelProvider(scope)).hasError,
        isTrue,
      );
    });

    test('disabling patches the integration and updates the row', () async {
      final api = FakeIntegrationsApi()
        ..onList = ((_, _) async => [integrationSummary('int_a')])
        ..onUpdate = ((id, r) async => integrationDetail(
          id,
          enabled: false,
          status: IntegrationStatus.disabled,
        ));
      final container = _container(api, scope);
      await _load(container, scope);

      await _notifier(container, scope).setEnabled('int_a', false);

      final row = _state(container, scope).rows.single.integration;
      expect(row.enabled, isFalse);
      expect(row.status, IntegrationStatus.disabled);
      expect(api.calls, contains('patch int_a enabled=false'));
      expect(_state(container, scope).busyIds, isEmpty);
    });

    test('a 409 refreshes the list and explains', () async {
      var listed = 0;
      final api = FakeIntegrationsApi()
        ..onList = ((_, _) async => [
          integrationSummary('int_a', enabled: ++listed > 1 ? false : true),
        ])
        ..onUpdate = ((id, r) async => throw dioError(409));
      final container = _container(api, scope);
      await _load(container, scope);

      await _notifier(container, scope).setEnabled('int_a', false);

      final state = _state(container, scope);
      expect(listed, 2, reason: 'list refreshed after the conflict');
      expect(state.rows.single.integration.enabled, isFalse);
      expect(state.message, contains('changed'));
    });

    test('a 403 on toggle keeps the row and reports the denial', () async {
      final api = FakeIntegrationsApi()
        ..onList = ((_, _) async => [integrationSummary('int_a')])
        ..onUpdate = ((id, r) async => throw dioError(403));
      final container = _container(api, scope);
      await _load(container, scope);

      await _notifier(container, scope).setEnabled('int_a', false);

      final state = _state(container, scope);
      expect(state.rows.single.integration.enabled, isTrue);
      expect(state.message, contains('does not allow'));
    });

    test('delete removes the row', () async {
      final api = FakeIntegrationsApi()
        ..onList = ((_, _) async => [
          integrationSummary('int_a'),
          integrationSummary('int_b'),
        ])
        ..onDelete = (id) async {};
      final container = _container(api, scope);
      await _load(container, scope);

      await _notifier(container, scope).delete('int_a');

      expect(_state(container, scope).rows.map((r) => r.integration.id), [
        'int_b',
      ]);
      expect(api.calls, contains('delete int_a'));
    });

    test('a failed delete keeps the row and reports it', () async {
      final api = FakeIntegrationsApi()
        ..onList = ((_, _) async => [integrationSummary('int_a')])
        ..onDelete = ((id) async => throw dioError(500));
      final container = _container(api, scope);
      await _load(container, scope);

      await _notifier(container, scope).delete('int_a');

      final state = _state(container, scope);
      expect(state.rows, hasLength(1));
      expect(state.message, contains('500'));
    });

    test('clearMessage drops the one-shot message', () async {
      final api = FakeIntegrationsApi()
        ..onList = ((_, _) async => [integrationSummary('int_a')])
        ..onDelete = ((id) async => throw dioError(500));
      final container = _container(api, scope);
      await _load(container, scope);
      await _notifier(container, scope).delete('int_a');

      _notifier(container, scope).clearMessage();

      expect(_state(container, scope).message, isNull);
    });
  });

  group('space scope', () {
    const scope = IntegrationListScope.space('eng');

    test(
      'lists own and offered integrations plus opted-out org ones',
      () async {
        final api = FakeIntegrationsApi()
          ..onList = (layer, spaceId) async {
            if (spaceId == 'eng') {
              return [
                integrationSummary(
                  'int_space',
                  layer: IntegrationLayer.space,
                  spaceId: 'eng',
                ),
                integrationSummary('int_org_on'),
              ];
            }
            // Every org integration the caller can see.
            return [
              integrationSummary('int_org_on'),
              integrationSummary('int_org_off'),
              integrationSummary('int_org_elsewhere'),
            ];
          }
          ..onGet = ((id) async => integrationDetail(
            id,
            spaceAccess: id == 'int_org_off' ? [optOutOf('eng')] : [],
          ));
        final container = _container(api, scope);

        final state = await _load(container, scope);

        final byId = {for (final r in state.rows) r.integration.id: r};
        expect(byId.keys, {'int_space', 'int_org_on', 'int_org_off'});
        expect(byId['int_org_on']!.optedOut, isFalse);
        expect(byId['int_org_off']!.optedOut, isTrue);
      },
    );

    test('opting out puts the opt_out row and marks the row', () async {
      final api = FakeIntegrationsApi()
        ..onList = ((layer, spaceId) async => [
          integrationSummary('int_org_on'),
        ])
        ..onPutAccess = (id, spaceId, mode) async {};
      final container = _container(api, scope);
      await _load(container, scope);

      await _notifier(container, scope).setOptedOut('int_org_on', true);

      expect(_state(container, scope).rows.single.optedOut, isTrue);
      expect(api.calls, contains('put-access int_org_on eng optOut'));
    });

    test('opting back in removes the opt_out row', () async {
      final api = FakeIntegrationsApi()
        ..onList = ((layer, spaceId) async =>
            spaceId == 'eng' ? [] : [integrationSummary('int_org_off')])
        ..onGet = ((id) async =>
            integrationDetail(id, spaceAccess: [optOutOf('eng')]))
        ..onDeleteAccess = (id, spaceId, mode) async {};
      final container = _container(api, scope);
      await _load(container, scope);

      await _notifier(container, scope).setOptedOut('int_org_off', false);

      expect(_state(container, scope).rows.single.optedOut, isFalse);
      expect(api.calls, contains('delete-access int_org_off eng optOut'));
    });

    test('a failed opt-out leaves the row as it was', () async {
      final api = FakeIntegrationsApi()
        ..onList = ((layer, spaceId) async => [
          integrationSummary('int_org_on'),
        ])
        ..onPutAccess = ((id, spaceId, mode) async => throw dioError(403));
      final container = _container(api, scope);
      await _load(container, scope);

      await _notifier(container, scope).setOptedOut('int_org_on', true);

      final state = _state(container, scope);
      expect(state.rows.single.optedOut, isFalse);
      expect(state.message, contains('does not allow'));
    });
  });

  group('connections scope', () {
    const scope = IntegrationListScope.connections();

    test('keeps own personal and per-user shared integrations only', () async {
      final api = FakeIntegrationsApi()
        ..onList = ((layer, spaceId) async => [
          integrationSummary(
            'int_mine',
            layer: IntegrationLayer.user,
            ownerUserId: 'me',
            mode: CredentialMode.perUser,
          ),
          integrationSummary(
            'int_theirs',
            layer: IntegrationLayer.user,
            ownerUserId: 'someone-else',
            mode: CredentialMode.perUser,
          ),
          integrationSummary('int_org_per_user', mode: CredentialMode.perUser),
          integrationSummary(
            'int_space_per_user',
            layer: IntegrationLayer.space,
            spaceId: 'eng',
            mode: CredentialMode.perUser,
          ),
          integrationSummary('int_org_shared'),
        ]);
      final container = _container(api, scope);

      final state = await _load(container, scope);

      expect(state.rows.map((r) => r.integration.id), [
        'int_mine',
        'int_org_per_user',
        'int_space_per_user',
      ]);
      expect(api.calls, ['list layer=null space=null']);
    });
  });
}
