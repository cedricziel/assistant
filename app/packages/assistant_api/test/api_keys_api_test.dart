import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for ApiKeysApi
void main() {
  final instance = AssistantApi().getApiKeysApi();

  group(ApiKeysApi, () {
    // `POST /api/users/me/api-keys` — create a new scoped API key.
    //
    //Future<CreateApiKeyResponse> createApiKey(CreateApiKeyRequest createApiKeyRequest) async
    test('test createApiKey', () async {
      // TODO
    });

    // `DELETE /api/users/me/api-keys/{id}` — revoke an API key.
    //
    //Future deleteApiKey(String id) async
    test('test deleteApiKey', () async {
      // TODO
    });

    // `GET /api/users/me/api-keys` — list the caller's API keys.
    //
    //Future<BuiltList<ApiKeySummary>> listApiKeys() async
    test('test listApiKeys', () async {
      // TODO
    });
  });
}
