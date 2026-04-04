import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';


/// tests for AgentCardApi
void main() {
  final instance = AssistantApi().getAgentCardApi();

  group(AgentCardApi, () {
    // `GET /.well-known/agent.json` -- Returns the public agent card.
    //
    //Future<AgentCard> getAgentCardWellKnown() async
    test('test getAgentCardWellKnown', () async {
      // TODO
    });

    // `GET /agent/authenticatedExtendedCard` -- Returns the extended agent card (same as public for now).
    //
    //Future<AgentCard> getExtendedAgentCard() async
    test('test getExtendedAgentCard', () async {
      // TODO
    });

  });
}
