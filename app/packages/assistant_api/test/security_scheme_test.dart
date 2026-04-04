import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for SecurityScheme
void main() {
  final instance = SecuritySchemeBuilder();
  // TODO add properties to the builder and call build()

  group(SecurityScheme, () {
    // API key-based authentication.
    // ApiKeySecurityScheme apiKeySecurityScheme
    test('to test the property `apiKeySecurityScheme`', () async {
      // TODO
    });

    // HTTP authentication (Basic, Bearer, etc.).
    // HttpAuthSecurityScheme httpAuthSecurityScheme
    test('to test the property `httpAuthSecurityScheme`', () async {
      // TODO
    });

    // Mutual TLS authentication.
    // MutualTlsSecurityScheme mtlsSecurityScheme
    test('to test the property `mtlsSecurityScheme`', () async {
      // TODO
    });

    // OAuth 2.0 authentication.
    // OAuth2SecurityScheme oauth2SecurityScheme
    test('to test the property `oauth2SecurityScheme`', () async {
      // TODO
    });

    // OpenID Connect authentication.
    // OpenIdConnectSecurityScheme openIdConnectSecurityScheme
    test('to test the property `openIdConnectSecurityScheme`', () async {
      // TODO
    });

  });
}
