import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for CallMcpAppToolRequest
void main() {
  final instance = CallMcpAppToolRequestBuilder();
  // TODO add properties to the builder and call build()

  group(CallMcpAppToolRequest, () {
    // The tool's arguments.
    // JsonObject arguments
    test('to test the property `arguments`', () async {
      // TODO
    });

    // The user confirmed this call. Required for a tool without `readOnlyHint: true`, and for any call the permission policy leaves to a human; without it such a call is refused with `409`.
    // bool confirmed
    test('to test the property `confirmed`', () async {
      // TODO
    });

    // The tool's name on the app's server.
    // String name
    test('to test the property `name`', () async {
      // TODO
    });
  });
}
