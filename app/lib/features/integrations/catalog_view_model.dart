import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';
import 'integration_failure.dart';

/// Search-as-you-type delay before a query hits the server.
const catalogSearchDebounce = Duration(milliseconds: 300);

/// Page size requested from `GET /api/integrations/catalog`.
const catalogPageSize = 50;

/// State of the catalog browser.
class CatalogState {
  const CatalogState({
    this.query = '',
    this.category,
    this.items = const [],
    this.nextCursor,
    this.categories = const [],
    this.isLoading = false,
    this.isLoadingMore = false,
    this.loadMoreFailed = false,
    this.error,
  });

  /// Search text the current [items] were fetched with.
  final String query;

  /// Category the current [items] were fetched with, if any.
  final String? category;

  final List<CatalogEntrySummary> items;

  /// Cursor for the next page; null on the last page.
  final String? nextCursor;

  /// Every category seen so far, sorted. Accumulates across refetches so the
  /// chips stay put while a category filter narrows [items].
  final List<String> categories;

  /// A first-page fetch (initial, search, filter or retry) is in flight.
  final bool isLoading;

  final bool isLoadingMore;

  /// The last "load more" failed; [items] and [nextCursor] are unchanged.
  final bool loadMoreFailed;

  /// Why the first-page fetch failed.
  final String? error;

  CatalogState copyWith({
    String? query,
    Object? category = _keep,
    List<CatalogEntrySummary>? items,
    Object? nextCursor = _keep,
    List<String>? categories,
    bool? isLoading,
    bool? isLoadingMore,
    bool? loadMoreFailed,
    Object? error = _keep,
  }) {
    return CatalogState(
      query: query ?? this.query,
      category: identical(category, _keep)
          ? this.category
          : category as String?,
      items: items ?? this.items,
      nextCursor: identical(nextCursor, _keep)
          ? this.nextCursor
          : nextCursor as String?,
      categories: categories ?? this.categories,
      isLoading: isLoading ?? this.isLoading,
      isLoadingMore: isLoadingMore ?? this.isLoadingMore,
      loadMoreFailed: loadMoreFailed ?? this.loadMoreFailed,
      error: identical(error, _keep) ? this.error : error as String?,
    );
  }
}

const Object _keep = Object();

/// View model for the integrations catalog browser.
class CatalogViewModel extends AsyncNotifier<CatalogState> {
  Timer? _debounce;

  /// Text typed since the last fetch; set while the debounce timer is armed.
  String? _pendingQuery;

  /// Incremented per first-page fetch so a slower, superseded response is
  /// dropped instead of overwriting newer results.
  int _generation = 0;

  @override
  Future<CatalogState> build() async {
    ref.onDispose(() => _debounce?.cancel());
    final api = ref.watch(apiClientProvider);
    if (api == null) return const CatalogState();
    return _fetchFirstPage(const CatalogState());
  }

  /// Updates the search text; the fetch runs once typing pauses.
  void setQuery(String query) {
    _debounce?.cancel();
    _pendingQuery = query;
    _debounce = Timer(
      catalogSearchDebounce,
      () => _refetch(query: _takePending()),
    );
  }

  /// Filters by [category]; selecting the active one (or null) clears it.
  Future<void> selectCategory(String? category) {
    final current = state.value?.category;
    return _refetch(
      query: _takePending(),
      category: category == current ? null : category,
    );
  }

  /// Drops the search text and the category filter.
  Future<void> clearFilters() {
    _takePending();
    return _refetch(query: '', category: null);
  }

  /// Fetches the next page and appends it.
  Future<void> loadMore() async {
    final current = state.value;
    final cursor = current?.nextCursor;
    if (current == null || cursor == null || current.isLoadingMore) return;

    final generation = _generation;
    state = AsyncData(
      current.copyWith(isLoadingMore: true, loadMoreFailed: false),
    );
    try {
      final page = await _fetchPage(current, cursor: cursor);
      if (_isStale(generation)) return;
      final latest = state.requireValue;
      state = AsyncData(
        latest.copyWith(
          items: [...latest.items, ...page.items],
          nextCursor: page.nextCursor,
          categories: _mergeCategories(latest.categories, page.items),
          isLoadingMore: false,
        ),
      );
    } catch (_) {
      if (_isStale(generation)) return;
      state = AsyncData(
        state.requireValue.copyWith(isLoadingMore: false, loadMoreFailed: true),
      );
    }
  }

  /// Re-runs the current search after a failure.
  Future<void> retry() => _refetch(query: _takePending());

  Future<void> _refetch({String? query, Object? category = _keep}) async {
    final current = state.value ?? const CatalogState();
    final next = current.copyWith(
      query: query?.trim(),
      category: category,
      isLoading: true,
      isLoadingMore: false,
      loadMoreFailed: false,
      error: null,
    );
    state = AsyncData(next);
    final result = await _fetchFirstPage(next);
    if (!ref.mounted) return;
    state = AsyncData(result);
  }

  /// The notifier was disposed, or a newer first-page fetch superseded
  /// [generation].
  bool _isStale(int generation) => !ref.mounted || generation != _generation;

  /// Returns the text typed during the debounce window, if any, and disarms
  /// the timer so it cannot fire a second fetch.
  String? _takePending() {
    _debounce?.cancel();
    final pending = _pendingQuery;
    _pendingQuery = null;
    return pending;
  }

  Future<CatalogState> _fetchFirstPage(CatalogState request) async {
    final generation = ++_generation;
    try {
      final page = await _fetchPage(request);
      if (!ref.mounted) return request;
      if (generation != _generation) return state.value ?? request;
      return request.copyWith(
        items: page.items,
        nextCursor: page.nextCursor,
        categories: _mergeCategories(request.categories, page.items),
        isLoading: false,
      );
    } catch (e) {
      if (!ref.mounted) return request;
      if (generation != _generation) return state.value ?? request;
      return request.copyWith(
        items: const [],
        nextCursor: null,
        isLoading: false,
        error: _describe(e),
      );
    }
  }

  Future<({List<CatalogEntrySummary> items, String? nextCursor})> _fetchPage(
    CatalogState request, {
    String? cursor,
  }) async {
    final api = ref.read(apiClientProvider);
    if (api == null) {
      throw const _CatalogLoadException('Not connected to a server.');
    }
    final query = request.query;
    final response = await api.integrations.listCatalogEntries(
      q: query.isEmpty ? null : query,
      category: request.category,
      limit: catalogPageSize,
      cursor: cursor,
    );
    final data = response.data;
    if (data == null) {
      throw const _CatalogLoadException(
        'The server sent an unexpected response.',
      );
    }
    return (items: data.items.toList(), nextCursor: data.nextCursor);
  }

  /// A short, user-readable reason; Dio's own message is a multi-line essay
  /// and other exceptions carry internals.
  String _describe(Object error) {
    if (error is _CatalogLoadException) return error.message;
    if (error is DioException) return describeIntegrationFailure(error).message;
    return 'Something went wrong while loading the catalog.';
  }

  List<String> _mergeCategories(
    List<String> known,
    List<CatalogEntrySummary> items,
  ) {
    final merged = {...known, for (final item in items) ...item.categories};
    return merged.toList()..sort();
  }
}

/// A load failure whose [message] is safe to show.
class _CatalogLoadException implements Exception {
  const _CatalogLoadException(this.message);

  final String message;
}

final catalogViewModelProvider =
    AsyncNotifierProvider.autoDispose<CatalogViewModel, CatalogState>(
      CatalogViewModel.new,
    );
