import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';

/// Combined view of the caller's account: their `UserDetail` plus the org's
/// auth mode (so the UI can render an OIDC banner / hide write controls).
class AccountState {
  const AccountState({required this.user, required this.isOidc, this.issuer});

  final UserDetail user;
  final bool isOidc;
  final String? issuer;
}

/// Loads the caller's `UserDetail` and resolves the org's `auth_mode`.
class AccountNotifier extends AsyncNotifier<AccountState> {
  @override
  Future<AccountState> build() async {
    final api = ref.watch(apiClientProvider);
    if (api == null) {
      throw StateError('No active connection');
    }
    final user = (await api.account.getCurrentUser()).data!;
    final org = (await api.orgs.getOrg(id: user.orgId)).data;
    final isOidc = org?.authMode == 'oidc';
    return AccountState(user: user, isOidc: isOidc);
  }

  /// `PATCH /api/users/me` with `{name}`.
  Future<void> updateName(String name) async {
    final api = ref.read(apiClientProvider);
    if (api == null) return;
    await api.account.updateCurrentUser(
      updateCurrentUserRequest: UpdateCurrentUserRequest((b) => b..name = name),
    );
    ref.invalidateSelf();
  }

  /// `PATCH /api/users/me` with `{email}`.
  Future<void> updateEmail(String email) async {
    final api = ref.read(apiClientProvider);
    if (api == null) return;
    await api.account.updateCurrentUser(
      updateCurrentUserRequest: UpdateCurrentUserRequest(
        (b) => b..email = email,
      ),
    );
    ref.invalidateSelf();
  }

  /// `POST /api/users/me/password`.
  Future<void> changePassword({
    required String currentPassword,
    required String newPassword,
  }) async {
    final api = ref.read(apiClientProvider);
    if (api == null) return;
    await api.account.changePassword(
      changePasswordRequest: ChangePasswordRequest(
        (b) => b
          ..currentPassword = currentPassword
          ..newPassword = newPassword,
      ),
    );
  }
}

final accountProvider = AsyncNotifierProvider<AccountNotifier, AccountState>(
  AccountNotifier.new,
);
