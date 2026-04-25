import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:assistant_app/features/spaces/space_provider.dart';
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// ---------------------------------------------------------------------------
// Fakes

class _FakePersonasApi extends PersonasApi {
  _FakePersonasApi(this._personas) : super(Dio(), standardSerializers);

  final List<PersonaSummary> _personas;
  int listCallCount = 0;

  @override
  Future<Response<BuiltList<PersonaSummary>>> listPersonas({
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    listCallCount++;
    return Response(
      requestOptions: RequestOptions(path: '/api/personas'),
      statusCode: 200,
      data: BuiltList<PersonaSummary>.from(_personas),
    );
  }
}

class _FakeApiClient extends ApiClient {
  _FakeApiClient(this._personasApi)
    : super(baseUrl: 'http://fake.test', token: 'test-token');

  final _FakePersonasApi _personasApi;

  @override
  PersonasApi get personas => _personasApi;
}

PersonaSummary _makePersona({required String id, required String name}) {
  return PersonaSummary(
    (b) => b
      ..id = id
      ..name = name
      ..description = '',
  );
}

// ---------------------------------------------------------------------------
// Tests

void main() {
  group('PersonasNotifier', () {
    test('loads personas on build', () async {
      final personas = [_makePersona(id: '1', name: 'Alpha')];
      final api = _FakeApiClient(_FakePersonasApi(personas));
      final container = ProviderContainer(
        overrides: [apiClientProvider.overrideWithValue(api)],
      );
      addTearDown(container.dispose);

      final sub = container.listen(personasProvider, (_, _) {});
      addTearDown(sub.close);

      final state = await container.read(personasProvider.future);
      expect(state.personas, hasLength(1));
      expect(state.personas.first.name, 'Alpha');
    });

    test('returns empty state when apiClient is null', () async {
      final container = ProviderContainer(
        overrides: [apiClientProvider.overrideWithValue(null)],
      );
      addTearDown(container.dispose);

      final sub = container.listen(personasProvider, (_, _) {});
      addTearDown(sub.close);

      final state = await container.read(personasProvider.future);
      expect(state.personas, isEmpty);
    });

    test('rebuilds when space selection changes', () async {
      final personasApi = _FakePersonasApi([
        _makePersona(id: '1', name: 'Alpha'),
      ]);
      final api = _FakeApiClient(personasApi);
      final container = ProviderContainer(
        overrides: [apiClientProvider.overrideWithValue(api)],
      );
      addTearDown(container.dispose);

      final sub = container.listen(personasProvider, (_, _) {});
      addTearDown(sub.close);

      await container.read(personasProvider.future);
      expect(personasApi.listCallCount, 1);

      // Change space selection — should trigger a rebuild.
      container
          .read(spaceSelectionProvider.notifier)
          .selectOrg(orgId: 'org-1', orgName: 'Org 1');
      await container.read(personasProvider.future);

      expect(
        personasApi.listCallCount,
        greaterThan(1),
        reason: 'space change should trigger persona re-fetch',
      );
    });

    test('selects default persona automatically', () async {
      final personas = [
        _makePersona(id: 'default', name: 'Default'),
        _makePersona(id: 'other', name: 'Other'),
      ];
      final api = _FakeApiClient(_FakePersonasApi(personas));
      final container = ProviderContainer(
        overrides: [apiClientProvider.overrideWithValue(api)],
      );
      addTearDown(container.dispose);

      final sub = container.listen(personasProvider, (_, _) {});
      addTearDown(sub.close);

      final state = await container.read(personasProvider.future);
      expect(state.activePersona, isNotNull);
      expect(state.activePersona!.id, 'default');
    });
  });
}
