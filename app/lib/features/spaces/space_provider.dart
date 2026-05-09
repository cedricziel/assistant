import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';

/// Active org + space selection state.
///
/// After OAuth2 login, the user selects an org and space. This selection is
/// stored in-memory (not persisted — re-derived from the JWT on next launch)
/// and drives conversation/persona list filtering.
class SpaceSelection {
  const SpaceSelection({
    this.orgId,
    this.orgName,
    this.spaceId,
    this.spaceName,
  });

  final String? orgId;
  final String? orgName;
  final String? spaceId;
  final String? spaceName;

  bool get isComplete => orgId != null && spaceId != null;

  SpaceSelection copyWith({
    String? orgId,
    String? orgName,
    String? spaceId,
    String? spaceName,
    bool clearSpace = false,
  }) {
    return SpaceSelection(
      orgId: orgId ?? this.orgId,
      orgName: orgName ?? this.orgName,
      spaceId: clearSpace ? null : (spaceId ?? this.spaceId),
      spaceName: clearSpace ? null : (spaceName ?? this.spaceName),
    );
  }
}

/// Notifier for the active org/space selection.
class SpaceSelectionNotifier extends Notifier<SpaceSelection> {
  @override
  SpaceSelection build() => const SpaceSelection();

  void selectOrg({required String orgId, required String orgName}) {
    state = state.copyWith(orgId: orgId, orgName: orgName, clearSpace: true);
  }

  void selectSpace({required String spaceId, required String spaceName}) {
    state = state.copyWith(spaceId: spaceId, spaceName: spaceName);
  }

  void clear() {
    state = const SpaceSelection();
  }
}

final spaceSelectionProvider =
    NotifierProvider<SpaceSelectionNotifier, SpaceSelection>(
      SpaceSelectionNotifier.new,
    );

/// Fetches the list of orgs the current user belongs to.
class OrgsNotifier extends AsyncNotifier<List<OrgSummary>> {
  @override
  Future<List<OrgSummary>> build() async {
    // Wait for the connection to settle before deciding "loading" vs "empty".
    // While `serverProfileProvider` is in `AsyncLoading`, this await keeps the
    // notifier in loading state too (so the UI shows a spinner, not the empty
    // card). Only after the profile resolves do we either return [] (no
    // active context) or hit the API.
    final connection = await ref.watch(serverProfileProvider.future);
    if (connection.profile == null) return [];

    final api = ref.read(apiClientProvider);
    if (api == null) return [];

    final response = await api.orgs.listOrgs();
    return response.data?.toList() ?? [];
  }
}

final orgsProvider = AsyncNotifierProvider<OrgsNotifier, List<OrgSummary>>(
  OrgsNotifier.new,
);

/// Fetches spaces for the currently selected org.
///
/// Automatically rebuilds when [spaceSelectionProvider] changes its orgId.
class SpacesNotifier extends AsyncNotifier<List<SpaceSummary>> {
  @override
  Future<List<SpaceSummary>> build() async {
    // Same loading-vs-empty distinction as OrgsNotifier: stay in AsyncLoading
    // while the connection is settling.
    final connection = await ref.watch(serverProfileProvider.future);
    final selection = ref.watch(spaceSelectionProvider);
    if (connection.profile == null || selection.orgId == null) return [];

    final api = ref.read(apiClientProvider);
    if (api == null) return [];

    final response = await api.spaces.listSpaces(orgId: selection.orgId!);
    return response.data?.toList() ?? [];
  }
}

final spacesProvider =
    AsyncNotifierProvider<SpacesNotifier, List<SpaceSummary>>(
      SpacesNotifier.new,
    );

/// Convenience: true when an org + space have been selected.
final hasSpaceSelectionProvider = Provider<bool>((ref) {
  return ref.watch(spaceSelectionProvider).isComplete;
});
