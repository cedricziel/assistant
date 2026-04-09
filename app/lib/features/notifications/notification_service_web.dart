// Web implementation: registers sw.js and subscribes to Web Push.
//
// Uses package:web and dart:js_interop (Flutter 3.x+ / Dart 3+).

import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import 'package:flutter/foundation.dart';
import 'package:web/web.dart' as web;

/// Request browser notification permission.
///
/// Returns `true` if permission was granted.
Future<bool> requestWebPermission() async {
  if (!_isSecureContext()) return false;
  try {
    final result = await web.Notification.requestPermission().toDart;
    // JSString == String comparison
    return result.toDart == 'granted';
  } catch (e) {
    debugPrint('[web_push] requestPermission failed: $e');
    return false;
  }
}

/// Register `sw.js` and subscribe to Web Push with the given VAPID public key.
///
/// Returns a map with `endpoint`, `p256dh`, and `auth` on success, or `null`.
Future<Map<String, dynamic>?> subscribeToWebPush(String vapidPublicKey) async {
  if (!_isSecureContext()) {
    debugPrint('[web_push] Insecure context — Web Push unavailable');
    return null;
  }
  try {
    final sw = web.window.navigator.serviceWorker;

    // 1. Register sw.js (idempotent — browser returns existing reg if already
    //    registered at this path).
    await sw.register('/sw.js'.toJS).toDart;

    // 2. Wait for an active service worker.
    await sw.ready.toDart;

    // 3. Get the registration for push subscription.
    final reg = await sw.getRegistration('/sw.js').toDart;
    if (reg == null) {
      debugPrint('[web_push] No SW registration found after register()');
      return null;
    }

    // 4. Build subscription options.
    final opts = web.PushSubscriptionOptionsInit(
      userVisibleOnly: true,
      applicationServerKey: _base64UrlToJSAny(vapidPublicKey),
    );

    // 5. Subscribe (idempotent — returns existing subscription if present).
    final sub = await reg.pushManager.subscribe(opts).toDart;

    // 6. Extract keys via toJSON() — returns base64url strings, no ArrayBuffer
    //    conversion needed.
    final json = sub.toJSON();
    final keys = json.keys;
    final p256dh =
        (keys.getProperty('p256dh'.toJS) as JSString).toDart;
    final auth =
        (keys.getProperty('auth'.toJS) as JSString).toDart;

    return {
      'endpoint': sub.endpoint,
      'p256dh': p256dh,
      'auth': auth,
    };
  } catch (e) {
    debugPrint('[web_push] subscribeToWebPush failed: $e');
    return null;
  }
}

/// Return the endpoint URL of the current push subscription, or `null`.
Future<String?> getCurrentPushEndpoint() async {
  if (!_isSecureContext()) return null;
  try {
    final sw = web.window.navigator.serviceWorker;
    final regs = await sw.getRegistrations().toDart;
    for (final reg in regs.toDart) {
      final sub = await reg.pushManager.getSubscription().toDart;
      if (sub != null) return sub.endpoint;
    }
  } catch (_) {}
  return null;
}

// -- Helpers -----------------------------------------------------------------

bool _isSecureContext() => web.window.isSecureContext;

/// Decode a base64url string to a [JSUint8Array] as `JSAny` for
/// `applicationServerKey`.
JSAny _base64UrlToJSAny(String input) {
  // Restore standard base64 padding and encoding.
  var padded = input.replaceAll('-', '+').replaceAll('_', '/');
  while (padded.length % 4 != 0) {
    padded += '=';
  }
  // Decode via atob → binary string → Uint8Array.
  final binaryStr = web.window.atob(padded);
  final bytes = JSUint8Array.withLength(binaryStr.length);
  for (var i = 0; i < binaryStr.length; i++) {
    bytes.setProperty(i.toJS, binaryStr.codeUnitAt(i).toJS);
  }
  return bytes;
}
