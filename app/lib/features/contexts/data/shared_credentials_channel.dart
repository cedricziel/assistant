import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// Platform channel that bridges to native Apple code for shared Keychain
/// operations.
///
/// On iOS, only [getTeamPrefix] is needed — `flutter_secure_storage` writes
/// directly to the shared Keychain group via `IOSOptions(groupId:)`.
///
/// On macOS, the `flutter_secure_storage` plugin ignores `groupId`
/// (`#if os(iOS)` in the native code), so [syncCredentialsMacOS] performs the
/// write on the native side.
class SharedCredentialsChannel {
  static const _channel = MethodChannel(
    'com.cedricziel.assistant/shared_credentials',
  );

  static String? _cachedTeamPrefix;

  /// Returns the Apple Team ID prefix (e.g. "ABCDE12345.") resolved at
  /// runtime from the Keychain. The result is cached for the app's lifetime.
  ///
  /// Returns `null` on platforms without a native channel (web).
  static Future<String?> getTeamPrefix() async {
    if (_cachedTeamPrefix != null) return _cachedTeamPrefix;

    try {
      final prefix = await _channel.invokeMethod<String>('getTeamPrefix');
      _cachedTeamPrefix = prefix;
      return prefix;
    } on MissingPluginException {
      // Web platform — no native channel available.
      return null;
    } catch (e) {
      debugPrint('SharedCredentialsChannel.getTeamPrefix failed: $e');
      return null;
    }
  }

  /// Writes credentials to the shared Keychain access group via the native
  /// method channel. **Only used on macOS** where `flutter_secure_storage`'s
  /// `IOSOptions.groupId` has no effect.
  static Future<void> syncCredentialsMacOS({
    required String? serverUrl,
    required String? authToken,
  }) async {
    try {
      await _channel.invokeMethod('syncCredentials', {
        'serverUrl': serverUrl,
        'authToken': authToken,
      });
    } on MissingPluginException {
      // Web platform — no native channel available.
    } catch (e) {
      debugPrint('SharedCredentialsChannel.syncCredentialsMacOS failed: $e');
    }
  }
}
