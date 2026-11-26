import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';

/// In-memory `PersonasApi` for one Persona (`p1`) that stores starter prompts.
class FakePersonasApi extends PersonasApi {
  FakePersonasApi({List<String> starters = const [], this.canEdit = true})
    : _starters = List.of(starters),
      super(Dio(), standardSerializers);

  List<String> _starters;
  final bool canEdit;
  Object? updateError;
  int getCalls = 0;
  final List<List<String>> updates = [];

  PersonaDetail _detail() => PersonaDetail(
    (b) => b
      ..id = 'p1'
      ..name = 'Alice'
      ..createdAt = '2026-01-01T00:00:00Z'
      ..updatedAt = '2026-01-01T00:00:00Z'
      ..skillAccessMode = 'all'
      ..canEditStarterPrompts = canEdit
      ..starterPrompts = ListBuilder<String>(_starters),
  );

  Response<T> _ok<T>(T data) => Response(
    requestOptions: RequestOptions(path: '/api/personas/p1'),
    statusCode: 200,
    data: data,
  );

  @override
  Future<Response<PersonaDetail>> getPersona({
    required String id,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    getCalls++;
    return _ok(_detail());
  }

  @override
  Future<Response<PersonaDetail>> updatePersona({
    required String id,
    required UpdatePersonaRequest updatePersonaRequest,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    final error = updateError;
    if (error != null) throw error;
    _starters = updatePersonaRequest.starterPrompts!.toList();
    updates.add(_starters);
    return _ok(_detail());
  }

  @override
  Future<Response<BuiltList<PersonaSummary>>> listPersonas({
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async => _ok(
    BuiltList<PersonaSummary>([
      PersonaSummary(
        (b) => b
          ..id = 'p1'
          ..name = 'Alice'
          ..description = '',
      ),
    ]),
  );
}

class FakePersonasApiClient extends ApiClient {
  FakePersonasApiClient(this._personasApi)
    : super(baseUrl: 'http://fake.test', token: 'test-token');

  final FakePersonasApi _personasApi;

  @override
  PersonasApi get personas => _personasApi;
}
