import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/client.dart';
import '../../api/endpoints/logs.dart';
import '../../api/models/log_entry.dart';
import '../connection/connection_provider.dart';

/// State for the logs screen.
class LogsState {
  const LogsState({
    this.logs = const [],
    this.searchQuery = '',
    this.isLoading = false,
    this.error,
  });

  final List<LogEntry> logs;

  /// Current keyword filter (debounced before the API call).
  final String searchQuery;

  final bool isLoading;
  final String? error;

  LogsState copyWith({
    List<LogEntry>? logs,
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
class LogsNotifier extends AutoDisposeAsyncNotifier<LogsState> {
  @override
  Future<LogsState> build() async {
    return _fetchLogs('');
  }

  LogsEndpoint? get _endpoint {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    final client = AssistantClient(
      baseUrl: profile.baseUrl,
      token: profile.token,
    );
    return LogsEndpoint(client);
  }

  Future<LogsState> _fetchLogs(String search) async {
    final endpoint = _endpoint;
    if (endpoint == null) return const LogsState();

    try {
      final logs = await endpoint.list(
        filters: LogFilters(
          limit: 100,
          search: search.trim().isEmpty ? null : search.trim(),
        ),
      );
      return LogsState(logs: logs, searchQuery: search);
    } catch (e) {
      return LogsState(error: e.toString(), searchQuery: search);
    }
  }

  /// Update the keyword filter and reload logs.
  Future<void> setSearch(String query) async {
    final current = state.valueOrNull ?? const LogsState();
    state = AsyncData(current.copyWith(isLoading: true, searchQuery: query));
    state = AsyncData(await _fetchLogs(query));
  }

  Future<void> refresh() async {
    final query = state.valueOrNull?.searchQuery ?? '';
    state = const AsyncLoading();
    state = AsyncData(await _fetchLogs(query));
  }
}

/// Provider for [LogsNotifier].
final logsProvider =
    AsyncNotifierProvider.autoDispose<LogsNotifier, LogsState>(
  LogsNotifier.new,
);
