import 'dart:async';

import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/connectivity_provider.dart';

void main() {
  group('isOnlineProvider', () {
    test('returns true when wifi is available', () async {
      final container = ProviderContainer(
        overrides: [
          connectivityProvider.overrideWith(
            (ref) => Stream.value([ConnectivityResult.wifi]),
          ),
        ],
      );
      addTearDown(container.dispose);

      // Keep a listener alive so autoDispose doesn't kick in.
      final sub = container.listen(isOnlineProvider, (_, _) {});
      await container.read(connectivityProvider.future);

      expect(container.read(isOnlineProvider), isTrue);
      sub.close();
    });

    test('returns true when mobile is available', () async {
      final container = ProviderContainer(
        overrides: [
          connectivityProvider.overrideWith(
            (ref) => Stream.value([ConnectivityResult.mobile]),
          ),
        ],
      );
      addTearDown(container.dispose);

      final sub = container.listen(isOnlineProvider, (_, _) {});
      await container.read(connectivityProvider.future);

      expect(container.read(isOnlineProvider), isTrue);
      sub.close();
    });

    test('returns false when connectivity is none', () async {
      final container = ProviderContainer(
        overrides: [
          connectivityProvider.overrideWith(
            (ref) => Stream.value([ConnectivityResult.none]),
          ),
        ],
      );
      addTearDown(container.dispose);

      final sub = container.listen(isOnlineProvider, (_, _) {});
      await container.read(connectivityProvider.future);

      expect(container.read(isOnlineProvider), isFalse);
      sub.close();
    });

    test('returns false when result list is empty', () async {
      final container = ProviderContainer(
        overrides: [
          connectivityProvider.overrideWith(
            (ref) => Stream.value(<ConnectivityResult>[]),
          ),
        ],
      );
      addTearDown(container.dispose);

      final sub = container.listen(isOnlineProvider, (_, _) {});
      await container.read(connectivityProvider.future);

      expect(container.read(isOnlineProvider), isFalse);
      sub.close();
    });

    test('returns true when loading (optimistic)', () {
      final container = ProviderContainer(
        overrides: [
          connectivityProvider.overrideWith((ref) => const Stream.empty()),
        ],
      );
      addTearDown(container.dispose);

      final sub = container.listen(isOnlineProvider, (_, _) {});

      // Stream hasn't emitted yet — loading state.
      expect(container.read(isOnlineProvider), isTrue);
      sub.close();
    });

    test('returns true with multiple results including wifi', () async {
      final container = ProviderContainer(
        overrides: [
          connectivityProvider.overrideWith(
            (ref) => Stream.value([
              ConnectivityResult.wifi,
              ConnectivityResult.ethernet,
            ]),
          ),
        ],
      );
      addTearDown(container.dispose);

      final sub = container.listen(isOnlineProvider, (_, _) {});
      await container.read(connectivityProvider.future);

      expect(container.read(isOnlineProvider), isTrue);
      sub.close();
    });

    test('transitions from online to offline', () async {
      final controller = StreamController<List<ConnectivityResult>>();
      addTearDown(controller.close);

      final container = ProviderContainer(
        overrides: [
          connectivityProvider.overrideWith((ref) => controller.stream),
        ],
      );
      addTearDown(container.dispose);

      final values = <bool>[];
      final sub = container.listen(isOnlineProvider, (_, next) {
        values.add(next);
      });

      controller.add([ConnectivityResult.wifi]);
      await Future.microtask(() {});
      expect(container.read(isOnlineProvider), isTrue);

      controller.add([ConnectivityResult.none]);
      await Future.microtask(() {});
      expect(container.read(isOnlineProvider), isFalse);

      sub.close();
    });
  });
}
