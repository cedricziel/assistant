import 'package:fresh_dio/fresh_dio.dart';

import '../features/auth/oauth_credentials.dart';
import '../features/contexts/data/context_repository.dart';
import '../features/contexts/models/context_model.dart';

/// Bridges fresh_dio's [TokenStorage] contract to [ContextRepository].
///
/// Scoped to a single [contextId]: every write goes through
/// `repository.saveContext(ctx.copyWith(...))` so credentials survive a
/// restart. [read] is on fresh_dio's hot path — it returns from an in-memory
/// cache seeded by [initial] and refreshed by every [write]/[delete].
class ContextTokenStorage implements TokenStorage<OAuthCredentials> {
  ContextTokenStorage({
    required this.contextId,
    required this.repository,
    OAuthCredentials? initial,
  }) : _current = initial;

  final String contextId;
  final ContextRepository repository;
  OAuthCredentials? _current;

  @override
  Future<OAuthCredentials?> read() async => _current;

  @override
  Future<void> write(OAuthCredentials token) async {
    _current = token;
    final ctx = await _loadCtx();
    if (ctx == null) return;
    await repository.saveContext(ctx.copyWith(oauthCredentials: token));
  }

  @override
  Future<void> delete() async {
    _current = null;
    final ctx = await _loadCtx();
    if (ctx == null) return;
    await repository.saveContext(ctx.copyWith(clearOAuthCredentials: true));
  }

  Future<AssistantContext?> _loadCtx() async {
    final all = await repository.loadContexts();
    for (final c in all) {
      if (c.id == contextId) return c;
    }
    return null;
  }
}
