import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for CommandsApi
void main() {
  final instance = AssistantApi().getCommandsApi();

  group(CommandsApi, () {
    // `POST /api/conversations/{id}/command` — execute a slash command.
    //
    //Future<CommandEventResponse> executeCommand(String id, ExecuteCommandRequest executeCommandRequest) async
    test('test executeCommand', () async {
      // TODO
    });

    // `GET /api/commands` — list all registered commands.
    //
    //Future<BuiltList<CommandDefResponse>> listCommands() async
    test('test listCommands', () async {
      // TODO
    });

    // `GET /api/conversations/{id}/events` — list command events for the timeline.
    //
    //Future<BuiltList<CommandEventResponse>> listConversationEvents(String id) async
    test('test listConversationEvents', () async {
      // TODO
    });
  });
}
