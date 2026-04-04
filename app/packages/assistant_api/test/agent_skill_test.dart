import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for AgentSkill
void main() {
  final instance = AgentSkillBuilder();
  // TODO add properties to the builder and call build()

  group(AgentSkill, () {
    // A detailed description of the skill.
    // String description
    test('to test the property `description`', () async {
      // TODO
    });

    // Example prompts or scenarios that this skill can handle.
    // BuiltList<String> examples
    test('to test the property `examples`', () async {
      // TODO
    });

    // A unique identifier for the skill.
    // String id
    test('to test the property `id`', () async {
      // TODO
    });

    // Supported input media types, overriding agent defaults.
    // BuiltList<String> inputModes
    test('to test the property `inputModes`', () async {
      // TODO
    });

    // A human-readable name for the skill.
    // String name
    test('to test the property `name`', () async {
      // TODO
    });

    // Supported output media types, overriding agent defaults.
    // BuiltList<String> outputModes
    test('to test the property `outputModes`', () async {
      // TODO
    });

    // Security schemes necessary for this skill.
    // BuiltList<SecurityRequirement> securityRequirements
    test('to test the property `securityRequirements`', () async {
      // TODO
    });

    // Keywords describing the skill's capabilities.
    // BuiltList<String> tags
    test('to test the property `tags`', () async {
      // TODO
    });

  });
}
