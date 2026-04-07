import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../connection/connection_provider.dart';

/// State for the analytics screen.
class AnalyticsState {
  const AnalyticsState({
    this.summary,
    this.windowHours = 24,
    this.isLoading = false,
    this.error,
  });

  final AnalyticsSummaryResponse? summary;
  final int windowHours;
  final bool isLoading;
  final String? error;

  AnalyticsState copyWith({
    AnalyticsSummaryResponse? summary,
    int? windowHours,
    bool? isLoading,
    String? error,
    bool clearError = false,
  }) {
    return AnalyticsState(
      summary: summary ?? this.summary,
      windowHours: windowHours ?? this.windowHours,
      isLoading: isLoading ?? this.isLoading,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

/// Notifier for the analytics screen.
class AnalyticsNotifier extends AsyncNotifier<AnalyticsState> {
  @override
  Future<AnalyticsState> build() async {
    return _fetchAnalytics(24);
  }

  ApiClient? get _api {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    return ApiClient(baseUrl: profile.baseUrl, token: profile.token);
  }

  Future<AnalyticsState> _fetchAnalytics(int windowHours) async {
    final api = _api;
    if (api == null) return AnalyticsState(windowHours: windowHours);

    try {
      final response =
          await api.analytics.getAnalytics(window: windowHours);
      return AnalyticsState(
        summary: response.data!,
        windowHours: windowHours,
      );
    } catch (e) {
      return AnalyticsState(
        windowHours: windowHours,
        error: e.toString(),
      );
    }
  }

  Future<void> refresh() async {
    final currentWindow = state.value?.windowHours ?? 24;
    state = const AsyncLoading();
    state = AsyncData(await _fetchAnalytics(currentWindow));
  }

  Future<void> setWindow(int windowHours) async {
    state = const AsyncLoading();
    state = AsyncData(await _fetchAnalytics(windowHours));
  }
}

/// Provider for [AnalyticsNotifier].
final analyticsProvider =
    AsyncNotifierProvider.autoDispose<AnalyticsNotifier, AnalyticsState>(
  AnalyticsNotifier.new,
);
