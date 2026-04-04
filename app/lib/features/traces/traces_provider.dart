import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/client.dart';
import '../../api/endpoints/traces.dart';
import '../../api/models/trace.dart';
import '../connection/connection_provider.dart';

/// State for the traces screen.
class TracesState {
  const TracesState({
    this.traces = const [],
    this.isLoading = false,
    this.error,
  });

  final List<TraceSummary> traces;
  final bool isLoading;
  final String? error;

  TracesState copyWith({
    List<TraceSummary>? traces,
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

  TracesEndpoint? get _endpoint {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    final client = AssistantClient(
      baseUrl: profile.baseUrl,
      token: profile.token,
    );
    return TracesEndpoint(client);
  }

  Future<TracesState> _fetchTraces() async {
    final endpoint = _endpoint;
    if (endpoint == null) return const TracesState();

    try {
      final traces = await endpoint.list(
        filters: const TraceFilters(limit: 50),
      );
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

  final TraceDetail? detail;
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

  TracesEndpoint? get _endpoint {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    final client = AssistantClient(
      baseUrl: profile.baseUrl,
      token: profile.token,
    );
    return TracesEndpoint(client);
  }

  Future<TraceDetailState> _fetchDetail(String traceId) async {
    final endpoint = _endpoint;
    if (endpoint == null) return const TraceDetailState();

    try {
      final detail = await endpoint.get(traceId);
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
