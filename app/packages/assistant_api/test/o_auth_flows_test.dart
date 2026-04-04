import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for OAuthFlows
void main() {
  final instance = OAuthFlowsBuilder();
  // TODO add properties to the builder and call build()

  group(OAuthFlows, () {
    // Configuration for the OAuth Authorization Code flow.
    // AuthorizationCodeOAuthFlow authorizationCode
    test('to test the property `authorizationCode`', () async {
      // TODO
    });

    // Configuration for the OAuth Client Credentials flow.
    // ClientCredentialsOAuthFlow clientCredentials
    test('to test the property `clientCredentials`', () async {
      // TODO
    });

    // Configuration for the OAuth Device Code flow.
    // DeviceCodeOAuthFlow deviceCode
    test('to test the property `deviceCode`', () async {
      // TODO
    });

    // Deprecated: Use Authorization Code + PKCE instead.
    // ImplicitOAuthFlow implicit
    test('to test the property `implicit`', () async {
      // TODO
    });

    // Deprecated: Use Authorization Code + PKCE or Device Code.
    // PasswordOAuthFlow password
    test('to test the property `password`', () async {
      // TODO
    });

  });
}
