import 'package:assistant_app/features/notifications/notification_preferences.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  group('NotificationPreferences', () {
    setUp(() {
      SharedPreferences.setMockInitialValues({});
    });

    test('defaults are all true', () async {
      final prefs = await SharedPreferences.getInstance();
      final np = NotificationPreferences(prefs);

      expect(np.notifyChatMessages, isTrue);
      expect(np.notifySkillComplete, isTrue);
      expect(np.notifyAgentErrors, isTrue);
    });

    test('persist and read round-trip for notifyChatMessages', () async {
      final prefs = await SharedPreferences.getInstance();
      final np = NotificationPreferences(prefs);

      await np.setNotifyChatMessages(false);
      expect(np.notifyChatMessages, isFalse);

      await np.setNotifyChatMessages(true);
      expect(np.notifyChatMessages, isTrue);
    });

    test('persist and read round-trip for notifySkillComplete', () async {
      final prefs = await SharedPreferences.getInstance();
      final np = NotificationPreferences(prefs);

      await np.setNotifySkillComplete(false);
      expect(np.notifySkillComplete, isFalse);
    });

    test('persist and read round-trip for notifyAgentErrors', () async {
      final prefs = await SharedPreferences.getInstance();
      final np = NotificationPreferences(prefs);

      await np.setNotifyAgentErrors(false);
      expect(np.notifyAgentErrors, isFalse);
    });

    test('values survive re-creation from same SharedPreferences', () async {
      final prefs = await SharedPreferences.getInstance();

      final np1 = NotificationPreferences(prefs);
      await np1.setNotifyChatMessages(false);

      // Simulate re-reading from the same instance.
      final np2 = NotificationPreferences(prefs);
      expect(np2.notifyChatMessages, isFalse);
    });
  });
}
