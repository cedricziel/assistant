import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for PushNotificationConfig
void main() {
  final instance = PushNotificationConfigBuilder();
  // TODO add properties to the builder and call build()

  group(PushNotificationConfig, () {
    // Authentication information required to send the notification.
    // AuthenticationInfo authentication
    test('to test the property `authentication`', () async {
      // TODO
    });

    // A unique identifier (UUID) for this push notification configuration.
    // String id
    test('to test the property `id`', () async {
      // TODO
    });

    // A token unique for this task or session.
    // String token
    test('to test the property `token`', () async {
      // TODO
    });

    // The URL where the notification should be sent.
    // String url
    test('to test the property `url`', () async {
      // TODO
    });

  });
}
