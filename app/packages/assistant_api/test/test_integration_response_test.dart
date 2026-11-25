import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for TestIntegrationResponse
void main() {
  final instance = TestIntegrationResponseBuilder();
  // TODO add properties to the builder and call build()

  group(TestIntegrationResponse, () {
    // Why the connection failed.
    // String error
    test('to test the property `error`', () async {
      // TODO
    });

    // DateTime lastConnectedAt
    test('to test the property `lastConnectedAt`', () async {
      // TODO
    });

    // Whether a connection was made and its tools listed.
    // bool ok
    test('to test the property `ok`', () async {
      // TODO
    });

    // The caller's status after this test.
    // IntegrationStatus status
    test('to test the property `status`', () async {
      // TODO
    });

    // int toolCount
    test('to test the property `toolCount`', () async {
      // TODO
    });

    // The tools the server offers, sorted. Empty when the test failed.
    // BuiltList<String> toolNames
    test('to test the property `toolNames`', () async {
      // TODO
    });
  });
}
