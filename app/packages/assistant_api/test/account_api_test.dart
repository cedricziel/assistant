import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for AccountApi
void main() {
  final instance = AssistantApi().getAccountApi();

  group(AccountApi, () {
    // `POST /api/users/me/password` — change the caller's password and revoke all their refresh tokens. The current access JWT keeps working until it naturally expires; API keys are untouched.
    //
    //Future changePassword(ChangePasswordRequest changePasswordRequest) async
    test('test changePassword', () async {
      // TODO
    });

    // `GET /api/users/me` — return the caller's `UserDetail`. Works in any `auth_mode`.
    //
    //Future<UserDetail> getCurrentUser() async
    test('test getCurrentUser', () async {
      // TODO
    });

    // `PATCH /api/users/me` — update the caller's name and/or email.
    //
    //Future<UpdateCurrentUserResponse> updateCurrentUser(UpdateCurrentUserRequest updateCurrentUserRequest) async
    test('test updateCurrentUser', () async {
      // TODO
    });
  });
}
