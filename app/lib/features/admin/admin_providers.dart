import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';
import '../spaces/space_provider.dart';

// ---------------------------------------------------------------------------
// Users

/// Manages the list of users in the active organization.
class AdminUsersNotifier extends AsyncNotifier<List<UserSummary>> {
  @override
  Future<List<UserSummary>> build() async {
    final api = ref.watch(apiClientProvider);
    final selection = ref.watch(spaceSelectionProvider);
    if (api == null || selection.orgId == null) return [];

    final response = await api.users.listUsers(orgId: selection.orgId!);
    return response.data?.toList() ?? [];
  }

  /// Invite a new user to the organization.
  Future<UserDetail?> inviteUser({
    required String email,
    required String name,
    String? password,
  }) async {
    final api = ref.read(apiClientProvider);
    final orgId = ref.read(spaceSelectionProvider).orgId;
    if (api == null || orgId == null) return null;

    final response = await api.users.createUser(
      orgId: orgId,
      createUserRequest: CreateUserRequest(
        (b) => b
          ..email = email
          ..name = name
          ..password = password,
      ),
    );

    ref.invalidateSelf();
    return response.data;
  }

  /// Remove a user from the organization.
  Future<void> removeUser(String userId) async {
    final api = ref.read(apiClientProvider);
    final orgId = ref.read(spaceSelectionProvider).orgId;
    if (api == null || orgId == null) return;

    await api.users.deleteUser(orgId: orgId, id: userId);
    ref.invalidateSelf();
  }
}

final adminUsersProvider =
    AsyncNotifierProvider<AdminUsersNotifier, List<UserSummary>>(
      AdminUsersNotifier.new,
    );

// ---------------------------------------------------------------------------
// Members

/// Manages space membership for the active org + space.
class AdminMembersNotifier extends AsyncNotifier<List<MemberEntry>> {
  @override
  Future<List<MemberEntry>> build() async {
    final api = ref.watch(apiClientProvider);
    final selection = ref.watch(spaceSelectionProvider);
    if (api == null || selection.orgId == null || selection.spaceId == null) {
      return [];
    }

    final response = await api.members.listMembers(
      orgId: selection.orgId!,
      spaceId: selection.spaceId!,
    );
    return response.data?.toList() ?? [];
  }

  /// Add a user to the current space with the given role.
  Future<MemberEntry?> addMember({
    required String userId,
    required String role,
  }) async {
    final api = ref.read(apiClientProvider);
    final selection = ref.read(spaceSelectionProvider);
    if (api == null || selection.orgId == null || selection.spaceId == null) {
      return null;
    }

    final response = await api.members.addMember(
      orgId: selection.orgId!,
      spaceId: selection.spaceId!,
      addMemberRequest: AddMemberRequest(
        (b) => b
          ..userId = userId
          ..role = role,
      ),
    );

    ref.invalidateSelf();
    return response.data;
  }

  /// Remove a member from the current space.
  Future<void> removeMember(String userId) async {
    final api = ref.read(apiClientProvider);
    final selection = ref.read(spaceSelectionProvider);
    if (api == null || selection.orgId == null || selection.spaceId == null) {
      return;
    }

    await api.members.removeMember(
      orgId: selection.orgId!,
      spaceId: selection.spaceId!,
      userId: userId,
    );
    ref.invalidateSelf();
  }
}

final adminMembersProvider =
    AsyncNotifierProvider<AdminMembersNotifier, List<MemberEntry>>(
      AdminMembersNotifier.new,
    );

// ---------------------------------------------------------------------------
// Spaces (admin CRUD — extends the read-only spacesProvider)

/// Admin space management: create and delete spaces in the active org.
class AdminSpacesNotifier extends AsyncNotifier<List<SpaceSummary>> {
  @override
  Future<List<SpaceSummary>> build() async {
    final api = ref.watch(apiClientProvider);
    final selection = ref.watch(spaceSelectionProvider);
    if (api == null || selection.orgId == null) return [];

    final response = await api.spaces.listSpaces(orgId: selection.orgId!);
    return response.data?.toList() ?? [];
  }

  /// Create a new space in the active org.
  Future<SpaceDetail?> createSpace({
    required String name,
    required String slug,
  }) async {
    final api = ref.read(apiClientProvider);
    final orgId = ref.read(spaceSelectionProvider).orgId;
    if (api == null || orgId == null) return null;

    final response = await api.spaces.createSpace(
      orgId: orgId,
      createSpaceRequest: CreateSpaceRequest(
        (b) => b
          ..name = name
          ..slug = slug,
      ),
    );

    ref.invalidateSelf();
    // Also refresh the main spaces provider used by the space selector.
    ref.invalidate(spacesProvider);
    return response.data;
  }

  /// Delete a space from the active org.
  Future<void> deleteSpace(String spaceId) async {
    final api = ref.read(apiClientProvider);
    final orgId = ref.read(spaceSelectionProvider).orgId;
    if (api == null || orgId == null) return;

    await api.spaces.deleteSpace(orgId: orgId, id: spaceId);
    ref.invalidateSelf();
    ref.invalidate(spacesProvider);
  }
}

final adminSpacesProvider =
    AsyncNotifierProvider<AdminSpacesNotifier, List<SpaceSummary>>(
      AdminSpacesNotifier.new,
    );

// ---------------------------------------------------------------------------
// Catalog

/// Manages the org-level catalog (skills, templates, interfaces).
class AdminCatalogNotifier extends AsyncNotifier<List<CatalogItemResponse>> {
  @override
  Future<List<CatalogItemResponse>> build() async {
    final api = ref.watch(apiClientProvider);
    final selection = ref.watch(spaceSelectionProvider);
    if (api == null || selection.orgId == null) return [];

    final response = await api.catalog.listCatalog(orgId: selection.orgId!);
    return response.data?.toList() ?? [];
  }

  /// Publish a new item to the catalog.
  Future<CatalogItemResponse?> publishItem({
    required String name,
    required String resourceType,
    String? description,
  }) async {
    final api = ref.read(apiClientProvider);
    final orgId = ref.read(spaceSelectionProvider).orgId;
    if (api == null || orgId == null) return null;

    final response = await api.catalog.publishCatalogItem(
      orgId: orgId,
      publishCatalogItemRequest: PublishCatalogItemRequest(
        (b) => b
          ..name = name
          ..resourceType = resourceType
          ..description = description,
      ),
    );

    ref.invalidateSelf();
    return response.data;
  }

  /// Remove an item from the catalog.
  Future<void> removeItem(String itemId) async {
    final api = ref.read(apiClientProvider);
    final orgId = ref.read(spaceSelectionProvider).orgId;
    if (api == null || orgId == null) return;

    await api.catalog.deleteCatalogItem(orgId: orgId, itemId: itemId);
    ref.invalidateSelf();
  }
}

final adminCatalogProvider =
    AsyncNotifierProvider<AdminCatalogNotifier, List<CatalogItemResponse>>(
      AdminCatalogNotifier.new,
    );
