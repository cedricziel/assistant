import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models/context_model.dart';

const _kContextsKey = 'assistant_contexts';
const _kActiveContextIdKey = 'assistant_active_context_id';
const _kTokenPrefix = 'assistant_context_token_';

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
  const ContextRepository({
    required SharedPreferences prefs,
    required SecureKeyValueStorage secureStorage,
  }) : _prefs = prefs,
       _secureStorage = secureStorage;

  final SharedPreferences _prefs;
  final SecureKeyValueStorage _secureStorage;

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
    final result = <AssistantContext>[];
    for (final ctx in contexts) {
      final token = await _secureStorage.read(key: '$_kTokenPrefix${ctx.id}');
      result.add(token != null ? ctx.copyWith(authToken: token) : ctx);
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
  }

  // -- Delete ---------------------------------------------------------------

  /// Removes the context with [id] from both stores.
  /// If it was the active context the active ID is cleared as well.
  Future<void> deleteContext(String id) async {
    final existing = await _loadMetadata();
    existing.removeWhere((c) => c.id == id);
    await _persistMetadata(existing);
    await _secureStorage.delete(key: '$_kTokenPrefix$id');

    if (getActiveContextId() == id) {
      await setActiveContextId(null);
    }
  }

  // -- Active context ID ----------------------------------------------------

  /// Returns the ID of the currently active context, or `null` if none.
  String? getActiveContextId() => _prefs.getString(_kActiveContextIdKey);

  /// Persists [id] as the active context.  Pass `null` to clear.
  Future<void> setActiveContextId(String? id) async {
    if (id == null) {
      await _prefs.remove(_kActiveContextIdKey);
    } else {
      await _prefs.setString(_kActiveContextIdKey, id);
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
