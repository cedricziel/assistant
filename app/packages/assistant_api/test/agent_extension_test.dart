import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for AgentExtension
void main() {
  final instance = AgentExtensionBuilder();
  // TODO add properties to the builder and call build()

  group(AgentExtension, () {
    // A human-readable description of how this agent uses the extension.
    // String description
    test('to test the property `description`', () async {
      // TODO
    });

    // Extension-specific configuration parameters.
    // BuiltMap<String, JsonObject> params
    test('to test the property `params`', () async {
      // TODO
    });

    // If true, the client must understand and comply with the extension.
    // bool required_
    test('to test the property `required_`', () async {
      // TODO
    });

    // The unique URI identifying the extension.
    // String uri
    test('to test the property `uri`', () async {
      // TODO
    });

  });
}
