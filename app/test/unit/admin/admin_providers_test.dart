import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/features/admin/admin_providers.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/spaces/space_provider.dart';
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// ---------------------------------------------------------------------------
// Fakes

class _FakeUsersApi extends UsersApi {
  _FakeUsersApi(this._users) : super(Dio(), standardSerializers);

  final List<UserSummary> _users;
  int listCallCount = 0;
  int createCallCount = 0;
  final List<String> deletedIds = [];

  @override
  Future<Response<BuiltList<UserSummary>>> listUsers({
    required String orgId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    listCallCount++;
    return Response(
      requestOptions: RequestOptions(path: '/api/orgs/$orgId/users'),
      statusCode: 200,
      data: BuiltList<UserSummary>.from(_users),
    );
  }

  @override
  Future<Response<UserDetail>> createUser({
    required String orgId,
    required CreateUserRequest createUserRequest,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    createCallCount++;
    final detail = UserDetail(
      (b) => b
        ..id = 'new-user-id'
        ..email = createUserRequest.email
        ..name = createUserRequest.name
        ..orgId = orgId
        ..createdAt = '2025-01-15'
        ..updatedAt = '2025-01-15',
    );
    return Response(
      requestOptions: RequestOptions(path: '/api/orgs/$orgId/users'),
      statusCode: 201,
      data: detail,
    );
  }

  @override
  Future<Response<void>> deleteUser({
    required String orgId,
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
      requestOptions: RequestOptions(path: '/api/orgs/$orgId/users/$id'),
      statusCode: 204,
    );
  }
}

class _FakeMembersApi extends MembersApi {
  _FakeMembersApi(this._members) : super(Dio(), standardSerializers);

  final List<MemberEntry> _members;
  int listCallCount = 0;
  int addCallCount = 0;
  final List<String> removedUserIds = [];

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
    listCallCount++;
    return Response(
      requestOptions: RequestOptions(
        path: '/api/orgs/$orgId/spaces/$spaceId/members',
      ),
      statusCode: 200,
      data: BuiltList<MemberEntry>.from(_members),
    );
  }

  @override
  Future<Response<MemberEntry>> addMember({
    required String orgId,
    required String spaceId,
    required AddMemberRequest addMemberRequest,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    addCallCount++;
    final entry = MemberEntry(
      (b) => b
        ..userId = addMemberRequest.userId
        ..spaceId = spaceId
        ..role = addMemberRequest.role
        ..createdAt = '2025-01-15',
    );
    return Response(
      requestOptions: RequestOptions(
        path: '/api/orgs/$orgId/spaces/$spaceId/members',
      ),
      statusCode: 201,
      data: entry,
    );
  }

  @override
  Future<Response<void>> removeMember({
    required String orgId,
    required String spaceId,
    required String userId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    removedUserIds.add(userId);
    return Response(
      requestOptions: RequestOptions(
        path: '/api/orgs/$orgId/spaces/$spaceId/members/$userId',
      ),
      statusCode: 204,
    );
  }
}

class _FakeCatalogApi extends CatalogApi {
  _FakeCatalogApi(this._items) : super(Dio(), standardSerializers);

  final List<CatalogItemResponse> _items;
  int listCallCount = 0;
  int publishCallCount = 0;
  final List<String> deletedIds = [];

  @override
  Future<Response<BuiltList<CatalogItemResponse>>> listCatalog({
    required String orgId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    listCallCount++;
    return Response(
      requestOptions: RequestOptions(path: '/api/orgs/$orgId/catalog'),
      statusCode: 200,
      data: BuiltList<CatalogItemResponse>.from(_items),
    );
  }

  @override
  Future<Response<CatalogItemResponse>> publishCatalogItem({
    required String orgId,
    required PublishCatalogItemRequest publishCatalogItemRequest,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    publishCallCount++;
    final item = CatalogItemResponse(
      (b) => b
        ..id = 'new-item-id'
        ..name = publishCatalogItemRequest.name
        ..description = publishCatalogItemRequest.description ?? ''
        ..resourceType = publishCatalogItemRequest.resourceType
        ..createdAt = '2025-01-15',
    );
    return Response(
      requestOptions: RequestOptions(path: '/api/orgs/$orgId/catalog'),
      statusCode: 201,
      data: item,
    );
  }

  @override
  Future<Response<void>> deleteCatalogItem({
    required String orgId,
    required String itemId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    deletedIds.add(itemId);
    return Response(
      requestOptions: RequestOptions(path: '/api/orgs/$orgId/catalog/$itemId'),
      statusCode: 204,
    );
  }
}

class _FakeSpacesApi extends SpacesApi {
  _FakeSpacesApi() : super(Dio(), standardSerializers);

  int createCallCount = 0;
  final List<String> deletedIds = [];

  @override
  Future<Response<SpaceDetail>> createSpace({
    required String orgId,
    required CreateSpaceRequest createSpaceRequest,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    createCallCount++;
    final detail = SpaceDetail(
      (b) => b
        ..id = 'new-space-id'
        ..name = createSpaceRequest.name
        ..slug = createSpaceRequest.slug
        ..orgId = orgId
        ..createdAt = '2025-01-15'
        ..updatedAt = '2025-01-15',
    );
    return Response(
      requestOptions: RequestOptions(path: '/api/orgs/$orgId/spaces'),
      statusCode: 201,
      data: detail,
    );
  }

  @override
  Future<Response<void>> deleteSpace({
    required String orgId,
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
      requestOptions: RequestOptions(path: '/api/orgs/$orgId/spaces/$id'),
      statusCode: 204,
    );
  }

  @override
  Future<Response<BuiltList<SpaceSummary>>> listSpaces({
    required String orgId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    return Response(
      requestOptions: RequestOptions(path: '/api/orgs/$orgId/spaces'),
      statusCode: 200,
      data: BuiltList<SpaceSummary>.from(<SpaceSummary>[]),
    );
  }
}

class _FakeApiClient extends ApiClient {
  _FakeApiClient({
    _FakeUsersApi? usersApi,
    _FakeMembersApi? membersApi,
    _FakeCatalogApi? catalogApi,
    _FakeSpacesApi? spacesApi,
  }) : _usersApi = usersApi ?? _FakeUsersApi([]),
       _membersApi = membersApi ?? _FakeMembersApi([]),
       _catalogApi = catalogApi ?? _FakeCatalogApi([]),
       _spacesApi = spacesApi ?? _FakeSpacesApi(),
       super(baseUrl: 'http://fake.test', token: 'test-token');

  final _FakeUsersApi _usersApi;
  final _FakeMembersApi _membersApi;
  final _FakeCatalogApi _catalogApi;
  final _FakeSpacesApi _spacesApi;

  @override
  UsersApi get users => _usersApi;

  @override
  MembersApi get members => _membersApi;

  @override
  CatalogApi get catalog => _catalogApi;

  @override
  SpacesApi get spaces => _spacesApi;
}

UserSummary _makeUser({
  required String id,
  required String name,
  String email = 'test@example.com',
}) {
  return UserSummary(
    (b) => b
      ..id = id
      ..name = name
      ..email = email,
  );
}

MemberEntry _makeMember({required String userId, String role = 'member'}) {
  return MemberEntry(
    (b) => b
      ..userId = userId
      ..spaceId = 'space-1'
      ..role = role
      ..createdAt = '2025-01-15',
  );
}

CatalogItemResponse _makeCatalogItem({
  required String id,
  required String name,
  String resourceType = 'skill',
}) {
  return CatalogItemResponse(
    (b) => b
      ..id = id
      ..name = name
      ..description = 'A test item'
      ..resourceType = resourceType
      ..createdAt = '2025-01-15',
  );
}

ProviderContainer _createContainer({
  required ApiClient api,
  String orgId = 'org-1',
  String spaceId = 'space-1',
}) {
  return ProviderContainer(
    overrides: [
      apiClientProvider.overrideWithValue(api),
      spaceSelectionProvider.overrideWith(() {
        return _PreloadedSelectionNotifier(orgId: orgId, spaceId: spaceId);
      }),
    ],
  );
}

class _PreloadedSelectionNotifier extends SpaceSelectionNotifier {
  _PreloadedSelectionNotifier({this.orgId, this.spaceId});
  final String? orgId;
  final String? spaceId;

  @override
  SpaceSelection build() => SpaceSelection(
    orgId: orgId,
    orgName: 'Test Org',
    spaceId: spaceId,
    spaceName: 'Test Space',
  );
}

// ---------------------------------------------------------------------------
// Tests

void main() {
  group('AdminUsersNotifier', () {
    test('loads users for the active org', () async {
      final users = [_makeUser(id: '1', name: 'Alice')];
      final api = _FakeApiClient(usersApi: _FakeUsersApi(users));
      final container = _createContainer(api: api);
      addTearDown(container.dispose);

      final result = await container.read(adminUsersProvider.future);
      expect(result, hasLength(1));
      expect(result.first.name, 'Alice');
    });

    test('returns empty when apiClient is null', () async {
      final container = ProviderContainer(
        overrides: [apiClientProvider.overrideWithValue(null)],
      );
      addTearDown(container.dispose);

      final result = await container.read(adminUsersProvider.future);
      expect(result, isEmpty);
    });

    test('inviteUser calls API and returns detail', () async {
      final usersApi = _FakeUsersApi([]);
      final api = _FakeApiClient(usersApi: usersApi);
      final container = _createContainer(api: api);
      addTearDown(container.dispose);

      await container.read(adminUsersProvider.future);
      final result = await container
          .read(adminUsersProvider.notifier)
          .inviteUser(email: 'bob@example.com', name: 'Bob');

      expect(result, isNotNull);
      expect(result!.email, 'bob@example.com');
      expect(usersApi.createCallCount, 1);
    });

    test('removeUser calls API delete', () async {
      final usersApi = _FakeUsersApi([_makeUser(id: 'u1', name: 'Old User')]);
      final api = _FakeApiClient(usersApi: usersApi);
      final container = _createContainer(api: api);
      addTearDown(container.dispose);

      await container.read(adminUsersProvider.future);
      await container.read(adminUsersProvider.notifier).removeUser('u1');

      expect(usersApi.deletedIds, ['u1']);
    });
  });

  group('AdminMembersNotifier', () {
    test('loads members for active org and space', () async {
      final members = [_makeMember(userId: 'u1', role: 'member')];
      final api = _FakeApiClient(membersApi: _FakeMembersApi(members));
      final container = _createContainer(api: api);
      addTearDown(container.dispose);

      final result = await container.read(adminMembersProvider.future);
      expect(result, hasLength(1));
      expect(result.first.userId, 'u1');
    });

    test('addMember calls API and returns entry', () async {
      final membersApi = _FakeMembersApi([]);
      final api = _FakeApiClient(membersApi: membersApi);
      final container = _createContainer(api: api);
      addTearDown(container.dispose);

      await container.read(adminMembersProvider.future);
      final result = await container
          .read(adminMembersProvider.notifier)
          .addMember(userId: 'u2', role: 'member');

      expect(result, isNotNull);
      expect(result!.userId, 'u2');
      expect(membersApi.addCallCount, 1);
    });

    test('removeMember calls API delete', () async {
      final membersApi = _FakeMembersApi([_makeMember(userId: 'u1')]);
      final api = _FakeApiClient(membersApi: membersApi);
      final container = _createContainer(api: api);
      addTearDown(container.dispose);

      await container.read(adminMembersProvider.future);
      await container.read(adminMembersProvider.notifier).removeMember('u1');

      expect(membersApi.removedUserIds, ['u1']);
    });
  });

  group('AdminCatalogNotifier', () {
    test('loads catalog items for active org', () async {
      final items = [_makeCatalogItem(id: 'c1', name: 'My Skill')];
      final api = _FakeApiClient(catalogApi: _FakeCatalogApi(items));
      final container = _createContainer(api: api);
      addTearDown(container.dispose);

      final result = await container.read(adminCatalogProvider.future);
      expect(result, hasLength(1));
      expect(result.first.name, 'My Skill');
    });

    test('publishItem calls API and returns item', () async {
      final catalogApi = _FakeCatalogApi([]);
      final api = _FakeApiClient(catalogApi: catalogApi);
      final container = _createContainer(api: api);
      addTearDown(container.dispose);

      await container.read(adminCatalogProvider.future);
      final result = await container
          .read(adminCatalogProvider.notifier)
          .publishItem(name: 'New Skill', resourceType: 'skill');

      expect(result, isNotNull);
      expect(result!.name, 'New Skill');
      expect(catalogApi.publishCallCount, 1);
    });

    test('removeItem calls API delete', () async {
      final catalogApi = _FakeCatalogApi([
        _makeCatalogItem(id: 'c1', name: 'Old Skill'),
      ]);
      final api = _FakeApiClient(catalogApi: catalogApi);
      final container = _createContainer(api: api);
      addTearDown(container.dispose);

      await container.read(adminCatalogProvider.future);
      await container.read(adminCatalogProvider.notifier).removeItem('c1');

      expect(catalogApi.deletedIds, ['c1']);
    });
  });

  group('AdminSpacesNotifier', () {
    test('createSpace calls API', () async {
      final spacesApi = _FakeSpacesApi();
      final api = _FakeApiClient(spacesApi: spacesApi);
      final container = _createContainer(api: api);
      addTearDown(container.dispose);

      // The spacesProvider is already from space_provider.dart, so test
      // through the admin notifier which exposes createSpace / deleteSpace.
      await container.read(adminSpacesProvider.future);
      final result = await container
          .read(adminSpacesProvider.notifier)
          .createSpace(name: 'Dev', slug: 'dev');

      expect(result, isNotNull);
      expect(result!.name, 'Dev');
      expect(spacesApi.createCallCount, 1);
    });

    test('deleteSpace calls API delete', () async {
      final spacesApi = _FakeSpacesApi();
      final api = _FakeApiClient(spacesApi: spacesApi);
      final container = _createContainer(api: api);
      addTearDown(container.dispose);

      await container.read(adminSpacesProvider.future);
      await container.read(adminSpacesProvider.notifier).deleteSpace('s1');

      expect(spacesApi.deletedIds, ['s1']);
    });
  });
}
