import 'package:assistant_api/assistant_api.dart';
import 'package:built_value/json_object.dart';
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

  /// Creates a workflow. Returns an error string on failure, null on success.
  Future<String?> createWorkflow({
    required String name,
    String? description,
    required Map<String, dynamic> graph,
    bool active = false,
  }) async {
    final client = _apiClient(ref);
    if (client == null) return 'Not connected';
    try {
      final request = WorkflowUpsertRequestBuilder()
        ..name = name
        ..description = description
        ..active = active
        ..graph = JsonObject(graph);
      await client.workflows.createWorkflow(
        workflowUpsertRequest: request.build(),
      );
      await refresh();
      return null;
    } catch (e) {
      return e.toString();
    }
  }

  /// Updates a workflow. Returns an error string on failure, null on success.
  Future<String?> updateWorkflow({
    required String id,
    required String name,
    String? description,
    required Map<String, dynamic> graph,
    bool? active,
  }) async {
    final client = _apiClient(ref);
    if (client == null) return 'Not connected';
    try {
      final request = WorkflowUpsertRequestBuilder()
        ..name = name
        ..description = description
        ..active = active
        ..graph = JsonObject(graph);
      await client.workflows.updateWorkflow(
        id: id,
        workflowUpsertRequest: request.build(),
      );
      await refresh();
      return null;
    } catch (e) {
      return e.toString();
    }
  }

  /// Deletes a workflow. Returns an error string on failure, null on success.
  Future<String?> deleteWorkflow(String id) async {
    final client = _apiClient(ref);
    if (client == null) return 'Not connected';
    try {
      await client.workflows.deleteWorkflow(id: id);
      await refresh();
      return null;
    } catch (e) {
      return e.toString();
    }
  }

  /// Activates a workflow. Returns an error string on failure, null on success.
  Future<String?> activateWorkflow(String id) async {
    final client = _apiClient(ref);
    if (client == null) return 'Not connected';
    try {
      await client.workflows.activateWorkflow(id: id);
      await refresh();
      return null;
    } catch (e) {
      return e.toString();
    }
  }

  /// Deactivates a workflow. Returns an error string on failure, null on success.
  Future<String?> deactivateWorkflow(String id) async {
    final client = _apiClient(ref);
    if (client == null) return 'Not connected';
    try {
      await client.workflows.deactivateWorkflow(id: id);
      await refresh();
      return null;
    } catch (e) {
      return e.toString();
    }
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

// ---------------------------------------------------------------------------
// Workflow run detail

/// State for a single workflow run detail.
class WorkflowRunDetailState {
  const WorkflowRunDetailState({
    this.runDetail,
    this.error,
  });

  final WorkflowRunDetail? runDetail;
  final String? error;
}

/// Family key combining workflow id + run id.
typedef WorkflowRunKey = ({String workflowId, String runId});

/// Notifier for a single workflow run detail.
class WorkflowRunDetailNotifier
    extends AsyncNotifier<WorkflowRunDetailState> {
  WorkflowRunDetailNotifier(this._key);

  final WorkflowRunKey _key;

  @override
  Future<WorkflowRunDetailState> build() => _fetch();

  Future<WorkflowRunDetailState> _fetch() async {
    final client = _apiClient(ref);
    if (client == null) throw Exception('Not connected');
    try {
      final res = await client.workflows.getWorkflowRun(
        id: _key.workflowId,
        runId: _key.runId,
      );
      return WorkflowRunDetailState(runDetail: res.data);
    } catch (e) {
      return WorkflowRunDetailState(error: e.toString());
    }
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetch());
  }
}

/// Family provider for [WorkflowRunDetailNotifier].
final workflowRunDetailProvider = AsyncNotifierProvider.autoDispose
    .family<WorkflowRunDetailNotifier, WorkflowRunDetailState, WorkflowRunKey>(
  (arg) => WorkflowRunDetailNotifier(arg),
);
