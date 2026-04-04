import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';


/// tests for PersonasApi
void main() {
  final instance = AssistantApi().getPersonasApi();

  group(PersonasApi, () {
    // `GET /api/personas` — list all personas defined on the server.
    //
    //Future<BuiltList<PersonaSummary>> listPersonas() async
    test('test listPersonas', () async {
      // TODO
    });

    // `POST /api/personas/active` — switch the active persona for the session.
    //
    //Future<PersonaSummary> setActivePersona(SetActivePersonaRequest setActivePersonaRequest) async
    test('test setActivePersona', () async {
      // TODO
    });

  });
}
