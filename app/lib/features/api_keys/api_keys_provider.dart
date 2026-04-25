import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';

/// Manages the list of API keys for the current user.
class ApiKeysNotifier extends AsyncNotifier<List<ApiKeySummary>> {
  @override
  Future<List<ApiKeySummary>> build() async {
    final api = ref.watch(apiClientProvider);
    if (api == null) return [];

    final response = await api.apiKeys.listApiKeys();
    return response.data?.toList() ?? [];
  }

  /// Create a new API key and return the response (includes plaintext).
  ///
  /// Keys are created with default (full) scopes. Scope selection can be
  /// added to the UI later.
  Future<CreateApiKeyResponse?> createKey({required String name}) async {
    final api = ref.read(apiClientProvider);
    if (api == null) return null;

    final response = await api.apiKeys.createApiKey(
      createApiKeyRequest: CreateApiKeyRequest((b) => b..name = name),
    );

    // Refresh the list after creation.
    ref.invalidateSelf();
    return response.data;
  }

  /// Revoke an API key by ID.
  Future<void> revokeKey(String id) async {
    final api = ref.read(apiClientProvider);
    if (api == null) return;

    await api.apiKeys.deleteApiKey(id: id);
    ref.invalidateSelf();
  }
}

final apiKeysProvider =
    AsyncNotifierProvider<ApiKeysNotifier, List<ApiKeySummary>>(
      ApiKeysNotifier.new,
    );
