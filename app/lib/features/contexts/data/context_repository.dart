import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models/context_model.dart';
import 'shared_credentials_channel.dart';

import '../../auth/oauth_credentials.dart';

const _kContextsKey = 'assistant_contexts';
const _kActiveContextIdKey = 'assistant_active_context_id';
const _kTokenPrefix = 'assistant_context_token_';
const _kOAuthPrefix = 'assistant_context_oauth_';

/// Well-known Keychain keys for native Siri/Shortcuts integration.
/// Swift App Intent code reads these directly from the Keychain.
const _kSiriServerUrl = 'assistant_siri_server_url';
const _kSiriAuthToken = 'assistant_siri_auth_token';

/// The Keychain service name that the native Swift `KeychainHelper` uses.
/// Must match the `service` parameter in `KeychainHelper(service:)`.
const _kNativeKeychainService = 'com.cedricziel.assistant';

/// The suffix of the shared Keychain access group (without the team prefix).
const _kSharedAccessGroupSuffix = 'com.cedricziel.assistant.shared';

/// Abstraction over platform secure key-value storage.
///
/// The real app uses [FlutterSecureStorageAdapter]; tests use [FakeSecureStorage].
abstract class SecureKeyValueStorage {
  Future<String?> read({required String key});
  Future<void> write({required String key, required String value});
  Future<void> delete({required String key});
}

/// Production adapter wrapping [FlutterSecureStorage].
class FlutterSecureStorageAdapter implements SecureKeyValueStorage {
  const FlutterSecureStorageAdapter(this._storage);
  final FlutterSecureStorage _storage;

  @override
  Future<String?> read({required String key}) => _storage.read(key: key);

  @override
  Future<void> write({required String key, required String value}) =>
      _storage.write(key: key, value: value);

  @override
  Future<void> delete({required String key}) => _storage.delete(key: key);
}

/// Manages persistence of [AssistantContext] objects.
///
/// Non-sensitive metadata (id, name, serverUrl, createdAt) is stored in
/// [SharedPreferences].  Auth tokens live in [SecureKeyValueStorage]
/// (platform keychain / secure enclave).
class ContextRepository {
  ContextRepository({
    required SharedPreferences prefs,
    required SecureKeyValueStorage secureStorage,
    String? appleTeamPrefix,
  }) : _prefs = prefs,
       _secureStorage = secureStorage,
       _appleTeamPrefix = appleTeamPrefix;

  final SharedPreferences _prefs;
  final SecureKeyValueStorage _secureStorage;

  /// Apple Team ID prefix (e.g. "ABCDE12345."), resolved at startup.
  /// `null` on web or when the native channel is unavailable.
  final String? _appleTeamPrefix;

  /// Returns `IOSOptions` that target the shared Keychain access group with
  /// the correct service name so items are readable by the native Swift
  /// `KeychainHelper`.
  ///
  /// Returns `null` if the team prefix is unavailable (web, or native call
  /// failed).
  IOSOptions? get _sharedIOSOptions {
    final prefix = _appleTeamPrefix;
    if (prefix == null || prefix.isEmpty) return null;
    return IOSOptions(
      accountName: _kNativeKeychainService,
      groupId: '$prefix$_kSharedAccessGroupSuffix',
    );
  }

  // -- Read -----------------------------------------------------------------

  /// Returns all saved contexts sorted by [AssistantContext.createdAt] ascending.
  /// Tokens are loaded from secure storage and merged back into each context.
  Future<List<AssistantContext>> loadContexts() async {
    final raw = _prefs.getString(_kContextsKey);
    if (raw == null || raw.isEmpty) return [];

    final list = jsonDecode(raw) as List<dynamic>;
    final contexts =
        list
            .map((e) => AssistantContext.fromJson(e as Map<String, dynamic>))
            .toList()
          ..sort((a, b) => a.createdAt.compareTo(b.createdAt));

    // Re-attach tokens from secure storage.
    // Failures are caught per-context so that a single secure-storage error
    // (common on web when the crypto key is unavailable after a hard reload)
    // does not prevent the contexts list from loading at all.
    final result = <AssistantContext>[];
    for (final ctx in contexts) {
      try {
        var restored = ctx;
        final token = await _secureStorage.read(key: '$_kTokenPrefix${ctx.id}');
        if (token != null) {
          restored = restored.copyWith(authToken: token);
        }
        final oauthJson = await _secureStorage.read(
          key: '$_kOAuthPrefix${ctx.id}',
        );
        if (oauthJson != null) {
          restored = restored.copyWith(
            oauthCredentials: OAuthCredentials.fromJsonString(oauthJson),
          );
        }
        result.add(restored);
      } catch (_) {
        result.add(ctx);
      }
    }
    return result;
  }

  // -- Write ----------------------------------------------------------------

  /// Saves [context] — inserts or updates by [AssistantContext.id].
  Future<void> saveContext(AssistantContext context) async {
    final existing = await _loadMetadata();
    final idx = existing.indexWhere((c) => c.id == context.id);
    if (idx >= 0) {
      existing[idx] = context;
    } else {
      existing.add(context);
    }
    await _persistMetadata(existing);

    if (context.authToken != null && context.authToken!.isNotEmpty) {
      await _secureStorage.write(
        key: '$_kTokenPrefix${context.id}',
        value: context.authToken!,
      );
    } else {
      await _secureStorage.delete(key: '$_kTokenPrefix${context.id}');
    }

    // Persist OAuth credentials.
    if (context.oauthCredentials != null) {
      await _secureStorage.write(
        key: '$_kOAuthPrefix${context.id}',
        value: context.oauthCredentials!.toJsonString(),
      );
    } else {
      await _secureStorage.delete(key: '$_kOAuthPrefix${context.id}');
    }

    // Keep Siri credentials in sync if this is the active context.
    if (context.id == getActiveContextId()) {
      await syncSiriCredentials();
    }
  }

  // -- Upsert by URL --------------------------------------------------------

  /// Saves [context] — if a context with the same [AssistantContext.serverUrl]
  /// already exists it is updated in-place (preserving its ID and createdAt);
  /// otherwise a new entry is inserted.
  ///
  /// Returns the context that was saved (the updated one if a match was found,
  /// or [context] unchanged).
  Future<AssistantContext> upsertContextByUrl(AssistantContext context) async {
    final existing = await _loadMetadata();
    final idx = existing.indexWhere((c) => c.serverUrl == context.serverUrl);
    final AssistantContext toSave;
    if (idx >= 0) {
      // Preserve ID and createdAt; update name, auth mode, and (below) tokens.
      toSave = existing[idx].copyWith(
        name: context.name,
        authToken: context.authToken,
        oauthCredentials: context.oauthCredentials,
        authMode: context.authMode,
      );
      existing[idx] = toSave;
    } else {
      toSave = context;
      existing.add(toSave);
    }
    await _persistMetadata(existing);

    if (toSave.authToken != null && toSave.authToken!.isNotEmpty) {
      await _secureStorage.write(
        key: '$_kTokenPrefix${toSave.id}',
        value: toSave.authToken!,
      );
    } else {
      await _secureStorage.delete(key: '$_kTokenPrefix${toSave.id}');
    }

    // Persist OAuth credentials.
    if (toSave.oauthCredentials != null) {
      await _secureStorage.write(
        key: '$_kOAuthPrefix${toSave.id}',
        value: toSave.oauthCredentials!.toJsonString(),
      );
    } else {
      await _secureStorage.delete(key: '$_kOAuthPrefix${toSave.id}');
    }

    if (toSave.id == getActiveContextId()) {
      await syncSiriCredentials();
    }
    return toSave;
  }

  // -- Delete ---------------------------------------------------------------

  /// Removes the context with [id] from both stores.
  /// If it was the active context the active ID is cleared as well.
  Future<void> deleteContext(String id) async {
    final existing = await _loadMetadata();
    existing.removeWhere((c) => c.id == id);
    await _persistMetadata(existing);
    await _secureStorage.delete(key: '$_kTokenPrefix$id');
    await _secureStorage.delete(key: '$_kOAuthPrefix$id');

    if (getActiveContextId() == id) {
      await setActiveContextId(null);
    }
  }

  // -- Active context ID ----------------------------------------------------

  /// Returns the ID of the currently active context, or `null` if none.
  String? getActiveContextId() => _prefs.getString(_kActiveContextIdKey);

  /// Persists [id] as the active context.  Pass `null` to clear.
  ///
  /// Also syncs Siri credentials so the native App Intent can reach the server.
  Future<void> setActiveContextId(String? id) async {
    if (id == null) {
      await _prefs.remove(_kActiveContextIdKey);
    } else {
      await _prefs.setString(_kActiveContextIdKey, id);
    }
    await syncSiriCredentials();
  }

  // -- Siri credential sync -------------------------------------------------

  /// Writes the active context's server URL and auth token to well-known
  /// Keychain keys so native Swift code (App Intents, share extension) can
  /// access them without the Flutter engine running.
  ///
  /// On iOS, credentials are written directly to the shared Keychain access
  /// group via `flutter_secure_storage` with `IOSOptions(groupId:)`.
  ///
  /// On macOS, the `flutter_secure_storage` plugin ignores `groupId`, so the
  /// write is performed via a native method channel instead.
  ///
  /// The default-scope write (Siri/Shortcuts backward compat) is always done.
  ///
  /// Call this whenever the active context changes or credentials are updated.
  Future<void> syncSiriCredentials() async {
    final activeId = getActiveContextId();
    if (activeId == null) {
      await _clearSiriCredentials();
      return;
    }

    final contexts = await loadContexts();
    final active = contexts.where((c) => c.id == activeId).firstOrNull;
    if (active == null) {
      await _clearSiriCredentials();
      return;
    }

    // Normalize empty token to null so both scopes stay consistent.
    final authToken = (active.authToken != null && active.authToken!.isNotEmpty)
        ? active.authToken
        : null;

    // Write to default scope (Siri/Shortcuts backward compat).
    await _secureStorage.write(key: _kSiriServerUrl, value: active.serverUrl);
    if (authToken != null) {
      await _secureStorage.write(key: _kSiriAuthToken, value: authToken);
    } else {
      await _secureStorage.delete(key: _kSiriAuthToken);
    }

    // Write to shared Keychain group (share extension + app extensions).
    await _syncSharedKeychain(
      serverUrl: active.serverUrl,
      authToken: authToken,
    );
  }

  Future<void> _clearSiriCredentials() async {
    await _secureStorage.delete(key: _kSiriServerUrl);
    await _secureStorage.delete(key: _kSiriAuthToken);
    await _syncSharedKeychain(serverUrl: null, authToken: null);
  }

  /// Writes credentials to the shared Keychain access group.
  ///
  /// On iOS: uses `flutter_secure_storage` with `IOSOptions(groupId:)`.
  /// On macOS: uses native method channel (plugin ignores `groupId`).
  Future<void> _syncSharedKeychain({
    required String? serverUrl,
    required String? authToken,
  }) async {
    if (kIsWeb) return;

    final iosOptions = _sharedIOSOptions;

    final platform = defaultTargetPlatform;

    // iOS: write directly via flutter_secure_storage.
    if (platform == TargetPlatform.iOS) {
      if (iosOptions == null) {
        debugPrint(
          'Skipping shared Keychain sync on iOS: Apple team prefix '
          'unavailable (getTeamPrefix returned null at startup).',
        );
        return;
      }
      final storage = const FlutterSecureStorage();
      try {
        if (serverUrl != null && serverUrl.isNotEmpty) {
          await storage.write(
            key: _kSiriServerUrl,
            value: serverUrl,
            iOptions: iosOptions,
          );
        } else {
          await storage.delete(key: _kSiriServerUrl, iOptions: iosOptions);
        }

        if (authToken != null && authToken.isNotEmpty) {
          await storage.write(
            key: _kSiriAuthToken,
            value: authToken,
            iOptions: iosOptions,
          );
        } else {
          await storage.delete(key: _kSiriAuthToken, iOptions: iosOptions);
        }
      } catch (e) {
        debugPrint('Failed to sync credentials to shared Keychain: $e');
      }
      return;
    }

    // macOS: use native method channel (groupId has no effect in the plugin).
    if (platform == TargetPlatform.macOS) {
      await SharedCredentialsChannel.syncCredentialsMacOS(
        serverUrl: serverUrl,
        authToken: authToken,
      );
    }
  }

  // -- Internal helpers -----------------------------------------------------

  /// Loads metadata (no tokens) from [SharedPreferences].
  Future<List<AssistantContext>> _loadMetadata() async {
    final raw = _prefs.getString(_kContextsKey);
    if (raw == null || raw.isEmpty) return [];
    final list = jsonDecode(raw) as List<dynamic>;
    return list
        .map((e) => AssistantContext.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<void> _persistMetadata(List<AssistantContext> contexts) async {
    await _prefs.setString(
      _kContextsKey,
      jsonEncode(contexts.map((c) => c.toJson()).toList()),
    );
  }
}
