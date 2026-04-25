import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/features/api_keys/api_keys_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// ---------------------------------------------------------------------------
// Fakes

class _FakeApiKeysApi extends ApiKeysApi {
  _FakeApiKeysApi(this._keys) : super(Dio(), standardSerializers);

  final List<ApiKeySummary> _keys;
  int listCallCount = 0;
  int createCallCount = 0;
  final List<String> deletedIds = [];

  @override
  Future<Response<BuiltList<ApiKeySummary>>> listApiKeys({
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    listCallCount++;
    return Response(
      requestOptions: RequestOptions(path: '/api/users/me/api-keys'),
      statusCode: 200,
      data: BuiltList<ApiKeySummary>.from(_keys),
    );
  }

  @override
  Future<Response<CreateApiKeyResponse>> createApiKey({
    required CreateApiKeyRequest createApiKeyRequest,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    createCallCount++;
    final response = CreateApiKeyResponse(
      (b) => b
        ..id = 'new-key-id'
        ..name = createApiKeyRequest.name
        ..keyPrefix = 'ask_abc'
        ..plaintext = 'ask_abcdefghijk1234567890'
        ..scopes = ListBuilder<String>(),
    );
    return Response(
      requestOptions: RequestOptions(path: '/api/users/me/api-keys'),
      statusCode: 201,
      data: response,
    );
  }

  @override
  Future<Response<void>> deleteApiKey({
    required String id,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    deletedIds.add(id);
    return Response(
      requestOptions: RequestOptions(path: '/api/users/me/api-keys/$id'),
      statusCode: 204,
    );
  }
}

class _FakeApiClient extends ApiClient {
  _FakeApiClient(this._apiKeysApi)
    : super(baseUrl: 'http://fake.test', token: 'test-token');

  final _FakeApiKeysApi _apiKeysApi;

  @override
  ApiKeysApi get apiKeys => _apiKeysApi;
}

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
      ..createdAt = '2025-01-15T10:00:00Z'
      ..scopes = ListBuilder<String>(),
  );
}

// ---------------------------------------------------------------------------
// Tests

void main() {
  group('ApiKeysNotifier', () {
    test('loads API keys on build', () async {
      final keys = [_makeKey(id: '1', name: 'CI Key')];
      final api = _FakeApiClient(_FakeApiKeysApi(keys));
      final container = ProviderContainer(
        overrides: [apiClientProvider.overrideWithValue(api)],
      );
      addTearDown(container.dispose);

      final result = await container.read(apiKeysProvider.future);
      expect(result, hasLength(1));
      expect(result.first.name, 'CI Key');
    });

    test('returns empty list when apiClient is null', () async {
      final container = ProviderContainer(
        overrides: [apiClientProvider.overrideWithValue(null)],
      );
      addTearDown(container.dispose);

      final result = await container.read(apiKeysProvider.future);
      expect(result, isEmpty);
    });

    test('createKey calls API and returns response with plaintext', () async {
      final apiKeysApi = _FakeApiKeysApi([]);
      final api = _FakeApiClient(apiKeysApi);
      final container = ProviderContainer(
        overrides: [apiClientProvider.overrideWithValue(api)],
      );
      addTearDown(container.dispose);

      await container.read(apiKeysProvider.future);
      final response = await container
          .read(apiKeysProvider.notifier)
          .createKey(name: 'Test Key');

      expect(response, isNotNull);
      expect(response!.plaintext, 'ask_abcdefghijk1234567890');
      expect(response.name, 'Test Key');
      expect(apiKeysApi.createCallCount, 1);
    });

    test('revokeKey calls API delete', () async {
      final keys = [_makeKey(id: 'key-1', name: 'Old Key')];
      final apiKeysApi = _FakeApiKeysApi(keys);
      final api = _FakeApiClient(apiKeysApi);
      final container = ProviderContainer(
        overrides: [apiClientProvider.overrideWithValue(api)],
      );
      addTearDown(container.dispose);

      await container.read(apiKeysProvider.future);
      await container.read(apiKeysProvider.notifier).revokeKey('key-1');

      expect(apiKeysApi.deletedIds, ['key-1']);
    });
  });
}
