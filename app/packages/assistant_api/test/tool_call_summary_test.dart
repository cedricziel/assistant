import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for ToolCallSummary
void main() {
  final instance = ToolCallSummaryBuilder();
  // TODO add properties to the builder and call build()

  group(ToolCallSummary, () {
    // JsonObject arguments
    test('to test the property `arguments`', () async {
      // TODO
    });

    // The tool that was called.
    // String name
    test('to test the property `name`', () async {
      // TODO
    });

    // The tool's output, truncated to a reasonable display length.
    // String result
    test('to test the property `result`', () async {
      // TODO
    });

    // `\"ok\"`, `\"error\"`, or `\"denied\"`.
    // String status
    test('to test the property `status`', () async {
      // TODO
    });
  });
}
