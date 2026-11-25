import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for IntegrationPolicyResponse
void main() {
  final instance = IntegrationPolicyResponseBuilder();
  // TODO add properties to the builder and call build()

  group(IntegrationPolicyResponse, () {
    // bool allowCustomUrls
    test('to test the property `allowCustomUrls`', () async {
      // TODO
    });

    // bool allowUserIntegrations
    test('to test the property `allowUserIntegrations`', () async {
      // TODO
    });

    // Catalog entry ids allowed when `catalog_mode` is `allowlist`.
    // BuiltList<String> allowedCatalogIds
    test('to test the property `allowedCatalogIds`', () async {
      // TODO
    });

    // CatalogMode catalogMode
    test('to test the property `catalogMode`', () async {
      // TODO
    });

    // Absent until an org admin first saves the policy.
    // DateTime updatedAt
    test('to test the property `updatedAt`', () async {
      // TODO
    });
  });
}
