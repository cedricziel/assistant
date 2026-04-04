import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../connection/connection_provider.dart';

/// State for the traces screen.
class TracesState {
  const TracesState({
    this.traces = const [],
    this.isLoading = false,
    this.error,
  });

  final List<TraceSummaryResponse> traces;
  final bool isLoading;
  final String? error;

  TracesState copyWith({
    List<TraceSummaryResponse>? traces,
    bool? isLoading,
    String? error,
    bool clearError = false,
  }) {
    return TracesState(
      traces: traces ?? this.traces,
      isLoading: isLoading ?? this.isLoading,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

/// Notifier for the traces list.
class TracesNotifier extends AutoDisposeAsyncNotifier<TracesState> {
  @override
  Future<TracesState> build() async {
    return _fetchTraces();
  }

  ApiClient? get _api {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    return ApiClient(baseUrl: profile.baseUrl, token: profile.token);
  }

  Future<TracesState> _fetchTraces() async {
    final api = _api;
    if (api == null) return const TracesState();

    try {
      final response = await api.traces.listTraces(limit: 50);
      final traces = response.data!.toList();
      return TracesState(traces: traces);
    } catch (e) {
      return TracesState(error: e.toString());
    }
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetchTraces());
  }
}

/// Provider for [TracesNotifier].
final tracesProvider =
    AsyncNotifierProvider.autoDispose<TracesNotifier, TracesState>(
  TracesNotifier.new,
);

/// State for a single trace detail.
class TraceDetailState {
  const TraceDetailState({
    this.detail,
    this.isLoading = false,
    this.error,
  });

  final TraceDetailResponse? detail;
  final bool isLoading;
  final String? error;
}

/// Notifier for a single trace's span breakdown.
class TraceDetailNotifier
    extends AutoDisposeFamilyAsyncNotifier<TraceDetailState, String> {
  @override
  Future<TraceDetailState> build(String traceId) async {
    return _fetchDetail(traceId);
  }

  ApiClient? get _api {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    return ApiClient(baseUrl: profile.baseUrl, token: profile.token);
  }

  Future<TraceDetailState> _fetchDetail(String traceId) async {
    final api = _api;
    if (api == null) return const TraceDetailState();

    try {
      final response = await api.traces.getTrace(traceId: traceId);
      final detail = response.data!;
      return TraceDetailState(detail: detail);
    } catch (e) {
      return TraceDetailState(error: e.toString());
    }
  }
}

/// Family provider for [TraceDetailNotifier].
final traceDetailProvider = AsyncNotifierProvider.autoDispose
    .family<TraceDetailNotifier, TraceDetailState, String>(
  TraceDetailNotifier.new,
);
