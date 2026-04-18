import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for ApiKeySecurityScheme
void main() {
  final instance = ApiKeySecuritySchemeBuilder();
  // TODO add properties to the builder and call build()

  group(ApiKeySecurityScheme, () {
    // An optional description for the security scheme.
    // String description
    test('to test the property `description`', () async {
      // TODO
    });

    // The location of the API key: \"query\", \"header\", or \"cookie\".
    // String location
    test('to test the property `location`', () async {
      // TODO
    });

    // The name of the header, query, or cookie parameter to be used.
    // String name
    test('to test the property `name`', () async {
      // TODO
    });
  });
}
