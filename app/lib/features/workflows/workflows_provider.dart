import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../connection/connection_provider.dart';

// ---------------------------------------------------------------------------
// API client helper

ApiClient? _apiClient(Ref ref) {
  final profile = ref.read(activeProfileProvider);
  if (profile == null) return null;
  return ApiClient(baseUrl: profile.baseUrl, token: profile.token);
}

// ---------------------------------------------------------------------------
// Workflows list

/// Notifier for the workflows list.
class WorkflowsNotifier extends AsyncNotifier<List<WorkflowSummary>> {
  @override
  Future<List<WorkflowSummary>> build() => _fetch();

  Future<List<WorkflowSummary>> _fetch() async {
    final client = _apiClient(ref);
    if (client == null) return [];
    final response = await client.workflows.listWorkflows();
    return response.data?.toList() ?? [];
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(_fetch);
  }
}

/// Provider for [WorkflowsNotifier].
final workflowsProvider =
    AsyncNotifierProvider.autoDispose<WorkflowsNotifier, List<WorkflowSummary>>(
  WorkflowsNotifier.new,
);

// ---------------------------------------------------------------------------
// Workflow detail

/// Combined detail + runs state.
class WorkflowDetailState {
  const WorkflowDetailState({
    required this.detail,
    required this.runs,
  });

  final WorkflowDetail detail;
  final List<WorkflowRunSummary> runs;
}

/// Notifier for a single workflow detail (detail + recent runs).
class WorkflowDetailNotifier
    extends AsyncNotifier<WorkflowDetailState> {
  WorkflowDetailNotifier(this._workflowId);

  final String _workflowId;

  @override
  Future<WorkflowDetailState> build() => _fetch();

  Future<WorkflowDetailState> _fetch() async {
    final client = _apiClient(ref);
    if (client == null) throw Exception('Not connected');
    final api = client.workflows;

    final detailRes = await api.getWorkflow(id: _workflowId);
    final detail = detailRes.data!;

    List<WorkflowRunSummary> runs = [];
    try {
      final runsRes = await api.listWorkflowRuns(id: _workflowId);
      runs = runsRes.data?.toList() ?? [];
    } catch (_) {
      // Runs are best-effort.
    }

    return WorkflowDetailState(detail: detail, runs: runs);
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(_fetch);
  }
}

/// Family provider for [WorkflowDetailNotifier].
final workflowDetailProvider = AsyncNotifierProvider.autoDispose
    .family<WorkflowDetailNotifier, WorkflowDetailState, String>(
  (arg) => WorkflowDetailNotifier(arg),
);
