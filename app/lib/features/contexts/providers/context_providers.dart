import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../data/context_repository.dart';
import '../data/shared_credentials_channel.dart';
import '../models/context_model.dart';

// -- Repository provider (override in ProviderScope for testing) ------------

/// Provider for [ContextRepository].
///
/// In production this is initialised via [initContextRepository]; in tests,
/// override this with a fake repository.
final contextRepositoryProvider = Provider<ContextRepository>((ref) {
  throw UnimplementedError(
    'contextRepositoryProvider must be initialised before use. '
    'Call initContextRepository() from main() and pass the override to ProviderScope.',
  );
});

/// Creates the production [ContextRepository].
///
/// Fetches the Apple Team ID prefix at startup so the repository can write
/// directly to the shared Keychain access group on iOS via
/// `IOSOptions(groupId:)`.
///
/// Usage in `main()`:
/// ```dart
/// final repo = await createContextRepository();
/// runApp(ProviderScope(
///   overrides: [contextRepositoryProvider.overrideWithValue(repo)],
///   child: const AssistantApp(),
/// ));
/// ```
Future<ContextRepository> createContextRepository() async {
  final prefs = await SharedPreferences.getInstance();
  final teamPrefix = await SharedCredentialsChannel.getTeamPrefix();
  return ContextRepository(
    prefs: prefs,
    secureStorage: const FlutterSecureStorageAdapter(FlutterSecureStorage()),
    appleTeamPrefix: teamPrefix,
  );
}

// -- All contexts -----------------------------------------------------------

/// Manages the full list of [AssistantContext] objects.
class ContextsNotifier extends AsyncNotifier<List<AssistantContext>> {
  @override
  Future<List<AssistantContext>> build() async {
    final repo = ref.watch(contextRepositoryProvider);
    return repo.loadContexts();
  }

  /// Inserts or updates [context] and refreshes the list.
  Future<void> saveContext(AssistantContext context) async {
    final repo = ref.read(contextRepositoryProvider);
    await repo.saveContext(context);
    ref.invalidateSelf();
    await future;
  }

  /// Removes the context with [id] and refreshes the list.
  Future<void> deleteContext(String id) async {
    final repo = ref.read(contextRepositoryProvider);
    await repo.deleteContext(id);
    ref.invalidateSelf();
    await future;
  }

  /// Inserts [context] or updates an existing context with the same
  /// [AssistantContext.serverUrl].  Returns the saved context (which may
  /// have a different ID if the URL matched an existing entry).
  Future<AssistantContext> upsertContextByUrl(AssistantContext context) async {
    final repo = ref.read(contextRepositoryProvider);
    final saved = await repo.upsertContextByUrl(context);
    ref.invalidateSelf();
    // Swallow any reload error (e.g. secure-storage failure on web) so the
    // caller always receives `saved` and can proceed to call activate().
    try {
      await future;
    } catch (_) {
      // Context was written to storage; load failure does not block activation.
    }
    return saved;
  }
}

final contextsProvider =
    AsyncNotifierProvider<ContextsNotifier, List<AssistantContext>>(
      ContextsNotifier.new,
    );

// -- Active context ---------------------------------------------------------

/// Manages which [AssistantContext] is currently active.
///
/// The active context ID is persisted in [SharedPreferences] via
/// [ContextRepository].  The active context object is derived by matching
/// the ID against the [contextsProvider] list.
class ActiveContextNotifier extends AsyncNotifier<AssistantContext?> {
  @override
  Future<AssistantContext?> build() async {
    final repo = ref.watch(contextRepositoryProvider);
    // Re-evaluate when the contexts list changes.
    final contexts = await ref.watch(contextsProvider.future);
    final activeId = repo.getActiveContextId();
    if (activeId == null) return null;
    try {
      return contexts.firstWhere((c) => c.id == activeId);
    } catch (_) {
      // Active ID no longer points to a valid context.
      return null;
    }
  }

  /// Marks [context] as the active context and persists the choice.
  Future<void> activate(AssistantContext context) async {
    final repo = ref.read(contextRepositoryProvider);
    await repo.setActiveContextId(context.id);
    state = AsyncData(context);
  }

  /// Clears the active context.
  Future<void> deactivate() async {
    final repo = ref.read(contextRepositoryProvider);
    await repo.setActiveContextId(null);
    state = const AsyncData(null);
  }
}

final activeContextProvider =
    AsyncNotifierProvider<ActiveContextNotifier, AssistantContext?>(
      ActiveContextNotifier.new,
    );

/// Synchronous convenience provider — `true` when an active context is set.
final hasActiveContextProvider = Provider<bool>((ref) {
  return ref.watch(activeContextProvider).value != null;
});
