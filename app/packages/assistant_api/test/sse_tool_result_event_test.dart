import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for SseToolResultEvent
void main() {
  final instance = SseToolResultEventBuilder();
  // TODO add properties to the builder and call build()

  group(SseToolResultEvent, () {
    // The arguments passed to the tool (JSON string).
    // String arguments
    test('to test the property `arguments`', () async {
      // TODO
    });

    // The tool's output or error message.
    // String result
    test('to test the property `result`', () async {
      // TODO
    });

    // Whether the tool call succeeded or failed.
    // String status
    test('to test the property `status`', () async {
      // TODO
    });

    // Name of the tool that was called.
    // String toolName
    test('to test the property `toolName`', () async {
      // TODO
    });
  });
}
