import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for AgentCapabilities
void main() {
  final instance = AgentCapabilitiesBuilder();
  // TODO add properties to the builder and call build()

  group(AgentCapabilities, () {
    // Indicates if the agent supports providing an extended agent card.
    // bool extendedAgentCard
    test('to test the property `extendedAgentCard`', () async {
      // TODO
    });

    // Protocol extensions supported by the agent.
    // BuiltList<AgentExtension> extensions
    test('to test the property `extensions`', () async {
      // TODO
    });

    // Indicates if the agent supports push notifications.
    // bool pushNotifications
    test('to test the property `pushNotifications`', () async {
      // TODO
    });

    // Indicates if the agent supports streaming responses.
    // bool streaming
    test('to test the property `streaming`', () async {
      // TODO
    });

  });
}
