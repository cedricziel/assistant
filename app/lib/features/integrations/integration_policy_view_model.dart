import 'package:assistant_api/assistant_api.dart';
import 'package:built_collection/built_collection.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';
import 'integration_failure.dart';

class IntegrationPolicyState {
  const IntegrationPolicyState({
    required this.policy,
    this.saving = false,
    this.message,
  });

  final IntegrationPolicyResponse policy;
  final bool saving;

  /// Why the last save failed.
  final String? message;

  IntegrationPolicyState copyWith({
    IntegrationPolicyResponse? policy,
    bool? saving,
    Object? message = _keep,
  }) => IntegrationPolicyState(
    policy: policy ?? this.policy,
    saving: saving ?? this.saving,
    message: identical(message, _keep) ? this.message : message as String?,
  );
}

const Object _keep = Object();

/// Loads and saves the org's integration policy.
class IntegrationPolicyViewModel extends AsyncNotifier<IntegrationPolicyState> {
  @override
  Future<IntegrationPolicyState> build() async {
    final api = ref.watch(apiClientProvider);
    if (api == null) throw StateError('No active connection');
    final response = await api.integrations.getIntegrationPolicy();
    return IntegrationPolicyState(policy: response.data!);
  }

  /// Replaces the policy; returns whether the server accepted it.
  Future<bool> save({
    required CatalogMode catalogMode,
    required List<String> allowedCatalogIds,
    required bool allowCustomUrls,
    required bool allowUserIntegrations,
  }) async {
    final current = state.requireValue;
    state = AsyncData(current.copyWith(saving: true, message: null));
    try {
      final response = await ref
          .read(apiClientProvider)!
          .integrations
          .putIntegrationPolicy(
            updateIntegrationPolicyRequest: UpdateIntegrationPolicyRequest(
              (b) => b
                ..catalogMode = catalogMode
                ..allowedCatalogIds = ListBuilder<String>(allowedCatalogIds)
                ..allowCustomUrls = allowCustomUrls
                ..allowUserIntegrations = allowUserIntegrations,
            ),
          );
      if (!ref.mounted) return false;
      state = AsyncData(
        IntegrationPolicyState(policy: response.data ?? current.policy),
      );
      return true;
    } catch (error) {
      if (!ref.mounted) return false;
      state = AsyncData(
        current.copyWith(
          saving: false,
          message: describeIntegrationFailure(error).message,
        ),
      );
      return false;
    }
  }
}

final integrationPolicyViewModelProvider =
    AsyncNotifierProvider.autoDispose<
      IntegrationPolicyViewModel,
      IntegrationPolicyState
    >(IntegrationPolicyViewModel.new, retry: (_, _) => null);
