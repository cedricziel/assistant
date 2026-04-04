import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for AgentInterface
void main() {
  final instance = AgentInterfaceBuilder();
  // TODO add properties to the builder and call build()

  group(AgentInterface, () {
    // The protocol binding (e.g., \"JSONRPC\", \"GRPC\", \"HTTP+JSON\").
    // String protocolBinding
    test('to test the property `protocolBinding`', () async {
      // TODO
    });

    // The version of the A2A protocol this interface exposes (e.g., \"1.0\").
    // String protocolVersion
    test('to test the property `protocolVersion`', () async {
      // TODO
    });

    // Tenant ID to be used in the request when calling the agent.
    // String tenant
    test('to test the property `tenant`', () async {
      // TODO
    });

    // The URL where this interface is available.
    // String url
    test('to test the property `url`', () async {
      // TODO
    });

  });
}
