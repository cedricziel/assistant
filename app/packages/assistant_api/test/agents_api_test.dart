import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for AgentsApi
void main() {
  final instance = AssistantApi().getAgentsApi();

  group(AgentsApi, () {
    // `DELETE /api/agents/{id}` — remove a registered agent.
    //
    //Future deleteAgent(String id) async
    test('test deleteAgent', () async {
      // TODO
    });

    // `GET /api/agents/{id}` — get an agent by ID.
    //
    //Future<AgentDetail> getAgent(String id) async
    test('test getAgent', () async {
      // TODO
    });

    // `GET /api/agents` — list all registered agents.
    //
    //Future<BuiltList<AgentSummary>> listAgents() async
    test('test listAgents', () async {
      // TODO
    });

    // `POST /api/agents` — register a new agent.
    //
    //Future<AgentDetail> registerAgent(RegisterAgentRequest registerAgentRequest) async
    test('test registerAgent', () async {
      // TODO
    });

    // `POST /api/agents/{id}/set-default` — set an agent as the default.
    //
    //Future<AgentDetail> setDefaultAgent(String id) async
    test('test setDefaultAgent', () async {
      // TODO
    });

    // `PUT /api/agents/{id}` — update an agent's card.
    //
    //Future<AgentDetail> updateAgent(String id, UpdateAgentRequest updateAgentRequest) async
    test('test updateAgent', () async {
      // TODO
    });
  });
}
