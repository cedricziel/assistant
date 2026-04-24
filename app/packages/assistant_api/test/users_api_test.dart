import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for UsersApi
void main() {
  final instance = AssistantApi().getUsersApi();

  group(UsersApi, () {
    // `POST /api/orgs/{org_id}/users` — create/invite a user.
    //
    //Future<UserDetail> createUser(String orgId, CreateUserRequest createUserRequest) async
    test('test createUser', () async {
      // TODO
    });

    // `DELETE /api/orgs/{org_id}/users/{id}` — remove a user (org-admin only).
    //
    //Future deleteUser(String orgId, String id) async
    test('test deleteUser', () async {
      // TODO
    });

    // `GET /api/orgs/{org_id}/users/{id}` — get user detail.
    //
    //Future<UserDetail> getUser(String orgId, String id) async
    test('test getUser', () async {
      // TODO
    });

    // `GET /api/orgs/{org_id}/users` — list users in an org.
    //
    //Future<BuiltList<UserSummary>> listUsers(String orgId) async
    test('test listUsers', () async {
      // TODO
    });

    // `PATCH /api/orgs/{org_id}/users/{id}` — update user name or email.
    //
    //Future<UserDetail> updateUser(String orgId, String id, UpdateUserRequest updateUserRequest) async
    test('test updateUser', () async {
      // TODO
    });
  });
}
