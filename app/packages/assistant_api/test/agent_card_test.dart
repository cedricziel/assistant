import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for AgentCard
void main() {
  final instance = AgentCardBuilder();
  // TODO add properties to the builder and call build()

  group(AgentCard, () {
    // A2A capability set supported by the agent.
    // AgentCapabilities capabilities
    test('to test the property `capabilities`', () async {
      // TODO
    });

    // Input modes the agent supports across all skills (media types).
    // BuiltList<String> defaultInputModes
    test('to test the property `defaultInputModes`', () async {
      // TODO
    });

    // Output media types supported by this agent.
    // BuiltList<String> defaultOutputModes
    test('to test the property `defaultOutputModes`', () async {
      // TODO
    });

    // A human-readable description of the agent's purpose.
    // String description
    test('to test the property `description`', () async {
      // TODO
    });

    // A URL providing additional documentation about the agent.
    // String documentationUrl
    test('to test the property `documentationUrl`', () async {
      // TODO
    });

    // A URL to an icon for the agent.
    // String iconUrl
    test('to test the property `iconUrl`', () async {
      // TODO
    });

    // A human-readable name for the agent.
    // String name
    test('to test the property `name`', () async {
      // TODO
    });

    // The service provider of the agent.
    // AgentProvider provider
    test('to test the property `provider`', () async {
      // TODO
    });

    // Security requirements for contacting the agent.
    // BuiltList<SecurityRequirement> securityRequirements
    test('to test the property `securityRequirements`', () async {
      // TODO
    });

    // Security scheme definitions for authenticating with this agent.
    // BuiltMap<String, SecurityScheme> securitySchemes
    test('to test the property `securitySchemes`', () async {
      // TODO
    });

    // JSON Web Signatures computed for this `AgentCard`.
    // BuiltList<AgentCardSignature> signatures
    test('to test the property `signatures`', () async {
      // TODO
    });

    // Skills represent the abilities of an agent.
    // BuiltList<AgentSkill> skills
    test('to test the property `skills`', () async {
      // TODO
    });

    // Ordered list of supported interfaces. The first entry is preferred.
    // BuiltList<AgentInterface> supportedInterfaces
    test('to test the property `supportedInterfaces`', () async {
      // TODO
    });

    // The version of the agent (e.g., \"1.0.0\").
    // String version
    test('to test the property `version`', () async {
      // TODO
    });
  });
}
