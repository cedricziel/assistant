import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// SharedPreferences key for [ShowResponseTimingNotifier].
const String kShowResponseTimingPrefsKey = 'assistant.showResponseTiming';

/// Whether finished assistant replies show their response timing. Per-device,
/// off by default, persisted under [kShowResponseTimingPrefsKey].
class ShowResponseTimingNotifier extends AsyncNotifier<bool> {
  @override
  Future<bool> build() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      return prefs.getBool(kShowResponseTimingPrefsKey) ?? false;
    } catch (e, st) {
      debugPrint('ShowResponseTimingNotifier read failed: $e\n$st');
      return false;
    }
  }

  /// Updates the in-memory state first, then persists it. A failed write keeps
  /// the in-memory value for this session.
  Future<void> set(bool value) async {
    state = AsyncData(value);
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setBool(kShowResponseTimingPrefsKey, value);
    } catch (e, st) {
      debugPrint('ShowResponseTimingNotifier write failed: $e\n$st');
    }
  }
}

final showResponseTimingProvider =
    AsyncNotifierProvider<ShowResponseTimingNotifier, bool>(
      ShowResponseTimingNotifier.new,
    );
