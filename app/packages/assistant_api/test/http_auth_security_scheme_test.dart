import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for HttpAuthSecurityScheme
void main() {
  final instance = HttpAuthSecuritySchemeBuilder();
  // TODO add properties to the builder and call build()

  group(HttpAuthSecurityScheme, () {
    // A hint for how the bearer token is formatted (e.g., \"JWT\").
    // String bearerFormat
    test('to test the property `bearerFormat`', () async {
      // TODO
    });

    // An optional description for the security scheme.
    // String description
    test('to test the property `description`', () async {
      // TODO
    });

    // The HTTP Authentication scheme (e.g., \"Bearer\").
    // String scheme
    test('to test the property `scheme`', () async {
      // TODO
    });

  });
}
