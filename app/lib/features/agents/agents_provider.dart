import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../connection/connection_provider.dart';

/// State for the agents list screen.
class AgentsState {
  const AgentsState({
    this.agents = const [],
    this.isLoading = false,
    this.error,
  });

  final List<AgentSummary> agents;
  final bool isLoading;
  final String? error;
}

/// Notifier for the agents list.
class AgentsNotifier extends AsyncNotifier<AgentsState> {
  @override
  Future<AgentsState> build() async {
    final api = ref.watch(apiClientProvider);
    if (api == null) return const AgentsState();
    return _fetchAgents();
  }

  ApiClient? get _api => ref.read(apiClientProvider);

  Future<AgentsState> _fetchAgents() async {
    final api = _api;
    if (api == null) return const AgentsState();

    try {
      final response = await api.agents.listAgents();
      final agents = response.data!.toList();
      return AgentsState(agents: agents);
    } catch (e) {
      return AgentsState(error: e.toString());
    }
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetchAgents());
  }
}

/// Provider for [AgentsNotifier].
final agentsProvider =
    AsyncNotifierProvider.autoDispose<AgentsNotifier, AgentsState>(
      AgentsNotifier.new,
    );

// ---------------------------------------------------------------------------
// Agent detail

/// State for a single agent detail screen.
class AgentDetailState {
  const AgentDetailState({this.agent, this.isLoading = false, this.error});

  final AgentDetail? agent;
  final bool isLoading;
  final String? error;
}

/// Notifier for a single agent detail.
class AgentDetailNotifier extends AsyncNotifier<AgentDetailState> {
  AgentDetailNotifier(this._agentId);

  final String _agentId;

  @override
  Future<AgentDetailState> build() async {
    final api = ref.watch(apiClientProvider);
    if (api == null) return const AgentDetailState();
    return _fetchDetail(_agentId);
  }

  ApiClient? get _api => ref.read(apiClientProvider);

  Future<AgentDetailState> _fetchDetail(String agentId) async {
    final api = _api;
    if (api == null) return const AgentDetailState();

    try {
      final response = await api.agents.getAgent(id: agentId);
      return AgentDetailState(agent: response.data!);
    } catch (e) {
      return AgentDetailState(error: e.toString());
    }
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetchDetail(_agentId));
  }
}

/// Family provider for [AgentDetailNotifier].
final agentDetailProvider = AsyncNotifierProvider.autoDispose
    .family<AgentDetailNotifier, AgentDetailState, String>(
      (arg) => AgentDetailNotifier(arg),
    );
