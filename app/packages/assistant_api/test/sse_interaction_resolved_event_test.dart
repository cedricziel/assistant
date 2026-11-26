import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for SseInteractionResolvedEvent
void main() {
  final instance = SseInteractionResolvedEventBuilder();
  // TODO add properties to the builder and call build()

  group(SseInteractionResolvedEvent, () {
    // String interactionId
    test('to test the property `interactionId`', () async {
      // TODO
    });

    // String kind
    test('to test the property `kind`', () async {
      // TODO
    });

    // InteractionOutcomeView outcome
    test('to test the property `outcome`', () async {
      // TODO
    });

    // DateTime resolvedAt
    test('to test the property `resolvedAt`', () async {
      // TODO
    });

    // The owner's answer. Only when `outcome` is `resolved`.
    // JsonObject response
    test('to test the property `response`', () async {
      // TODO
    });

    // Id of the permission rule an \"always\" answer saved.
    // String ruleId
    test('to test the property `ruleId`', () async {
      // TODO
    });
  });
}
