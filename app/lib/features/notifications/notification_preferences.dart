import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

const _kNotifyChatMessages = 'notifications.chat_messages';
const _kNotifySkillComplete = 'notifications.skill_complete';
const _kNotifyAgentErrors = 'notifications.agent_errors';

/// Persistent notification preferences backed by [SharedPreferences].
class NotificationPreferences {
  NotificationPreferences(this._prefs);

  final SharedPreferences _prefs;

  bool get notifyChatMessages =>
      _prefs.getBool(_kNotifyChatMessages) ?? true;

  Future<void> setNotifyChatMessages(bool value) =>
      _prefs.setBool(_kNotifyChatMessages, value);

  bool get notifySkillComplete =>
      _prefs.getBool(_kNotifySkillComplete) ?? true;

  Future<void> setNotifySkillComplete(bool value) =>
      _prefs.setBool(_kNotifySkillComplete, value);

  bool get notifyAgentErrors =>
      _prefs.getBool(_kNotifyAgentErrors) ?? true;

  Future<void> setNotifyAgentErrors(bool value) =>
      _prefs.setBool(_kNotifyAgentErrors, value);
}

// -- Provider ----------------------------------------------------------------

class NotificationPreferencesNotifier
    extends AsyncNotifier<NotificationPreferences> {
  @override
  Future<NotificationPreferences> build() async {
    final prefs = await SharedPreferences.getInstance();
    return NotificationPreferences(prefs);
  }
}

final notificationPreferencesProvider =
    AsyncNotifierProvider<NotificationPreferencesNotifier,
        NotificationPreferences>(
  NotificationPreferencesNotifier.new,
);
