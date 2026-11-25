import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/integrations/integration_policy_view_model.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'integration_fakes.dart';

ProviderContainer _container(FakeIntegrationsApi api) {
  final container = ProviderContainer(
    overrides: [
      apiClientProvider.overrideWithValue(FakeApiClient(integrationsApi: api)),
    ],
  );
  addTearDown(container.dispose);
  container.listen(integrationPolicyViewModelProvider, (_, _) {});
  return container;
}

IntegrationPolicyViewModel _notifier(ProviderContainer c) =>
    c.read(integrationPolicyViewModelProvider.notifier);

IntegrationPolicyState _state(ProviderContainer c) =>
    c.read(integrationPolicyViewModelProvider).requireValue;

void main() {
  test('loads the current policy', () async {
    final api = FakeIntegrationsApi()
      ..onGetPolicy = (() async => policy(customUrls: true));
    final container = _container(api);

    final state = await container.read(
      integrationPolicyViewModelProvider.future,
    );

    expect(state.policy.allowCustomUrls, isTrue);
    expect(state.saving, isFalse);
  });

  test('is loading before the policy arrives', () {
    final api = FakeIntegrationsApi()
      ..onGetPolicy = (() => Future.delayed(const Duration(days: 1), policy));
    final container = _container(api);

    expect(
      container.read(integrationPolicyViewModelProvider).isLoading,
      isTrue,
    );
  });

  test('surfaces a failed load as an error', () async {
    final api = FakeIntegrationsApi()
      ..onGetPolicy = (() async => throw dioError(500));
    final container = _container(api);

    await expectLater(
      container.read(integrationPolicyViewModelProvider.future),
      throwsA(anything),
    );
    expect(container.read(integrationPolicyViewModelProvider).hasError, isTrue);
  });

  test('save sends the whole policy and keeps the server answer', () async {
    UpdateIntegrationPolicyRequest? sent;
    final api = FakeIntegrationsApi()
      ..onGetPolicy = (() async => policy())
      ..onPutPolicy = ((request) async {
        sent = request;
        return policy(
          mode: CatalogMode.allowlist,
          allowed: ['curated/linear-app-mcp'],
          customUrls: true,
          userIntegrations: false,
        );
      });
    final container = _container(api);
    await container.read(integrationPolicyViewModelProvider.future);

    final saved = await _notifier(container).save(
      catalogMode: CatalogMode.allowlist,
      allowedCatalogIds: ['curated/linear-app-mcp'],
      allowCustomUrls: true,
      allowUserIntegrations: false,
    );

    expect(saved, isTrue);
    expect(sent!.catalogMode, CatalogMode.allowlist);
    expect(sent!.allowedCatalogIds!.toList(), ['curated/linear-app-mcp']);
    expect(sent!.allowCustomUrls, isTrue);
    expect(sent!.allowUserIntegrations, isFalse);
    final state = _state(container);
    expect(state.policy.catalogMode, CatalogMode.allowlist);
    expect(state.saving, isFalse);
    expect(state.message, isNull);
  });

  test('a 403 keeps the old policy and says why', () async {
    final api = FakeIntegrationsApi()
      ..onGetPolicy = (() async => policy())
      ..onPutPolicy = ((_) async => throw dioError(403));
    final container = _container(api);
    await container.read(integrationPolicyViewModelProvider.future);

    final saved = await _notifier(container).save(
      catalogMode: CatalogMode.none,
      allowedCatalogIds: const [],
      allowCustomUrls: false,
      allowUserIntegrations: true,
    );

    expect(saved, isFalse);
    final state = _state(container);
    expect(state.policy.catalogMode, CatalogMode.all);
    expect(state.message, contains('does not allow'));
    expect(state.saving, isFalse);
  });

  test('a 422 shows the server explanation', () async {
    final api = FakeIntegrationsApi()
      ..onGetPolicy = (() async => policy())
      ..onPutPolicy = ((request) async =>
          throw dioError(422, body: {'error': 'unknown catalog id'}));
    final container = _container(api);
    await container.read(integrationPolicyViewModelProvider.future);

    await _notifier(container).save(
      catalogMode: CatalogMode.allowlist,
      allowedCatalogIds: ['nope'],
      allowCustomUrls: false,
      allowUserIntegrations: true,
    );

    expect(_state(container).message, contains('unknown catalog id'));
  });

  test('retry reloads after an error', () async {
    var attempts = 0;
    final api = FakeIntegrationsApi()
      ..onGetPolicy = (() async {
        if (attempts++ == 0) throw dioError(500);
        return policy(customUrls: true);
      });
    final container = _container(api);
    await expectLater(
      container.read(integrationPolicyViewModelProvider.future),
      throwsA(anything),
    );

    container.invalidate(integrationPolicyViewModelProvider);
    final state = await container.read(
      integrationPolicyViewModelProvider.future,
    );

    expect(state.policy.allowCustomUrls, isTrue);
  });
}
