import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/features/chat/models_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// -- Fake ModelsApi ----------------------------------------------------------

class _FakeModelsApi extends ModelsApi {
  _FakeModelsApi({this.models = const [], this.error, this.pending})
    : super(Dio(), standardSerializers);

  final List<ModelSummary> models;
  final Exception? error;
  final Completer<void>? pending;

  @override
  Future<Response<BuiltList<ModelSummary>>> listModels({
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    if (pending != null) await pending!.future;
    if (error != null) throw error!;
    return Response(
      data: BuiltList.of(models),
      requestOptions: RequestOptions(path: '/api/models'),
      statusCode: 200,
    );
  }
}

class _FakeApiClient extends ApiClient {
  _FakeApiClient({required this.fakeModels})
    : super(baseUrl: 'http://fake.test', token: 'test-token');

  final _FakeModelsApi fakeModels;

  @override
  ModelsApi get models => fakeModels;
}

// -- Helpers -----------------------------------------------------------------

ModelSummary _model(String name, {bool isDefault = false}) => ModelSummary(
  (b) => b
    ..name = name
    ..isDefault = isDefault,
);

ProviderContainer _container({ApiClient? client}) =>
    ProviderContainer(overrides: [apiClientProvider.overrideWithValue(client)]);

void main() {
  group('modelsProvider', () {
    test('is empty without a client', () async {
      final container = _container(client: null);
      addTearDown(container.dispose);

      expect(await container.read(modelsProvider.future), isEmpty);
    });

    test('is loading until the request settles', () async {
      final pending = Completer<void>();
      final container = _container(
        client: _FakeApiClient(
          fakeModels: _FakeModelsApi(
            models: [_model('model-a', isDefault: true)],
            pending: pending,
          ),
        ),
      );
      addTearDown(container.dispose);

      expect(container.read(modelsProvider).isLoading, isTrue);

      pending.complete();
      final models = await container.read(modelsProvider.future);
      expect(models, hasLength(1));
    });

    test('maps the default first, then the listed models', () async {
      final container = _container(
        client: _FakeApiClient(
          fakeModels: _FakeModelsApi(
            models: [_model('model-a', isDefault: true), _model('model-b')],
          ),
        ),
      );
      addTearDown(container.dispose);

      final models = await container.read(modelsProvider.future);

      expect(models.map((m) => m.name), ['model-a', 'model-b']);
      expect(models.first.isDefault, isTrue);
      expect(models.last.isDefault, isFalse);
    });

    test('an empty list stays empty', () async {
      final container = _container(
        client: _FakeApiClient(fakeModels: _FakeModelsApi()),
      );
      addTearDown(container.dispose);

      expect(await container.read(modelsProvider.future), isEmpty);
    });

    test('a single model is returned as-is', () async {
      final container = _container(
        client: _FakeApiClient(
          fakeModels: _FakeModelsApi(models: [_model('only', isDefault: true)]),
        ),
      );
      addTearDown(container.dispose);

      final models = await container.read(modelsProvider.future);

      expect(models, hasLength(1));
      expect(models.single.isDefault, isTrue);
    });

    test('surfaces a failure instead of crashing', () async {
      final container = _container(
        client: _FakeApiClient(
          fakeModels: _FakeModelsApi(
            error: DioException(
              requestOptions: RequestOptions(path: '/api/models'),
              type: DioExceptionType.connectionError,
            ),
          ),
        ),
      );
      addTearDown(container.dispose);

      final completer = Completer<AsyncValue<List<RegenerateModel>>>();
      final sub = container.listen(modelsProvider, (_, next) {
        if (next.hasError && !completer.isCompleted) {
          completer.complete(next);
        }
      });
      addTearDown(sub.close);

      final value = await completer.future;
      expect(value.error, isA<DioException>());
    });
  });
}
