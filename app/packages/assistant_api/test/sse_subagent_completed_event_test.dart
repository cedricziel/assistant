import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for SseSubagentCompletedEvent
void main() {
  final instance = SseSubagentCompletedEventBuilder();
  // TODO add properties to the builder and call build()

  group(SseSubagentCompletedEvent, () {
    // Unique identifier of the subagent.
    // String agentId
    test('to test the property `agentId`', () async {
      // TODO
    });

    // Completion status (e.g. \"success\", \"error\").
    // String status
    test('to test the property `status`', () async {
      // TODO
    });

    // Summary of the subagent's work.
    // String summary
    test('to test the property `summary`', () async {
      // TODO
    });
  });
}
