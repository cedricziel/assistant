import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for OAuth2SecurityScheme
void main() {
  final instance = OAuth2SecuritySchemeBuilder();
  // TODO add properties to the builder and call build()

  group(OAuth2SecurityScheme, () {
    // An optional description for the security scheme.
    // String description
    test('to test the property `description`', () async {
      // TODO
    });

    // Configuration for the supported OAuth 2.0 flows.
    // OAuthFlows flows
    test('to test the property `flows`', () async {
      // TODO
    });

    // URL to the OAuth2 authorization server metadata (RFC 8414).
    // String oauth2MetadataUrl
    test('to test the property `oauth2MetadataUrl`', () async {
      // TODO
    });

  });
}
