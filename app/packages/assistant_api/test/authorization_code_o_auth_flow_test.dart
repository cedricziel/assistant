import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for AuthorizationCodeOAuthFlow
void main() {
  final instance = AuthorizationCodeOAuthFlowBuilder();
  // TODO add properties to the builder and call build()

  group(AuthorizationCodeOAuthFlow, () {
    // The authorization URL.
    // String authorizationUrl
    test('to test the property `authorizationUrl`', () async {
      // TODO
    });

    // Indicates if PKCE (RFC 7636) is required.
    // bool pkceRequired
    test('to test the property `pkceRequired`', () async {
      // TODO
    });

    // The URL for obtaining refresh tokens.
    // String refreshUrl
    test('to test the property `refreshUrl`', () async {
      // TODO
    });

    // Available scopes for the OAuth2 security scheme.
    // BuiltMap<String, String> scopes
    test('to test the property `scopes`', () async {
      // TODO
    });

    // The token URL.
    // String tokenUrl
    test('to test the property `tokenUrl`', () async {
      // TODO
    });

  });
}
