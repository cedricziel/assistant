import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Tracks the unread notification count.
///
/// - Incremented by [NotificationService.show] on macOS.
/// - Cleared by the root widget when [AppLifecycleState.resumed] fires.
///
/// Note: `tray_manager` 0.2.x does not expose a `setBadgeLabel` API on macOS.
/// The badge count is maintained in-memory so the UI can reflect it (e.g. as a
/// overlay badge on the tray icon using a composite image), but tray icon
/// updates are left as a future enhancement when the API becomes available.
class NotificationBadgeNotifier extends Notifier<int> {
  @override
  int build() => 0;

  void increment() {
    state = state + 1;
    _logBadge();
  }

  void clear() {
    if (state == 0) return;
    state = 0;
    _logBadge();
  }

  void _logBadge() {
    if (kDebugMode) {
      debugPrint('[NotificationBadge] unread count: $state');
    }
    // TODO(badges): update tray icon overlay when tray_manager exposes API.
  }
}

final notificationBadgeProvider =
    NotifierProvider<NotificationBadgeNotifier, int>(
  NotificationBadgeNotifier.new,
);
