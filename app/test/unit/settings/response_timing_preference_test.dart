import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_platform_interface.dart';

import 'package:assistant_app/features/settings/response_timing_preference.dart';

/// A preferences store whose every call fails, as on a browser with storage
/// blocked.
class _ThrowingStore extends SharedPreferencesStorePlatform {
  @override
  Future<bool> clear() => throw StateError('storage unavailable');

  @override
  Future<Map<String, Object>> getAll() =>
      throw StateError('storage unavailable');

  @override
  Future<bool> remove(String key) => throw StateError('storage unavailable');

  @override
  Future<bool> setValue(String valueType, String key, Object value) =>
      throw StateError('storage unavailable');
}

void breakStorage() {
  SharedPreferences.resetStatic();
  SharedPreferencesStorePlatform.instance = _ThrowingStore();
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  tearDown(SharedPreferences.resetStatic);

  group('showResponseTimingProvider', () {
    test('yields false when no value is stored', () async {
      SharedPreferences.setMockInitialValues({});
      final container = ProviderContainer();
      addTearDown(container.dispose);

      expect(await container.read(showResponseTimingProvider.future), isFalse);
    });

    test(
      'set(true) updates state first, persists, and a fresh container reads it',
      () async {
        SharedPreferences.setMockInitialValues({});
        final container = ProviderContainer();
        addTearDown(container.dispose);
        await container.read(showResponseTimingProvider.future);

        final write = container
            .read(showResponseTimingProvider.notifier)
            .set(true);
        expect(
          container.read(showResponseTimingProvider).value,
          isTrue,
          reason: 'state must update before the write completes',
        );
        await write;

        expect(kShowResponseTimingPrefsKey, 'assistant.showResponseTiming');
        final prefs = await SharedPreferences.getInstance();
        expect(prefs.getBool(kShowResponseTimingPrefsKey), isTrue);

        final fresh = ProviderContainer();
        addTearDown(fresh.dispose);
        expect(await fresh.read(showResponseTimingProvider.future), isTrue);
      },
    );

    test('yields false when reading preferences throws', () async {
      breakStorage();
      final container = ProviderContainer();
      addTearDown(container.dispose);

      expect(await container.read(showResponseTimingProvider.future), isFalse);
    });

    test('keeps in-memory state when writing preferences throws', () async {
      SharedPreferences.setMockInitialValues({});
      final container = ProviderContainer();
      addTearDown(container.dispose);
      await container.read(showResponseTimingProvider.future);

      breakStorage();
      await container.read(showResponseTimingProvider.notifier).set(true);

      expect(container.read(showResponseTimingProvider).value, isTrue);
    });
  });
}
