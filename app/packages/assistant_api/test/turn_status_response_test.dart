import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for TurnStatusResponse
void main() {
  final instance = TurnStatusResponseBuilder();
  // TODO add properties to the builder and call build()

  group(TurnStatusResponse, () {
    // String conversationId
    test('to test the property `conversationId`', () async {
      // TODO
    });

    // Wall-clock timestamp of the most recent SSE event recorded for the turn, in RFC 3339 format. `null` when [`state`] is `unknown`.
    // String lastEventAt
    test('to test the property `lastEventAt`', () async {
      // TODO
    });

    // Kind of the most recent SSE event (`run_started`, `token`, `status`, `thinking`, `tool_result`, `agent_error`, `done`, etc.). `null` when [`state`] is `unknown`.
    // String lastEventKind
    test('to test the property `lastEventKind`', () async {
      // TODO
    });

    // TurnState state
    test('to test the property `state`', () async {
      // TODO
    });

    // String turnId
    test('to test the property `turnId`', () async {
      // TODO
    });
  });
}
