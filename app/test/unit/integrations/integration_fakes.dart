import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';

/// A failed response with [status], as Dio reports it.
DioException dioError(int status, {Object? body}) {
  final options = RequestOptions(path: '/api/integrations');
  return DioException(
    requestOptions: options,
    response: Response(requestOptions: options, statusCode: status, data: body),
    type: DioExceptionType.badResponse,
  );
}

Response<T> ok<T>(T data, {int status = 200}) => Response(
  requestOptions: RequestOptions(path: '/fake'),
  statusCode: status,
  data: data,
);

final _stamp = DateTime.utc(2026, 1, 1);

IntegrationSummary integrationSummary(
  String id, {
  IntegrationLayer layer = IntegrationLayer.org,
  String? name,
  bool enabled = true,
  IntegrationStatus status = IntegrationStatus.ready,
  CredentialMode mode = CredentialMode.shared,
  String? ownerUserId,
  String? spaceId,
  bool hasCredential = true,
}) {
  return IntegrationSummary(
    (b) => b
      ..id = id
      ..layer = layer
      ..slug = id.replaceAll('_', '-')
      ..name = name ?? id
      ..connectUrl = 'https://$id.example.com/mcp'
      ..authKind = IntegrationAuthKind.header
      ..credentialMode = mode
      ..trust = IntegrationTrust.confirm
      ..enabled = enabled
      ..status = status
      ..hasCredential = hasCredential
      ..ownerUserId = ownerUserId
      ..spaceId = spaceId
      ..createdAt = _stamp
      ..updatedAt = _stamp,
  );
}

IntegrationDetail integrationDetail(
  String id, {
  IntegrationLayer layer = IntegrationLayer.org,
  String? name,
  bool enabled = true,
  IntegrationStatus status = IntegrationStatus.ready,
  CredentialMode mode = CredentialMode.shared,
  String? spaceId,
  List<SpaceAccessEntry> spaceAccess = const [],
}) {
  return IntegrationDetail(
    (b) => b
      ..id = id
      ..layer = layer
      ..slug = id.replaceAll('_', '-')
      ..name = name ?? id
      ..connectUrl = 'https://$id.example.com/mcp'
      ..authKind = IntegrationAuthKind.header
      ..credentialMode = mode
      ..trust = IntegrationTrust.confirm
      ..enabled = enabled
      ..status = status
      ..hasCredential = false
      ..spaceId = spaceId
      ..spaceAccess = ListBuilder<SpaceAccessEntry>(spaceAccess)
      ..createdAt = _stamp
      ..updatedAt = _stamp.add(const Duration(minutes: 1)),
  );
}

SpaceAccessEntry optOutOf(String spaceId) => SpaceAccessEntry(
  (b) => b
    ..spaceId = spaceId
    ..mode = SpaceAccessMode.optOut,
);

SpaceSummary spaceSummary(String id, {String? name}) => SpaceSummary(
  (b) => b
    ..id = id
    ..name = name ?? id
    ..slug = id,
);

MemberEntry memberEntry(String spaceId, String userId, String role) =>
    MemberEntry(
      (b) => b
        ..spaceId = spaceId
        ..userId = userId
        ..role = role
        ..createdAt = '2026-01-01',
    );

IntegrationPolicyResponse policy({
  CatalogMode mode = CatalogMode.all,
  List<String> allowed = const [],
  bool customUrls = false,
  bool userIntegrations = true,
}) {
  return IntegrationPolicyResponse(
    (b) => b
      ..catalogMode = mode
      ..allowedCatalogIds = ListBuilder<String>(allowed)
      ..allowCustomUrls = customUrls
      ..allowUserIntegrations = userIntegrations,
  );
}

/// A scriptable [IntegrationsApi]; every call is recorded in [calls].
class FakeIntegrationsApi extends IntegrationsApi {
  FakeIntegrationsApi() : super(Dio(), standardSerializers);

  final calls = <String>[];

  Future<List<IntegrationSummary>> Function(
    IntegrationLayer? layer,
    String? spaceId,
  )?
  onList;
  Future<IntegrationDetail> Function(String id)? onGet;
  Future<IntegrationDetail> Function(CreateIntegrationRequest request)?
  onCreate;
  Future<IntegrationDetail> Function(String id, UpdateIntegrationRequest r)?
  onUpdate;
  Future<void> Function(String id)? onDelete;
  Future<IntegrationPolicyResponse> Function()? onGetPolicy;
  Future<IntegrationPolicyResponse> Function(UpdateIntegrationPolicyRequest r)?
  onPutPolicy;
  Future<void> Function(String id, String spaceId, SpaceAccessMode mode)?
  onPutAccess;
  Future<void> Function(String id, String spaceId, SpaceAccessMode mode)?
  onDeleteAccess;

  @override
  Future<Response<IntegrationListResponse>> listIntegrations({
    IntegrationLayer? layer,
    String? spaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    calls.add('list layer=${layer?.name} space=$spaceId');
    final items = await onList!(layer, spaceId);
    return ok(
      IntegrationListResponse(
        (b) => b..items = ListBuilder<IntegrationSummary>(items),
      ),
    );
  }

  @override
  Future<Response<IntegrationDetail>> getIntegration({
    required String id,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    calls.add('get $id');
    return ok(await onGet!(id));
  }

  @override
  Future<Response<IntegrationDetail>> createIntegration({
    required CreateIntegrationRequest createIntegrationRequest,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    calls.add('create');
    return ok(await onCreate!(createIntegrationRequest), status: 201);
  }

  @override
  Future<Response<IntegrationDetail>> updateIntegration({
    required String id,
    required UpdateIntegrationRequest updateIntegrationRequest,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    calls.add('patch $id enabled=${updateIntegrationRequest.enabled}');
    return ok(await onUpdate!(id, updateIntegrationRequest));
  }

  @override
  Future<Response<void>> deleteIntegration({
    required String id,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    calls.add('delete $id');
    await onDelete!(id);
    return ok<void>(null, status: 204);
  }

  @override
  Future<Response<IntegrationPolicyResponse>> getIntegrationPolicy({
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    calls.add('get-policy');
    return ok(await onGetPolicy!());
  }

  @override
  Future<Response<IntegrationPolicyResponse>> putIntegrationPolicy({
    required UpdateIntegrationPolicyRequest updateIntegrationPolicyRequest,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    calls.add('put-policy');
    return ok(await onPutPolicy!(updateIntegrationPolicyRequest));
  }

  @override
  Future<Response<SpaceAccessEntry>> putSpaceAccess({
    required String id,
    required String spaceId,
    required SetSpaceAccessRequest setSpaceAccessRequest,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    calls.add('put-access $id $spaceId ${setSpaceAccessRequest.mode.name}');
    await onPutAccess!(id, spaceId, setSpaceAccessRequest.mode);
    return ok(
      SpaceAccessEntry(
        (b) => b
          ..spaceId = spaceId
          ..mode = setSpaceAccessRequest.mode,
      ),
    );
  }

  @override
  Future<Response<void>> deleteSpaceAccess({
    required String id,
    required String spaceId,
    required SpaceAccessMode mode,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    calls.add('delete-access $id $spaceId ${mode.name}');
    await onDeleteAccess!(id, spaceId, mode);
    return ok<void>(null, status: 204);
  }
}

/// A scriptable [MembersApi].
class FakeMembersApi extends MembersApi {
  FakeMembersApi(this.membersBySpace) : super(Dio(), standardSerializers);

  /// Per-space member list; a space mapped to an exception throws it.
  final Map<String, Object> membersBySpace;

  @override
  Future<Response<BuiltList<MemberEntry>>> listMembers({
    required String orgId,
    required String spaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    final members = membersBySpace[spaceId];
    if (members is Exception) throw members;
    return ok(BuiltList<MemberEntry>.from(members! as List<MemberEntry>));
  }
}

class FakeApiClient extends ApiClient {
  FakeApiClient({this.integrationsApi, this.membersApi})
    : super(baseUrl: 'http://fake.test', token: 'test-token');

  final IntegrationsApi? integrationsApi;
  final MembersApi? membersApi;

  @override
  IntegrationsApi get integrations => integrationsApi!;

  @override
  MembersApi get members => membersApi!;
}
