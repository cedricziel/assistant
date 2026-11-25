import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for PutOrgOidcConfigRequest
void main() {
  final instance = PutOrgOidcConfigRequestBuilder();
  // TODO add properties to the builder and call build()

  group(PutOrgOidcConfigRequest, () {
    // BuiltList<String> allowedEmailDomains
    test('to test the property `allowedEmailDomains`', () async {
      // TODO
    });

    // Defaults to `true`.
    // bool autoProvision
    test('to test the property `autoProvision`', () async {
      // TODO
    });

    // String clientId
    test('to test the property `clientId`', () async {
      // TODO
    });

    // Omit to keep the stored secret, `null` to clear it.
    // String clientSecret
    test('to test the property `clientSecret`', () async {
      // TODO
    });

    // `https` URL, or `http` for `localhost` / loopback addresses.
    // String issuerUrl
    test('to test the property `issuerUrl`', () async {
      // TODO
    });
  });
}
