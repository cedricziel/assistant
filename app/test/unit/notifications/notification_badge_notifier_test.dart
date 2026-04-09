import 'package:assistant_app/features/notifications/notification_badge_notifier.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('NotificationBadgeNotifier', () {
    late ProviderContainer container;

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() {
      container.dispose();
    });

    test('initial count is 0', () {
      expect(container.read(notificationBadgeProvider), equals(0));
    });

    test('increment increases count', () {
      final notifier = container.read(notificationBadgeProvider.notifier);
      notifier.increment();
      expect(container.read(notificationBadgeProvider), equals(1));

      notifier.increment();
      expect(container.read(notificationBadgeProvider), equals(2));
    });

    test('clear resets count to 0', () {
      final notifier = container.read(notificationBadgeProvider.notifier);
      notifier.increment();
      notifier.increment();
      notifier.clear();
      expect(container.read(notificationBadgeProvider), equals(0));
    });

    test('clear on already-zero count is a no-op', () {
      final notifier = container.read(notificationBadgeProvider.notifier);
      // Should not throw.
      notifier.clear();
      expect(container.read(notificationBadgeProvider), equals(0));
    });
  });
}
