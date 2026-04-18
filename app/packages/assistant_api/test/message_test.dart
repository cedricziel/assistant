import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for Message
void main() {
  final instance = MessageBuilder();
  // TODO add properties to the builder and call build()

  group(Message, () {
    // The context id of the message.
    // String contextId
    test('to test the property `contextId`', () async {
      // TODO
    });

    // The URIs of extensions present or contributing to this message.
    // BuiltList<String> extensions
    test('to test the property `extensions`', () async {
      // TODO
    });

    // Unique identifier (UUID) of the message, created by the message creator.
    // String messageId
    test('to test the property `messageId`', () async {
      // TODO
    });

    // Any metadata to provide along with the message.
    // BuiltMap<String, JsonObject> metadata
    test('to test the property `metadata`', () async {
      // TODO
    });

    // Content parts of the message.
    // BuiltList<ModelPart> parts
    test('to test the property `parts`', () async {
      // TODO
    });

    // A list of task IDs that this message references for additional context.
    // BuiltList<String> referenceTaskIds
    test('to test the property `referenceTaskIds`', () async {
      // TODO
    });

    // Identifies the sender of the message.
    // Role role
    test('to test the property `role`', () async {
      // TODO
    });

    // The task id of the message.
    // String taskId
    test('to test the property `taskId`', () async {
      // TODO
    });
  });
}
