import 'dart:async';

import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:dio/dio.dart' show DioException;
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models/stream_event.dart';
import '../connection/connection_provider.dart';
import '../spaces/space_provider.dart';

// -- Conversation list -------------------------------------------------------

/// State for the conversation list.
class ConversationListState {
  const ConversationListState({
    this.conversations = const [],
    this.isLoading = false,
    this.error,
  });

  final List<ConversationListEntry> conversations;
  final bool isLoading;
  final String? error;

  ConversationListState copyWith({
    List<ConversationListEntry>? conversations,
    bool? isLoading,
    String? error,
    bool clearError = false,
  }) {
    return ConversationListState(
      conversations: conversations ?? this.conversations,
      isLoading: isLoading ?? this.isLoading,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

/// Manages the list of conversations via an SSE stream.
///
/// Subscribes to `GET /api/conversations/stream` which sends an initial
/// `snapshot` event followed by `upserted`/`deleted` deltas. Local state
/// is patched reactively — no manual refresh needed.
class ConversationListNotifier extends AsyncNotifier<ConversationListState> {
  static const _debounceDuration = Duration(milliseconds: 300);
  static const _maxReconnectAttempts = 5;
  static const _baseReconnectDelay = Duration(seconds: 2);
  static const _maxReconnectDelay = Duration(seconds: 30);

  StreamSubscription<ConversationListEvent>? _subscription;
  Timer? _debounceTimer;
  Timer? _reconnectTimer;
  int _reconnectAttempts = 0;
  final Map<String, ConversationListEntry> _pendingUpserts = {};

  @override
  Future<ConversationListState> build() async {
    final api = ref.watch(apiClientProvider);
    // Rebuild (reconnect stream) when the active space changes.
    ref.watch(spaceSelectionProvider);
    if (api == null) {
      _resetStream();
      return const ConversationListState();
    }

    _subscribe(api);

    ref.onDispose(() {
      _subscription?.cancel();
      _subscription = null;
      _debounceTimer?.cancel();
      _debounceTimer = null;
      _reconnectTimer?.cancel();
      _reconnectTimer = null;
    });

    // Return loading state — the snapshot event will replace it.
    return const ConversationListState(isLoading: true);
  }

  void _resetStream() {
    _subscription?.cancel();
    _subscription = null;
    _debounceTimer?.cancel();
    _debounceTimer = null;
    _reconnectTimer?.cancel();
    _reconnectTimer = null;
    _pendingUpserts.clear();
  }

  void _subscribe(ApiClient api) {
    _resetStream();
    _subscription = api.streamConversations().listen(
      _onEvent,
      onError: (Object e) => _onStreamError(e, api),
      cancelOnError: false,
    );
  }

  void _onStreamError(Object e, ApiClient api) {
    if (e is ApiAuthException) {
      _reconnectTimer?.cancel();
      _reconnectTimer = null;
      state = AsyncData(ConversationListState(error: e.message));
      return;
    }

    // Preserve existing conversations during reconnect attempts.
    final current = state.value ?? const ConversationListState();

    if (_reconnectAttempts >= _maxReconnectAttempts) {
      // Exhausted — show error, keep conversations visible.
      state = AsyncData(current.copyWith(error: 'Connection error: $e'));
      return;
    }

    _scheduleReconnect(api);
  }

  void _scheduleReconnect(ApiClient api) {
    _reconnectTimer?.cancel();

    final delayMs =
        (_baseReconnectDelay.inMilliseconds * (1 << _reconnectAttempts)).clamp(
          0,
          _maxReconnectDelay.inMilliseconds,
        );
    _reconnectAttempts++;

    _reconnectTimer = Timer(Duration(milliseconds: delayMs), () {
      if (!ref.mounted) return;
      _subscribe(api);
    });
  }

  void _onEvent(ConversationListEvent event) {
    switch (event) {
      case ConversationSnapshotEvent(:final conversations):
        _reconnectAttempts = 0;
        _flushPendingUpserts();
        state = AsyncData(ConversationListState(conversations: conversations));
      case ConversationUpsertedEvent(:final conversation):
        _pendingUpserts[conversation.id] = conversation;
        _debounceTimer?.cancel();
        _debounceTimer = Timer(_debounceDuration, _flushPendingUpserts);
      case ConversationDeletedEvent(:final conversationId):
        _pendingUpserts.remove(conversationId);
        _remove(conversationId);
    }
  }

  void _flushPendingUpserts() {
    if (_pendingUpserts.isEmpty) return;
    _debounceTimer?.cancel();
    _debounceTimer = null;

    final current = state.value ?? const ConversationListState();
    final ids = _pendingUpserts.keys.toSet();
    final updated =
        current.conversations.where((c) => !ids.contains(c.id)).toList()
          ..addAll(_pendingUpserts.values)
          ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    _pendingUpserts.clear();
    state = AsyncData(current.copyWith(conversations: updated));
  }

  void _upsert(ConversationListEntry conv) {
    final current = state.value ?? const ConversationListState();
    final updated = current.conversations.where((c) => c.id != conv.id).toList()
      ..insert(0, conv)
      ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    state = AsyncData(current.copyWith(conversations: updated));
  }

  void _remove(String id) {
    final current = state.value ?? const ConversationListState();
    state = AsyncData(
      current.copyWith(
        conversations: current.conversations.where((c) => c.id != id).toList(),
      ),
    );
  }

  ApiClient? get _api => ref.read(apiClientProvider);

  /// Reconnect the stream. Useful as a retry after errors.
  Future<void> refresh() async {
    final api = _api;
    if (api == null) return;
    _reconnectAttempts = 0;
    state = const AsyncLoading();
    _subscribe(api);
  }

  /// Create a new conversation. Returns the ID for navigation.
  /// The stream will deliver the list update automatically.
  Future<String?> createConversation({String? title}) async {
    final api = _api;
    if (api == null) return null;

    try {
      final response = await api.conversations.createConversation(
        createConversationRequest: CreateConversationRequest(
          (b) => b.title = title,
        ),
      );
      return response.data?.id;
    } catch (e) {
      return null;
    }
  }

  /// Add an already-created conversation to the front of the local list.
  ///
  /// Accepts a [ConversationSummary] from the generated API and converts it.
  /// This is a compatibility shim — the stream will also deliver the update,
  /// but this provides immediate feedback before the event arrives.
  void prependConversation(ConversationSummary conv) {
    _upsert(
      ConversationListEntry(
        id: conv.id,
        title: conv.title,
        createdAt: conv.createdAt.toUtc(),
        updatedAt: conv.updatedAt.toUtc(),
      ),
    );
  }

  /// Rename a conversation (`PATCH` with only `title`).
  ///
  /// Returns `null` on success, or a user-presentable error message. The list
  /// is not updated here: the stream delivers the resulting `upserted` event.
  Future<String?> renameConversation(String id, String title) =>
      _patch(id, (b) => b.title = title);

  /// Pin or unpin a conversation (`PATCH` with only `pinned`).
  ///
  /// Returns `null` on success, or a user-presentable error message. The list
  /// is not updated here: the stream delivers the resulting `upserted` event.
  Future<String?> setPinned(String id, bool pinned) =>
      _patch(id, (b) => b.pinned = pinned);

  Future<String?> _patch(
    String id,
    void Function(UpdateConversationRequestBuilder) updates,
  ) async {
    final api = _api;
    if (api == null) return 'Not connected';
    try {
      await api.conversations.updateConversation(
        id: id,
        updateConversationRequest: UpdateConversationRequest(updates),
      );
      return null;
    } on DioException catch (e) {
      final data = e.response?.data;
      if (data is Map && data['error'] is String) {
        return data['error'] as String;
      }
      return e.message ?? 'Request failed';
    } catch (e) {
      return 'Request failed: $e';
    }
  }

  /// Delete a conversation by ID.
  /// The stream will deliver the list update automatically.
  Future<void> deleteConversation(String id) async {
    final api = _api;
    if (api == null) return;
    await api.conversations.deleteConversation(id: id);
  }
}

/// Provider for [ConversationListNotifier].
final conversationListProvider =
    AsyncNotifierProvider.autoDispose<
      ConversationListNotifier,
      ConversationListState
    >(ConversationListNotifier.new);
