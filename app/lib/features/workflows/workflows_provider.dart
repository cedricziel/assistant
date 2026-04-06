import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';

/// Simple data class for a workflow summary.
class WorkflowSummary {
  const WorkflowSummary({
    required this.id,
    required this.name,
    required this.description,
    required this.active,
  });

  final String id;
  final String name;
  final String description;
  final bool active;

  factory WorkflowSummary.fromJson(Map<String, dynamic> json) {
    return WorkflowSummary(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? '',
      description: json['description'] as String? ?? '',
      active: json['active'] as bool? ?? false,
    );
  }
}

/// State for the workflows list screen.
class WorkflowsState {
  const WorkflowsState({
    this.workflows = const [],
    this.isLoading = false,
    this.error,
  });

  final List<WorkflowSummary> workflows;
  final bool isLoading;
  final String? error;
}

/// Notifier for the workflows list.
class WorkflowsNotifier extends AsyncNotifier<WorkflowsState> {
  @override
  Future<WorkflowsState> build() async {
    return _fetchWorkflows();
  }

  Dio? get _dio {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    return Dio(
      BaseOptions(
        baseUrl: profile.baseUrl,
        headers: {'Authorization': 'Bearer ${profile.token}'},
      ),
    );
  }

  Future<WorkflowsState> _fetchWorkflows() async {
    final dio = _dio;
    if (dio == null) return const WorkflowsState();

    try {
      final response = await dio.get<List<dynamic>>('/api/workflows');
      final data = response.data ?? [];
      final workflows = data
          .whereType<Map<String, dynamic>>()
          .map(WorkflowSummary.fromJson)
          .toList();
      return WorkflowsState(workflows: workflows);
    } catch (e) {
      return WorkflowsState(error: e.toString());
    }
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetchWorkflows());
  }
}

/// Provider for [WorkflowsNotifier].
final workflowsProvider =
    AsyncNotifierProvider.autoDispose<WorkflowsNotifier, WorkflowsState>(
  WorkflowsNotifier.new,
);

// ---------------------------------------------------------------------------
// Workflow detail

/// Simple data class for a workflow detail.
class WorkflowDetail {
  const WorkflowDetail({
    required this.id,
    required this.name,
    required this.description,
    required this.active,
    this.webhookUrl,
    this.runs = const [],
  });

  final String id;
  final String name;
  final String description;
  final bool active;
  final String? webhookUrl;
  final List<WorkflowRun> runs;

  factory WorkflowDetail.fromJson(Map<String, dynamic> json) {
    return WorkflowDetail(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? '',
      description: json['description'] as String? ?? '',
      active: json['active'] as bool? ?? false,
      webhookUrl: json['webhook_url'] as String?,
    );
  }
}

/// Simple data class for a workflow run.
class WorkflowRun {
  const WorkflowRun({
    required this.id,
    required this.status,
    required this.startedAt,
  });

  final String id;
  final String status;
  final String startedAt;

  factory WorkflowRun.fromJson(Map<String, dynamic> json) {
    return WorkflowRun(
      id: json['id'] as String? ?? '',
      status: json['status'] as String? ?? '',
      startedAt: json['started_at'] as String? ?? '',
    );
  }
}

/// State for the workflow detail screen.
class WorkflowDetailState {
  const WorkflowDetailState({
    this.detail,
    this.runs = const [],
    this.isLoading = false,
    this.error,
  });

  final WorkflowDetail? detail;
  final List<WorkflowRun> runs;
  final bool isLoading;
  final String? error;
}

/// Notifier for a single workflow detail.
class WorkflowDetailNotifier extends AsyncNotifier<WorkflowDetailState> {
  WorkflowDetailNotifier(this._workflowId);

  final String _workflowId;

  @override
  Future<WorkflowDetailState> build() async {
    return _fetchDetail(_workflowId);
  }

  Dio? get _dio {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    return Dio(
      BaseOptions(
        baseUrl: profile.baseUrl,
        headers: {'Authorization': 'Bearer ${profile.token}'},
      ),
    );
  }

  Future<WorkflowDetailState> _fetchDetail(String workflowId) async {
    final dio = _dio;
    if (dio == null) return const WorkflowDetailState();

    try {
      final detailResponse =
          await dio.get<Map<String, dynamic>>('/api/workflows/$workflowId');
      final detail = WorkflowDetail.fromJson(detailResponse.data!);

      List<WorkflowRun> runs = [];
      try {
        final runsResponse = await dio
            .get<List<dynamic>>('/api/workflows/$workflowId/runs');
        runs = (runsResponse.data ?? [])
            .whereType<Map<String, dynamic>>()
            .map(WorkflowRun.fromJson)
            .toList();
      } catch (_) {
        // Runs endpoint may not exist — ignore.
      }

      return WorkflowDetailState(detail: detail, runs: runs);
    } catch (e) {
      return WorkflowDetailState(error: e.toString());
    }
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetchDetail(_workflowId));
  }
}

/// Family provider for [WorkflowDetailNotifier].
final workflowDetailProvider = AsyncNotifierProvider.autoDispose
    .family<WorkflowDetailNotifier, WorkflowDetailState, String>(
  (arg) => WorkflowDetailNotifier(arg),
);
