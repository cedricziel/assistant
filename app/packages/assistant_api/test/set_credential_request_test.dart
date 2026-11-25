import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for SetCredentialRequest
void main() {
  final instance = SetCredentialRequestBuilder();
  // TODO add properties to the builder and call build()

  group(SetCredentialRequest, () {
    // The header that carries the value. Only accepted when the integration has no header name yet, and then only from someone who manages its layer.
    // String headerName
    test('to test the property `headerName`', () async {
      // TODO
    });

    // The secret that fills the header template, e.g. the token after `Bearer `. Surrounding whitespace is dropped. It is stored encrypted and never returned.
    // String value
    test('to test the property `value`', () async {
      // TODO
    });
  });
}
