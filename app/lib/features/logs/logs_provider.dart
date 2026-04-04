import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../connection/connection_provider.dart';

/// State for the logs screen.
class LogsState {
  const LogsState({
    this.logs = const [],
    this.searchQuery = '',
    this.isLoading = false,
    this.error,
  });

  final List<LogEntryResponse> logs;

  /// Current keyword filter (debounced before the API call).
  final String searchQuery;

  final bool isLoading;
  final String? error;

  LogsState copyWith({
    List<LogEntryResponse>? logs,
    String? searchQuery,
    bool? isLoading,
    String? error,
    bool clearError = false,
  }) {
    return LogsState(
      logs: logs ?? this.logs,
      searchQuery: searchQuery ?? this.searchQuery,
      isLoading: isLoading ?? this.isLoading,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

/// Notifier for the logs list with debounced keyword filter.
class LogsNotifier extends AsyncNotifier<LogsState> {
  @override
  Future<LogsState> build() async {
    return _fetchLogs('');
  }

  ApiClient? get _api {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    return ApiClient(baseUrl: profile.baseUrl, token: profile.token);
  }

  Future<LogsState> _fetchLogs(String search) async {
    final api = _api;
    if (api == null) return const LogsState();

    try {
      final response = await api.logs.listLogs(
        limit: 100,
        search: search.trim().isEmpty ? null : search.trim(),
      );
      final logs = response.data!.toList();
      return LogsState(logs: logs, searchQuery: search);
    } catch (e) {
      return LogsState(error: e.toString(), searchQuery: search);
    }
  }

  /// Update the keyword filter and reload logs.
  Future<void> setSearch(String query) async {
    final current = state.value ?? const LogsState();
    state = AsyncData(current.copyWith(isLoading: true, searchQuery: query));
    state = AsyncData(await _fetchLogs(query));
  }

  Future<void> refresh() async {
    final query = state.value?.searchQuery ?? '';
    state = const AsyncLoading();
    state = AsyncData(await _fetchLogs(query));
  }
}

/// Provider for [LogsNotifier].
final logsProvider =
    AsyncNotifierProvider.autoDispose<LogsNotifier, LogsState>(
  LogsNotifier.new,
);
