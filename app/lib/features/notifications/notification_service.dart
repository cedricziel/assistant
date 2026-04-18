import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:http/http.dart' as http;

import '../connection/connection_provider.dart';
import 'notification_badge_notifier.dart';

// -- JS interop for web push -------------------------------------------------

// ignore: avoid_web_libraries_in_flutter
import 'notification_service_web_stub.dart'
    if (dart.library.js_interop) 'notification_service_web.dart'
    as web_impl;

// ---------------------------------------------------------------------------

/// Platform-agnostic notification service.
///
/// - **macOS**: delegates to `flutter_local_notifications`
///   (`UNUserNotificationCenter`).
/// - **Web (tab open)**: uses `flutter_local_notifications` web support.
/// - **Web (background / tab closed)**: handled by the PWA service worker
///   (`sw.js`) receiving VAPID-signed Web Push messages from the Rust server.
///   `initialize()` registers `sw.js` and posts the subscription to the
///   backend so the server can send pushes.
/// - **Other platforms**: graceful no-op.
class NotificationService {
  NotificationService({required Ref ref}) : _ref = ref;

  final Ref _ref;
  final _plugin = FlutterLocalNotificationsPlugin();

  bool _initialized = false;
  bool _permissionGranted = false;

  /// Go router context used for navigation on notification tap.
  GoRouter? router;

  /// Initialize the notification plugin (lazy — call before first `show()`).
  ///
  /// Idempotent: subsequent calls are no-ops.
  Future<void> initialize() async {
    if (_initialized) return;
    _initialized = true;

    if (!_isSupportedPlatform) {
      debugPrint('[NotificationService] Platform not supported — no-op');
      return;
    }

    const darwinSettings = DarwinInitializationSettings(
      requestAlertPermission: false, // request lazily on first use
      requestSoundPermission: false,
      requestBadgePermission: false,
    );

    const settings = InitializationSettings(
      macOS: darwinSettings,
      iOS: darwinSettings,
    );

    try {
      await _plugin.initialize(
        settings: settings,
        onDidReceiveNotificationResponse: _onNotificationResponse,
      );
    } catch (e) {
      debugPrint('[NotificationService] initialize failed: $e');
      _initialized = false;
      return;
    }

    // Web: register sw.js and subscribe for background push.
    if (kIsWeb) {
      _initializeWebPush();
    }
  }

  /// Request OS/browser notification permission.
  ///
  /// Returns `true` if permission was granted.  Subsequent calls are no-ops
  /// if permission was already granted.
  Future<bool> requestPermission() async {
    if (_permissionGranted) return true;
    if (!_isSupportedPlatform) return false;

    try {
      if (defaultTargetPlatform == TargetPlatform.macOS) {
        final granted = await _plugin
            .resolvePlatformSpecificImplementation<
              MacOSFlutterLocalNotificationsPlugin
            >()
            ?.requestPermissions(alert: true, badge: true, sound: true);
        _permissionGranted = granted ?? false;
      } else if (defaultTargetPlatform == TargetPlatform.iOS) {
        final granted = await _plugin
            .resolvePlatformSpecificImplementation<
              IOSFlutterLocalNotificationsPlugin
            >()
            ?.requestPermissions(alert: true, badge: true, sound: true);
        _permissionGranted = granted ?? false;
      } else if (kIsWeb) {
        // Web permission is requested by the browser when subscribing to push.
        _permissionGranted = await web_impl.requestWebPermission();
      }
    } catch (e) {
      debugPrint('[NotificationService] requestPermission failed: $e');
    }

    return _permissionGranted;
  }

  /// Show a local notification.
  ///
  /// On macOS: fires via `flutter_local_notifications` (foreground + background).
  /// On web (foreground): fires via flutter_local_notifications web.
  /// On web (background): handled by the service worker — no-op here.
  ///
  /// Respects `notifyChatMessages` / related preferences before calling.
  Future<void> show(String title, String body, {String? conversationId}) async {
    if (!_isSupportedPlatform) return;
    if (!_initialized) await initialize();

    final granted = await requestPermission();
    if (!granted) return;

    // Truncate body to 80 characters.
    final truncated = body.length > 80 ? '${body.substring(0, 77)}…' : body;

    const darwinDetails = DarwinNotificationDetails(
      presentAlert: true,
      presentBadge: true,
      presentSound: true,
    );
    const details = NotificationDetails(
      macOS: darwinDetails,
      iOS: darwinDetails,
    );

    try {
      await _plugin.show(
        id: 0,
        title: title,
        body: truncated,
        notificationDetails: details,
        payload: conversationId,
      );
    } catch (e) {
      debugPrint('[NotificationService] show failed: $e');
      return;
    }

    // Increment badge counter on macOS and iOS.
    if (!kIsWeb &&
        (defaultTargetPlatform == TargetPlatform.macOS ||
            defaultTargetPlatform == TargetPlatform.iOS)) {
      _ref.read(notificationBadgeProvider.notifier).increment();
    }
  }

  /// Unsubscribe from Web Push (called when user disables notifications or
  /// on dispose).
  Future<void> unsubscribeFromWebPush() async {
    if (!kIsWeb) return;
    final endpoint = await web_impl.getCurrentPushEndpoint();
    if (endpoint == null) return;

    final profile = _ref.read(activeProfileProvider);
    if (profile == null) return;

    try {
      await http.delete(
        Uri.parse('${profile.baseUrl}/api/push/subscribe'),
        headers: {
          'Content-Type': 'application/json',
          'Authorization': 'Bearer ${profile.token}',
        },
        body: jsonEncode({'endpoint': endpoint}),
      );
    } catch (e) {
      debugPrint('[NotificationService] unsubscribeFromWebPush failed: $e');
    }
  }

  // -- Internal ---------------------------------------------------------------

  bool get _isSupportedPlatform {
    if (kIsWeb) return true;
    if (defaultTargetPlatform == TargetPlatform.macOS) return true;
    if (defaultTargetPlatform == TargetPlatform.iOS) return true;
    return false;
  }

  Future<void> _initializeWebPush() async {
    final profile = _ref.read(activeProfileProvider);
    if (profile == null) return;

    try {
      // 1. Fetch VAPID public key from the server.
      final resp = await http.get(
        Uri.parse('${profile.baseUrl}/api/push/vapid-public-key'),
        headers: {'Authorization': 'Bearer ${profile.token}'},
      );
      if (resp.statusCode != 200) {
        debugPrint(
          '[NotificationService] VAPID key fetch failed: ${resp.statusCode}',
        );
        return;
      }
      final vapidKey =
          (jsonDecode(resp.body) as Map<String, dynamic>)['public_key']
              as String;

      // 2. Register sw.js and subscribe to push.
      final subscription = await web_impl.subscribeToWebPush(vapidKey);
      if (subscription == null) return;

      // 3. POST the subscription to the backend.
      final postResp = await http.post(
        Uri.parse('${profile.baseUrl}/api/push/subscribe'),
        headers: {
          'Content-Type': 'application/json',
          'Authorization': 'Bearer ${profile.token}',
        },
        body: jsonEncode(subscription),
      );
      if (postResp.statusCode == 201) {
        _permissionGranted = true;
        debugPrint('[NotificationService] Web Push subscription registered');
      }
    } catch (e) {
      debugPrint('[NotificationService] Web Push init failed: $e');
    }
  }

  void _onNotificationResponse(NotificationResponse response) {
    final conversationId = response.payload;
    if (conversationId != null && conversationId.isNotEmpty && router != null) {
      router!.go('/chat/$conversationId');
    }
    // Clear badge when user taps a notification.
    _ref.read(notificationBadgeProvider.notifier).clear();
  }
}

// -- Provider ----------------------------------------------------------------

/// Singleton `NotificationService` provider.
///
/// Lazy: initialized on first access.  Keeps the service alive for the
/// lifetime of the app.
final notificationServiceProvider = Provider<NotificationService>((ref) {
  return NotificationService(ref: ref);
});
