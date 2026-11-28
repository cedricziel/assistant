import 'package:assistant_api/assistant_api.dart'
    show RuleScope, ToolPermissionRuleSummary;
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';
import '../integrations/caller_roles.dart';
import '../spaces/space_provider.dart';

/// Optimistic revoke shared by the caller's and the space's rule lists.
mixin _RevocableRules on AsyncNotifier<List<ToolPermissionRuleSummary>> {
  /// Revokes the rule [id]. The row disappears at once; when the server call
  /// fails the row returns to its place and the error is rethrown.
  Future<void> revoke(String id) async {
    final api = ref.read(apiClientProvider);
    final current = state.value;
    if (api == null || current == null) return;
    final index = current.indexWhere((r) => r.id == id);
    if (index < 0) return;
    final removed = current[index];

    state = AsyncData([...current]..removeAt(index));
    try {
      await api.deleteToolPermissionRule(id);
    } catch (_) {
      final now = state.value ?? const <ToolPermissionRuleSummary>[];
      if (!now.any((r) => r.id == id)) {
        state = AsyncData(
          [...now]..insert(index.clamp(0, now.length), removed),
        );
      }
      rethrow;
    }
  }
}

/// The caller's saved "Always allow" / "Always reject" tool rules in the
/// selected space. Rules are stored per space and per user, so this reloads
/// when the space changes.
class ToolPermissionRulesNotifier
    extends AsyncNotifier<List<ToolPermissionRuleSummary>>
    with _RevocableRules {
  @override
  Future<List<ToolPermissionRuleSummary>> build() async {
    final api = ref.watch(apiClientProvider);
    ref.watch(spaceSelectionProvider);
    if (api == null) return const [];
    return api.listToolPermissionRules();
  }
}

final toolPermissionRulesProvider =
    AsyncNotifierProvider.autoDispose<
      ToolPermissionRulesNotifier,
      List<ToolPermissionRuleSummary>
    >(ToolPermissionRulesNotifier.new, retry: (_, _) => null);

/// Every member's saved rules in the selected space, for space and org admins.
///
/// A member or viewer never triggers the request: the provider returns an
/// empty list without calling the API, and the UI does not watch it for them.
class SpaceToolPermissionRulesNotifier
    extends AsyncNotifier<List<ToolPermissionRuleSummary>>
    with _RevocableRules {
  @override
  Future<List<ToolPermissionRuleSummary>> build() async {
    final api = ref.watch(apiClientProvider);
    final selection = ref.watch(spaceSelectionProvider);
    if (api == null || selection.spaceId == null) return const [];

    final roles = await ref.watch(callerRolesProvider.future);
    if (!roles.canManageSpace(selection.spaceId!)) return const [];

    return api.listToolPermissionRules(scope: RuleScope.space);
  }
}

final spaceToolPermissionRulesProvider =
    AsyncNotifierProvider.autoDispose<
      SpaceToolPermissionRulesNotifier,
      List<ToolPermissionRuleSummary>
    >(SpaceToolPermissionRulesNotifier.new, retry: (_, _) => null);
