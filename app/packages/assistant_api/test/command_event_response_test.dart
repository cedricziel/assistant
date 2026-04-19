import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for CommandEventResponse
void main() {
  final instance = CommandEventResponseBuilder();
  // TODO add properties to the builder and call build()

  group(CommandEventResponse, () {
    // Acknowledgement text returned by the command.
    // String ackText
    test('to test the property `ackText`', () async {
      // TODO
    });

    // Command that was executed.
    // String command
    test('to test the property `command`', () async {
      // TODO
    });

    // When the event was created.
    // DateTime createdAt
    test('to test the property `createdAt`', () async {
      // TODO
    });

    // Event type (always `\"command\"` for now).
    // String eventType
    test('to test the property `eventType`', () async {
      // TODO
    });

    // Unique event ID.
    // String id
    test('to test the property `id`', () async {
      // TODO
    });

    // JsonObject payload
    test('to test the property `payload`', () async {
      // TODO
    });
  });
}
