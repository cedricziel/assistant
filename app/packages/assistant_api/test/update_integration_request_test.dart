import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for UpdateIntegrationRequest
void main() {
  final instance = UpdateIntegrationRequestBuilder();
  // TODO add properties to the builder and call build()

  group(UpdateIntegrationRequest, () {
    // Custom URL integrations only. Changing it deletes the stored credentials, so they cannot be sent to the new host.
    // String connectUrl
    test('to test the property `connectUrl`', () async {
      // TODO
    });

    // bool enabled
    test('to test the property `enabled`', () async {
      // TODO
    });

    // Custom URL integrations only.
    // String headerName
    test('to test the property `headerName`', () async {
      // TODO
    });

    // Custom URL integrations only.
    // String headerValuePrefix
    test('to test the property `headerValuePrefix`', () async {
      // TODO
    });

    // String name
    test('to test the property `name`', () async {
      // TODO
    });

    // String slug
    test('to test the property `slug`', () async {
      // TODO
    });

    // IntegrationTrust trust
    test('to test the property `trust`', () async {
      // TODO
    });
  });
}
