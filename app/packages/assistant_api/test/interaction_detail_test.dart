import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for InteractionDetail
void main() {
  final instance = InteractionDetailBuilder();
  // TODO add properties to the builder and call build()

  group(InteractionDetail, () {
    // String conversationId
    test('to test the property `conversationId`', () async {
      // TODO
    });

    // DateTime createdAt
    test('to test the property `createdAt`', () async {
      // TODO
    });

    // DateTime expiresAt
    test('to test the property `expiresAt`', () async {
      // TODO
    });

    // String interactionId
    test('to test the property `interactionId`', () async {
      // TODO
    });

    // `tool_approval`, `question`, or a kind a newer server added.
    // String kind
    test('to test the property `kind`', () async {
      // TODO
    });

    // Whether the kind renders a card in the timeline.
    // bool rendersCard
    test('to test the property `rendersCard`', () async {
      // TODO
    });

    // The kind-specific request (for `tool_approval`: `tool_name`, `arguments`, `arguments_truncated`, `reason`, `options`, `suggested_rule`; for `question`: `questions`).
    // JsonObject request
    test('to test the property `request`', () async {
      // TODO
    });

    // DateTime resolvedAt
    test('to test the property `resolvedAt`', () async {
      // TODO
    });

    // The owner's answer. Present once `status` is `resolved`.
    // JsonObject response
    test('to test the property `response`', () async {
      // TODO
    });

    // Id of the permission rule an \"always\" answer saved.
    // String ruleId
    test('to test the property `ruleId`', () async {
      // TODO
    });

    // The run (turn) that asked.
    // String runId
    test('to test the property `runId`', () async {
      // TODO
    });

    // InteractionStatusView status
    test('to test the property `status`', () async {
      // TODO
    });

    // Provider id of the tool call this interaction belongs to, if any.
    // String toolCallId
    test('to test the property `toolCallId`', () async {
      // TODO
    });
  });
}
