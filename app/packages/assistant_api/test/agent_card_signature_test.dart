import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for AgentCardSignature
void main() {
  final instance = AgentCardSignatureBuilder();
  // TODO add properties to the builder and call build()

  group(AgentCardSignature, () {
    // The unprotected JWS header values.
    // BuiltMap<String, JsonObject> header
    test('to test the property `header`', () async {
      // TODO
    });

    // The protected JWS header, base64url-encoded JSON object.
    // String protected
    test('to test the property `protected`', () async {
      // TODO
    });

    // The computed signature, base64url-encoded.
    // String signature
    test('to test the property `signature`', () async {
      // TODO
    });

  });
}
