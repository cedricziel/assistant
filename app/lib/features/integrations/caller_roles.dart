import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../account/account_provider.dart';
import '../connection/connection_provider.dart';
import '../spaces/space_provider.dart';

/// What the signed-in user may manage, derived from their space memberships.
///
/// The API does not report the caller's role directly, so it is read from the
/// member list of each space. The server still enforces every write; this only
/// decides which controls to show.
class CallerRoles {
  const CallerRoles({required this.spaces, required this.roles});

  final List<SpaceSummary> spaces;

  /// The caller's role per space id; spaces without an entry have none.
  final Map<String, String> roles;

  bool get isOrgAdmin => roles.values.contains('org_admin');

  bool canManageSpace(String spaceId) =>
      isOrgAdmin || roles[spaceId] == 'space_admin';

  List<SpaceSummary> get manageableSpaces =>
      spaces.where((space) => canManageSpace(space.id)).toList();
}

final callerRolesProvider = FutureProvider.autoDispose<CallerRoles>((
  ref,
) async {
  final api = ref.watch(apiClientProvider);
  final orgId = ref.watch(spaceSelectionProvider).orgId;
  final (spaces, userId) = await (
    ref.watch(spacesProvider.future),
    ref.watch(currentUserIdProvider.future),
  ).wait;
  if (api == null || orgId == null) {
    return CallerRoles(spaces: spaces, roles: const {});
  }

  final roles = <String, String>{};
  await Future.wait([
    for (final space in spaces)
      () async {
        try {
          final response = await api.members.listMembers(
            orgId: orgId,
            spaceId: space.id,
          );
          for (final member in response.data ?? const <MemberEntry>[]) {
            if (member.userId == userId) roles[space.id] = member.role;
          }
        } catch (_) {
          // Members may not read a space's member list: no role is shown.
        }
      }(),
  ]);
  return CallerRoles(spaces: spaces, roles: roles);
});
