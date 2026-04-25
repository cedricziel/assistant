import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/onboarding/onboarding_provider.dart';

void main() {
  group('needsOnboardingProvider', () {
    test('false when onboarding status is true (has persona)', () {
      final container = ProviderContainer(
        overrides: [
          onboardingStatusProvider.overrideWith(() => _StubOnboarding(true)),
        ],
      );
      addTearDown(container.dispose);

      // Let the async notifier settle.
      container.read(onboardingStatusProvider);

      // needsOnboarding is derived from the async value.
      // When the value is true (has persona), needs onboarding is false.
      expect(container.read(needsOnboardingProvider), isFalse);
    });

    test('true when onboarding status is false (no persona)', () async {
      final container = ProviderContainer(
        overrides: [
          onboardingStatusProvider.overrideWith(() => _StubOnboarding(false)),
        ],
      );
      addTearDown(container.dispose);

      // Wait for the async provider to resolve.
      await container.read(onboardingStatusProvider.future);

      expect(container.read(needsOnboardingProvider), isTrue);
    });

    test('false while loading (optimistic — do not block)', () {
      final container = ProviderContainer(
        overrides: [
          onboardingStatusProvider.overrideWith(() => _SlowOnboarding()),
        ],
      );
      addTearDown(container.dispose);

      // Trigger the provider but don't await.
      container.read(onboardingStatusProvider);

      // While loading, the value is null, so needsOnboarding is false.
      expect(container.read(needsOnboardingProvider), isFalse);
    });
  });
}

class _StubOnboarding extends AsyncNotifier<bool>
    implements OnboardingNotifier {
  _StubOnboarding(this._hasPersona);
  final bool _hasPersona;

  @override
  Future<bool> build() async => _hasPersona;

  @override
  void refresh() => ref.invalidateSelf();
}

class _SlowOnboarding extends AsyncNotifier<bool>
    implements OnboardingNotifier {
  @override
  Future<bool> build() async {
    await Future<void>.delayed(const Duration(seconds: 60));
    return true;
  }

  @override
  void refresh() => ref.invalidateSelf();
}
