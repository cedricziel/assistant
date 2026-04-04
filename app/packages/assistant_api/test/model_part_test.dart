import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for ModelPart
void main() {
  final instance = ModelPartBuilder();
  // TODO add properties to the builder and call build()

  group(ModelPart, () {
    // Arbitrary structured data as a JSON value.
    // JsonObject data
    test('to test the property `data`', () async {
      // TODO
    });

    // An optional filename for the file.
    // String filename
    test('to test the property `filename`', () async {
      // TODO
    });

    // The MIME type of the part content.
    // String mediaType
    test('to test the property `mediaType`', () async {
      // TODO
    });

    // Metadata associated with this part.
    // BuiltMap<String, JsonObject> metadata
    test('to test the property `metadata`', () async {
      // TODO
    });

    // Raw byte content of a file, base64-encoded in JSON.
    // String raw
    test('to test the property `raw`', () async {
      // TODO
    });

    // The string content of a text part.
    // String text
    test('to test the property `text`', () async {
      // TODO
    });

    // A URL pointing to the file's content.
    // String url
    test('to test the property `url`', () async {
      // TODO
    });

  });
}
