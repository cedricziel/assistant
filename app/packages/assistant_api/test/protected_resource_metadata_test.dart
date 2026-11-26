import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for ProtectedResourceMetadata
void main() {
  final instance = ProtectedResourceMetadataBuilder();
  // TODO add properties to the builder and call build()

  group(ProtectedResourceMetadata, () {
    // Authorization servers that issue tokens for this resource.
    // BuiltList<String> authorizationServers
    test('to test the property `authorizationServers`', () async {
      // TODO
    });

    // BuiltList<String> bearerMethodsSupported
    test('to test the property `bearerMethodsSupported`', () async {
      // TODO
    });

    // Canonical URL of the MCP endpoint; tokens must be issued for it.
    // String resource
    test('to test the property `resource`', () async {
      // TODO
    });

    // BuiltList<String> scopesSupported
    test('to test the property `scopesSupported`', () async {
      // TODO
    });
  });
}
