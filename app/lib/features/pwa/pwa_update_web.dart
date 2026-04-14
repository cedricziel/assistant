// Web implementation: JS-interop bindings for the PWA update detection
// functions exposed in index.html.
//
// The index.html script listens for the service worker 'controllerchange'
// event, which fires when Flutter's flutter_service_worker.js calls
// skipWaiting() + clients.claim() after a new version is deployed.
// This is more reliable than polling the 'waiting' state, which Flutter's
// SW skips entirely.

import 'dart:js_interop';

import 'package:web/web.dart' as web;

@JS('registerPwaUpdateCallback')
external void _jsRegisterPwaUpdateCallback(JSFunction callback);

/// Register [onUpdate] to be called when a new service worker version has
/// taken control of the page (i.e. a new app version is available).
///
/// If the update already fired before this is called, [onUpdate] is invoked
/// immediately.
void registerPwaUpdateCallback(void Function() onUpdate) {
  try {
    _jsRegisterPwaUpdateCallback(onUpdate.toJS);
  } catch (_) {
    // Gracefully ignore if the JS function is not available (e.g. no SW).
  }
}

/// Reload the page to apply the new service worker version.
void reloadPwa() => web.window.location.reload();
