import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for SendMessageConfiguration
void main() {
  final instance = SendMessageConfigurationBuilder();
  // TODO add properties to the builder and call build()

  group(SendMessageConfiguration, () {
    // Media types the client accepts for response parts.
    // BuiltList<String> acceptedOutputModes
    test('to test the property `acceptedOutputModes`', () async {
      // TODO
    });

    // If true, wait until the task reaches a terminal or interrupted state.
    // bool blocking
    test('to test the property `blocking`', () async {
      // TODO
    });

    // Max number of recent messages to include in the response.
    // int historyLength
    test('to test the property `historyLength`', () async {
      // TODO
    });

    // Push notification configuration for task updates.
    // PushNotificationConfig pushNotificationConfig
    test('to test the property `pushNotificationConfig`', () async {
      // TODO
    });

  });
}
