import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';
import '../spaces/space_provider.dart';

/// Onboarding status: whether the current user has created at least one persona.
class OnboardingNotifier extends AsyncNotifier<bool> {
  @override
  Future<bool> build() async {
    final api = ref.watch(apiClientProvider);
    if (api == null) return true; // No connection — assume onboarded.

    try {
      final response = await api.templates.onboardingStatus();
      return response.data?.hasPersona ?? true;
    } catch (_) {
      // If the endpoint isn't available, assume onboarded.
      return true;
    }
  }

  /// Force re-check after persona creation.
  void refresh() => ref.invalidateSelf();
}

final onboardingStatusProvider =
    AsyncNotifierProvider<OnboardingNotifier, bool>(OnboardingNotifier.new);

/// Fetches available persona templates from the org catalog.
class TemplatesNotifier extends AsyncNotifier<List<TemplateResponse>> {
  @override
  Future<List<TemplateResponse>> build() async {
    final api = ref.watch(apiClientProvider);
    final selection = ref.watch(spaceSelectionProvider);
    if (api == null || selection.orgId == null) return [];

    try {
      final response = await api.templates.listTemplates(
        orgId: selection.orgId!,
      );
      return response.data?.toList() ?? [];
    } catch (_) {
      return [];
    }
  }
}

final templatesProvider =
    AsyncNotifierProvider<TemplatesNotifier, List<TemplateResponse>>(
      TemplatesNotifier.new,
    );

/// Convenience: true when user needs onboarding (no persona yet).
final needsOnboardingProvider = Provider<bool>((ref) {
  final status = ref.watch(onboardingStatusProvider);
  // While loading, don't block — assume no onboarding needed.
  return status.value == false;
});
