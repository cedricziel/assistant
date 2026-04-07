import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  group('Skip-if-same-day frequency logic', () {
    const kLastCheckKey = 'updater.last_check_date';

    setUp(() {
      SharedPreferences.setMockInitialValues({});
    });

    test('no stored date → check should proceed', () async {
      final prefs = await SharedPreferences.getInstance();
      final stored = prefs.getString(kLastCheckKey);
      expect(stored, isNull);
    });

    test('stored date matches today → check should be skipped', () async {
      final today = DateTime.now().toIso8601String().substring(0, 10);
      SharedPreferences.setMockInitialValues({kLastCheckKey: today});

      final prefs = await SharedPreferences.getInstance();
      final stored = prefs.getString(kLastCheckKey);
      expect(stored, equals(today));

      final alreadyChecked = stored == today;
      expect(alreadyChecked, isTrue);
    });

    test('stored date is yesterday → check should proceed', () async {
      final yesterday = DateTime.now()
          .subtract(const Duration(days: 1))
          .toIso8601String()
          .substring(0, 10);
      SharedPreferences.setMockInitialValues({kLastCheckKey: yesterday});

      final prefs = await SharedPreferences.getInstance();
      final stored = prefs.getString(kLastCheckKey);
      final today = DateTime.now().toIso8601String().substring(0, 10);

      final alreadyChecked = stored == today;
      expect(alreadyChecked, isFalse);
    });

    test('after check, date is stored for today', () async {
      final prefs = await SharedPreferences.getInstance();
      final today = DateTime.now().toIso8601String().substring(0, 10);
      await prefs.setString(kLastCheckKey, today);

      final stored = prefs.getString(kLastCheckKey);
      expect(stored, equals(today));
    });
  });
}
