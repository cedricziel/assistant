import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// SharedPreferences key for [SidebarCollapsedNotifier].
const String kSidebarCollapsedPrefsKey = 'assistant.sidebarCollapsed';

/// Whether the navigation sidebar is collapsed to icon-only mode on wide
/// screens. Persists across app launches via [SharedPreferences] under
/// [kSidebarCollapsedPrefsKey].
class SidebarCollapsedNotifier extends AsyncNotifier<bool> {
  @override
  Future<bool> build() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      return prefs.getBool(kSidebarCollapsedPrefsKey) ?? false;
    } catch (e, st) {
      debugPrint('SidebarCollapsedNotifier read failed: $e\n$st');
      return false;
    }
  }

  Future<void> toggle() async {
    final current = state.value ?? false;
    final next = !current;
    state = AsyncData(next);
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setBool(kSidebarCollapsedPrefsKey, next);
    } catch (e, st) {
      debugPrint('SidebarCollapsedNotifier write failed: $e\n$st');
    }
  }
}

final sidebarCollapsedProvider =
    AsyncNotifierProvider<SidebarCollapsedNotifier, bool>(
      SidebarCollapsedNotifier.new,
    );
