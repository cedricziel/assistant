import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for SurfaceSummary
void main() {
  final instance = SurfaceSummaryBuilder();
  // TODO add properties to the builder and call build()

  group(SurfaceSummary, () {
    // The user message that carried the submitting action.
    // String actionMessageId
    test('to test the property `actionMessageId`', () async {
      // TODO
    });

    // Index of the creating call inside that message's tool calls.
    // int anchorCallIndex
    test('to test the property `anchorCallIndex`', () async {
      // TODO
    });

    // The tool-call message the surface hangs off.
    // String anchorMessageId
    test('to test the property `anchorMessageId`', () async {
      // TODO
    });

    // Whether the user can still answer the surface: it is `active` and no user message follows its anchor on the returned path.
    // bool answerable
    test('to test the property `answerable`', () async {
      // TODO
    });

    // DateTime createdAt
    test('to test the property `createdAt`', () async {
      // TODO
    });

    // String fallbackText
    test('to test the property `fallbackText`', () async {
      // TODO
    });

    // A single synthesized A2UI `createSurface` holding the current components and data model.
    // BuiltList<JsonObject> messages
    test('to test the property `messages`', () async {
      // TODO
    });

    // `\"active\"` or `\"submitted\"`.
    // String status
    test('to test the property `status`', () async {
      // TODO
    });

    // The action that submitted the surface.
    // JsonObject submittedAction
    test('to test the property `submittedAction`', () async {
      // TODO
    });

    // String surfaceId
    test('to test the property `surfaceId`', () async {
      // TODO
    });

    // String title
    test('to test the property `title`', () async {
      // TODO
    });

    // Provider id of the creating tool call, when the provider supplied one.
    // String toolCallId
    test('to test the property `toolCallId`', () async {
      // TODO
    });

    // int turn
    test('to test the property `turn`', () async {
      // TODO
    });
  });
}
