import 'package:assistant_app/features/notifications/notification_service.dart';
import 'package:assistant_app/features/settings/settings_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  setUp(() {
    SharedPreferences.setMockInitialValues({});
  });

  Widget buildUnderTest() {
    return ProviderScope(
      overrides: [
        // Stub out NotificationService so it never calls the real plugin.
        notificationServiceProvider.overrideWith(
          (ref) => _FakeNotificationService(ref: ref),
        ),
      ],
      child: const MaterialApp(home: SettingsScreen()),
    );
  }

  group('SettingsScreen – Notifications section', () {
    testWidgets('renders all three notification toggles', (tester) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pump(); // let async provider load

      expect(find.text('New chat messages'), findsOneWidget);
      expect(find.text('Skill completions'), findsOneWidget);
      expect(find.text('Agent errors'), findsOneWidget);
    });

    testWidgets('toggles are on by default', (tester) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pump();

      final switches = tester.widgetList<Switch>(find.byType(Switch)).toList();
      expect(switches, hasLength(3));
      for (final sw in switches) {
        expect(sw.value, isTrue, reason: 'All toggles should default to true');
      }
    });

    testWidgets('tapping "New chat messages" toggle turns it off',
        (tester) async {
      await tester.pumpWidget(buildUnderTest());
      await tester.pump();

      final chatTile = find.ancestor(
        of: find.text('New chat messages'),
        matching: find.byType(SwitchListTile),
      );
      await tester.tap(chatTile);
      await tester.pump();

      // Re-read the persisted value.
      final prefs = await SharedPreferences.getInstance();
      expect(prefs.getBool('notifications.chat_messages'), isFalse);
    });
  });
}

// ---------------------------------------------------------------------------
// Fake NotificationService that never calls the real plugin.

class _FakeNotificationService extends NotificationService {
  _FakeNotificationService({required super.ref});

  @override
  Future<void> initialize() async {}

  @override
  Future<bool> requestPermission() async => true;

  @override
  Future<void> show(
    String title,
    String body, {
    String? conversationId,
  }) async {}
}
