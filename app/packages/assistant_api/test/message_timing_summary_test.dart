import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for MessageTimingSummary
void main() {
  final instance = MessageTimingSummaryBuilder();
  // TODO add properties to the builder and call build()

  group(MessageTimingSummary, () {
    // Time the model spent generating tokens, in milliseconds.
    // int decodeMs
    test('to test the property `decodeMs`', () async {
      // TODO
    });

    // Output tokens summed over every model call in the turn.
    // int outputTokens
    test('to test the property `outputTokens`', () async {
      // TODO
    });

    // Turn start to the first visible answer token, in milliseconds.
    // int timeToFirstTokenMs
    test('to test the property `timeToFirstTokenMs`', () async {
      // TODO
    });

    // Turn start to just before the final answer was saved, in milliseconds.
    // int totalMs
    test('to test the property `totalMs`', () async {
      // TODO
    });
  });
}
