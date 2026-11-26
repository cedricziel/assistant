import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for SseInteractionRequestedEvent
void main() {
  final instance = SseInteractionRequestedEventBuilder();
  // TODO add properties to the builder and call build()

  group(SseInteractionRequestedEvent, () {
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

    // Provider id of the tool call this belongs to, when there is one.
    // String toolCallId
    test('to test the property `toolCallId`', () async {
      // TODO
    });
  });
}
