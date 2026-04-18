import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for SubscribeRequest
void main() {
  final instance = SubscribeRequestBuilder();
  // TODO add properties to the builder and call build()

  group(SubscribeRequest, () {
    // Base64url-encoded 16-byte authentication secret from the browser subscription.
    // String auth
    test('to test the property `auth`', () async {
      // TODO
    });

    // The push endpoint URL assigned by the browser's push service.
    // String endpoint
    test('to test the property `endpoint`', () async {
      // TODO
    });

    // Base64url-encoded P-256 ECDH public key from the browser subscription.
    // String p256dh
    test('to test the property `p256dh`', () async {
      // TODO
    });
  });
}
