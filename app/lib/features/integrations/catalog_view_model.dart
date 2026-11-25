import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';

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
    _debounce = Timer(catalogSearchDebounce, () => _refetch(query: query));
  }

  /// Filters by [category]; selecting the active one (or null) clears it.
  Future<void> selectCategory(String? category) {
    _debounce?.cancel();
    final current = state.value?.category;
    return _refetch(category: category == current ? null : category);
  }

  /// Drops the search text and the category filter.
  Future<void> clearFilters() {
    _debounce?.cancel();
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
      if (generation != _generation) return;
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
      if (generation != _generation) return;
      state = AsyncData(
        state.requireValue.copyWith(isLoadingMore: false, loadMoreFailed: true),
      );
    }
  }

  /// Re-runs the current search after a failure.
  Future<void> retry() => _refetch();

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
    state = AsyncData(await _fetchFirstPage(next));
  }

  Future<CatalogState> _fetchFirstPage(CatalogState request) async {
    final generation = ++_generation;
    try {
      final page = await _fetchPage(request);
      if (generation != _generation) return state.value ?? request;
      return request.copyWith(
        items: page.items,
        nextCursor: page.nextCursor,
        categories: _mergeCategories(request.categories, page.items),
        isLoading: false,
      );
    } catch (e) {
      if (generation != _generation) return state.value ?? request;
      return request.copyWith(
        items: const [],
        nextCursor: null,
        isLoading: false,
        error: e.toString(),
      );
    }
  }

  Future<({List<CatalogEntrySummary> items, String? nextCursor})> _fetchPage(
    CatalogState request, {
    String? cursor,
  }) async {
    final api = ref.read(apiClientProvider);
    if (api == null) throw StateError('Not connected');
    final query = request.query;
    final response = await api.integrations.listCatalogEntries(
      q: query.isEmpty ? null : query,
      category: request.category,
      limit: catalogPageSize,
      cursor: cursor,
    );
    final data = response.data;
    if (data == null) throw StateError('Empty catalog response');
    return (items: data.items.toList(), nextCursor: data.nextCursor);
  }

  List<String> _mergeCategories(
    List<String> known,
    List<CatalogEntrySummary> items,
  ) {
    final merged = {...known, for (final item in items) ...item.categories};
    return merged.toList()..sort();
  }
}

final catalogViewModelProvider =
    AsyncNotifierProvider.autoDispose<CatalogViewModel, CatalogState>(
      CatalogViewModel.new,
    );
