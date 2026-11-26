import 'dart:async';

import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_ui/models.dart';
import 'package:dio/dio.dart' show CancelToken, DioException;
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/models/stream_event.dart';
import '../connection/connection_provider.dart';

// -- Conversation search ------------------------------------------------------

/// What the sidebar search shows.
sealed class ConversationSearchState {
  const ConversationSearchState();
}

/// The query is empty: show the conversation list.
class ConversationSearchIdle extends ConversationSearchState {
  const ConversationSearchIdle();
}

/// A request for the current query is in flight and nothing is shown yet.
class ConversationSearchLoading extends ConversationSearchState {
  const ConversationSearchLoading();
}

/// The latest results. An empty list means nothing matched.
class ConversationSearchData extends ConversationSearchState {
  const ConversationSearchData(this.results);

  final List<ConversationSearchItem> results;
}

/// The request failed.
class ConversationSearchError extends ConversationSearchState {
  const ConversationSearchError(this.message);

  final String message;
}

/// The text typed into the sidebar search field.
class ConversationSearchQuery extends Notifier<String> {
  @override
  String build() => '';

  void set(String query) => state = query;

  void clear() => state = '';
}

final conversationSearchQueryProvider =
    NotifierProvider<ConversationSearchQuery, String>(
      ConversationSearchQuery.new,
    );

/// Runs the sidebar search.
///
/// Input is debounced by 250 ms. A newer request cancels the one in flight and
/// its result is dropped. While a query is active the notifier follows the
/// conversation stream: a `deleted` event removes that result at once, and an
/// `upserted` event for a visible result re-runs the query (debounced), keeping
/// the old results on screen meanwhile.
class ConversationSearchNotifier extends Notifier<ConversationSearchState> {
  static const _debounce = Duration(milliseconds: 250);

  Timer? _timer;
  CancelToken? _cancelToken;
  StreamSubscription<ConversationListEvent>? _events;
  int _generation = 0;
  String _query = '';

  @override
  ConversationSearchState build() {
    ref.onDispose(() {
      _timer?.cancel();
      _cancelToken?.cancel();
      _events?.cancel();
    });
    ref.listen(conversationSearchQueryProvider, (_, next) => _onQuery(next));
    // Rebuilds (a new client) restart from the current text.
    final api = ref.watch(apiClientProvider);
    _query = '';
    final initial = ref.read(conversationSearchQueryProvider).trim();
    if (initial.isEmpty || api == null) return const ConversationSearchIdle();
    scheduleMicrotask(() => _onQuery(initial));
    return const ConversationSearchLoading();
  }

  void _onQuery(String raw) {
    final query = raw.trim();
    if (query == _query) return;
    _query = query;
    _abandonInFlight();
    if (query.isEmpty) {
      _events?.cancel();
      _events = null;
      state = const ConversationSearchIdle();
      return;
    }
    _listenToStream();
    state = const ConversationSearchLoading();
    _schedule();
  }

  /// Sends the current query again right away (the error state's Retry).
  void retry() {
    if (_query.isEmpty) return;
    _abandonInFlight();
    state = const ConversationSearchLoading();
    _run();
  }

  void _schedule() {
    _timer?.cancel();
    _timer = Timer(_debounce, _run);
  }

  void _abandonInFlight() {
    _generation++;
    _timer?.cancel();
    _timer = null;
    _cancelToken?.cancel();
    _cancelToken = null;
  }

  Future<void> _run() async {
    final api = ref.read(apiClientProvider);
    if (api == null) return;
    _cancelToken?.cancel();
    final token = _cancelToken = CancelToken();
    final generation = ++_generation;
    try {
      final response = await api.conversations.searchConversations(
        q: _query,
        cancelToken: token,
      );
      if (generation != _generation) return;
      final summaries = response.data ?? const <ConversationSearchSummary>[];
      state = ConversationSearchData([for (final s in summaries) _toItem(s)]);
    } on DioException catch (e) {
      if (generation != _generation || CancelToken.isCancel(e)) return;
      state = ConversationSearchError(_describe(e));
    } catch (e) {
      if (generation != _generation) return;
      state = ConversationSearchError('Search failed: $e');
    }
  }

  void _listenToStream() {
    if (_events != null) return;
    final api = ref.read(apiClientProvider);
    if (api == null) return;
    _events = api.streamConversations().listen(
      _onEvent,
      // A dropped stream only costs live updates; the results stay valid.
      onError: (Object _) {},
      cancelOnError: false,
    );
  }

  void _onEvent(ConversationListEvent event) {
    final current = state;
    if (current is! ConversationSearchData) return;
    switch (event) {
      case ConversationDeletedEvent(:final conversationId):
        if (!current.results.any((r) => r.id == conversationId)) return;
        state = ConversationSearchData([
          for (final r in current.results)
            if (r.id != conversationId) r,
        ]);
      case ConversationUpsertedEvent(:final conversation):
        if (!current.results.any((r) => r.id == conversation.id)) return;
        _schedule();
      case ConversationSnapshotEvent():
        break;
    }
  }

  static ConversationSearchItem _toItem(ConversationSearchSummary s) {
    List<SearchSegmentItem> segments(Iterable<SearchSegment> list) => [
      for (final seg in list)
        SearchSegmentItem(text: seg.text, highlighted: seg.highlighted),
    ];
    return ConversationSearchItem(
      id: s.id,
      title: s.title,
      titleSegments: segments(s.titleSegments),
      matches: [for (final m in s.matches) segments(m.segments)],
      archivedAt: s.archivedAt,
    );
  }

  static String _describe(DioException e) {
    final data = e.response?.data;
    if (data is Map && data['error'] is String) return data['error'] as String;
    return e.message ?? 'Search failed';
  }
}

final conversationSearchProvider =
    NotifierProvider<ConversationSearchNotifier, ConversationSearchState>(
      ConversationSearchNotifier.new,
    );
