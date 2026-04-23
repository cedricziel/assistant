import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for OauthApi
void main() {
  final instance = AssistantApi().getOauthApi();

  group(OauthApi, () {
    // GET /oauth/authorize — render login form.
    //
    //Future authorizeGet({ String responseType, String clientId, String redirectUri, String state, String codeChallenge, String codeChallengeMethod, String scope }) async
    test('test authorizeGet', () async {
      // TODO
    });

    // POST /oauth/authorize — validate credentials, generate auth code, redirect.
    //
    //Future authorizePost(String clientId, String redirectUri, { String codeChallenge, String email, String password, String scope, String state }) async
    test('test authorizePost', () async {
      // TODO
    });

    // POST /oauth/device — initiate the device authorization flow.
    //
    //Future<DeviceCodeResponseSchema> deviceInitiate(String clientId, { String scope }) async
    test('test deviceInitiate', () async {
      // TODO
    });

    // GET /oauth/device/verify — render page where user enters the code.
    //
    //Future deviceVerifyPage({ String userCode }) async
    test('test deviceVerifyPage', () async {
      // TODO
    });

    // POST /oauth/device/verify — user approves the device code.
    //
    //Future deviceVerifySubmit(String userCode, { String userId }) async
    test('test deviceVerifySubmit', () async {
      // TODO
    });

    // GET /.well-known/oauth-authorization-server
    //
    //Future<ServerMetadata> metadata() async
    test('test metadata', () async {
      // TODO
    });

    // POST /oauth/register — register a new OAuth2 client.
    //
    //Future<ClientInfoSchema> register(ClientRegistrationSchema clientRegistrationSchema) async
    test('test register', () async {
      // TODO
    });

    // POST /oauth/revoke — revoke a refresh token.
    //
    //Future revoke(String token, { String clientId, String tokenTypeHint }) async
    test('test revoke', () async {
      // TODO
    });

    // POST /oauth/token
    //
    //Future<TokenResponse> token(String grantType, { String clientId, String code, String codeVerifier, String deviceCode, String redirectUri, String refreshToken }) async
    test('test token', () async {
      // TODO
    });
  });
}
