import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/shared/nav_shell.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('sidebarCollapsedProvider', () {
    test('defaults to false when key is absent', () async {
      SharedPreferences.setMockInitialValues({});
      final container = ProviderContainer();
      addTearDown(container.dispose);

      final value = await container.read(sidebarCollapsedProvider.future);

      expect(value, isFalse);
    });

    test('reads persisted true from SharedPreferences', () async {
      SharedPreferences.setMockInitialValues({
        'assistant.sidebarCollapsed': true,
      });
      final container = ProviderContainer();
      addTearDown(container.dispose);

      final value = await container.read(sidebarCollapsedProvider.future);

      expect(value, isTrue);
    });

    test('reads persisted false from SharedPreferences', () async {
      SharedPreferences.setMockInitialValues({
        'assistant.sidebarCollapsed': false,
      });
      final container = ProviderContainer();
      addTearDown(container.dispose);

      final value = await container.read(sidebarCollapsedProvider.future);

      expect(value, isFalse);
    });

    test('toggle() writes the new value through', () async {
      SharedPreferences.setMockInitialValues({});
      final container = ProviderContainer();
      addTearDown(container.dispose);

      // Wait for initial build.
      expect(await container.read(sidebarCollapsedProvider.future), isFalse);

      await container.read(sidebarCollapsedProvider.notifier).toggle();

      expect(container.read(sidebarCollapsedProvider).value, isTrue);

      final prefs = await SharedPreferences.getInstance();
      expect(prefs.getBool('assistant.sidebarCollapsed'), isTrue);
    });

    test('toggle() flips back and writes again', () async {
      SharedPreferences.setMockInitialValues({
        'assistant.sidebarCollapsed': true,
      });
      final container = ProviderContainer();
      addTearDown(container.dispose);

      expect(await container.read(sidebarCollapsedProvider.future), isTrue);
      await container.read(sidebarCollapsedProvider.notifier).toggle();
      expect(container.read(sidebarCollapsedProvider).value, isFalse);

      final prefs = await SharedPreferences.getInstance();
      expect(prefs.getBool('assistant.sidebarCollapsed'), isFalse);
    });
  });
}
