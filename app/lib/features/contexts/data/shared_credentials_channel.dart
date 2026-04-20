import 'package:flutter/services.dart';

/// Platform channel that writes credentials to the shared Keychain access
/// group so native app extensions can read them.
class SharedCredentialsChannel {
  static const _channel = MethodChannel(
    'com.cedricziel.assistant/shared_credentials',
  );

  /// Writes (or clears) the server URL and auth token in the shared
  /// Keychain access group.
  static Future<void> syncCredentials({
    required String? serverUrl,
    required String? authToken,
  }) async {
    try {
      await _channel.invokeMethod('syncCredentials', {
        'serverUrl': serverUrl,
        'authToken': authToken,
      });
    } on MissingPluginException {
      // Platform doesn't support the channel (e.g. web). Silently ignore.
    } catch (_) {
      // Binding not initialized (unit tests) or other platform error.
    }
  }
}
